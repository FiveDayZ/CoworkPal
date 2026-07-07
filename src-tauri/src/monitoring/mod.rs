mod fake;
mod sysinfo_adapter;
#[cfg(windows)]
mod windows_perf;

use std::time::Duration;

pub use fake::FakeHardwareSensorAdapter;
pub use sysinfo_adapter::SysinfoAdapter;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    commands::record_internal_achievement_event,
    app_state::AppState,
    events::{FOCUS_SESSION_UPDATED, HARDWARE_METRICS, WORKLOG_UPDATED, WORKSHOP_UPDATED},
    models::{current_timestamp_ms, FocusSessionStatus, HardwareMetricsSnapshot, HardwareSnapshot},
    pet::{PetStateService, FOCUS_DISTRACTION_SILENCE_MS},
    workshop::ProductionService,
};

pub trait HardwareSensorAdapter: Send + Sync {
    fn sample(&mut self) -> HardwareSnapshot;
}

pub fn create_default_adapter() -> Box<dyn HardwareSensorAdapter> {
    match std::env::var("COREWORKPAL_HARDWARE_ADAPTER") {
        Ok(value) if value.eq_ignore_ascii_case("fake") => {
            Box::new(FakeHardwareSensorAdapter::new())
        }
        _ => Box::new(SysinfoAdapter::new()),
    }
}

pub fn start_hardware_snapshot_pump(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let (snapshot, interval_ms) = {
                let state = app.state::<AppState>();
                let settings = state.settings.read().await.clone();
                let interval_ms = resolve_sampling_interval_ms(&app, &settings);

                // `sample()` may spawn subprocesses (nvidia-smi, powershell)
                // that block for hundreds of ms. Run it on a blocking thread
                // so the Tauri async runtime is not stalled on every tick.
                let adapter = state.hardware_adapter.clone();
                let snapshot = tokio::task::spawn_blocking(move || {
                    match adapter.lock() {
                        Ok(mut adapter) => adapter.sample(),
                        Err(error) => {
                            tracing::warn!("hardware adapter lock failed: {error}");
                            HardwareSnapshot::default()
                        }
                    }
                })
                .await
                .unwrap_or_else(|join_error| {
                    tracing::warn!("hardware sample task panicked: {join_error}");
                    HardwareSnapshot::default()
                });

                {
                    let mut last_snapshot = state.last_snapshot.write().await;
                    *last_snapshot = Some(snapshot.clone());
                }

                (snapshot, interval_ms)
            };

            if let Err(error) = app.emit(HARDWARE_METRICS, HardwareMetricsSnapshot::from(&snapshot)) {
                tracing::warn!("failed to emit {HARDWARE_METRICS}: {error}");
            }

            crate::taskbar_embed::sync_taskbar_monitor(&app).await;
            update_workshop_for_snapshot(&app, &snapshot).await;
            update_work_log_for_snapshot(&app, &snapshot).await;
            PetStateService::update_for_snapshot(&app, &snapshot).await;

            tokio::time::sleep(Duration::from_millis(interval_ms.max(1000))).await;
        }
    });
}

fn resolve_sampling_interval_ms(app: &AppHandle, settings: &crate::models::AppSettings) -> u64 {
    if settings.enable_low_power_mode {
        return 5000;
    }

    if has_visible_monitor_surface(app) {
        settings.sampling_interval_ms.max(1000)
    } else {
        settings.background_sampling_interval_ms.max(3000)
    }
}

fn has_visible_monitor_surface(app: &AppHandle) -> bool {
    ["main", "monitor-bar", "pet-panel", "taskbar-monitor"]
        .into_iter()
        .any(|label| {
            app.get_webview_window(label)
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false)
        })
}

async fn update_work_log_for_snapshot(app: &AppHandle, snapshot: &HardwareSnapshot) {
    let state = app.state::<AppState>();
    let date = crate::models::date_key_from_timestamp(snapshot.timestamp);
    let updated_report = {
        let mut work_logs = state.work_logs.write().await;
        let entry = work_logs
            .entries
            .entry(date.clone())
            .or_insert_with(|| crate::models::WorkLogEntry::new(date, snapshot.timestamp));

        entry.record_snapshot(snapshot, snapshot.timestamp);
        let report = crate::models::WorkLogReport::from_entry(entry.clone());

        if let Err(error) = state.storage.save_work_logs(&work_logs) {
            tracing::warn!("failed to save work logs: {error}");
        }

        report
    };

    if let Err(error) = app.emit(WORKLOG_UPDATED, updated_report) {
        tracing::warn!("failed to emit {WORKLOG_UPDATED}: {error}");
    }

    // Focus-session distraction detection: if a session is active and input has
    // been silent past the threshold, count one distraction (deduped per
    // continuous silent stretch via last_distraction_at).
    record_focus_distraction_if_needed(app, snapshot.timestamp).await;
}

async fn record_focus_distraction_if_needed(app: &AppHandle, now_ms: i64) {
    let state = app.state::<AppState>();

    // Step 1: under the cat_runtime lock, decide whether this tick should count
    // a distraction and capture the target session id.
    let session_id = {
        let mut runtime = state.cat_runtime.write().await;
        let Some(session_id) = runtime.active_focus_session_id.clone() else {
            return;
        };
        let silent_for = runtime
            .last_input_at
            .map(|t| now_ms.saturating_sub(t))
            .unwrap_or(i64::MAX);
        let already_counted_this_stretch = runtime
            .last_distraction_at
            .is_some_and(|t| now_ms.saturating_sub(t) < FOCUS_DISTRACTION_SILENCE_MS);
        if silent_for < FOCUS_DISTRACTION_SILENCE_MS || already_counted_this_stretch {
            return;
        }
        runtime.last_distraction_at = Some(now_ms);
        session_id
    };

    // Step 2: under the focus_sessions lock, increment the counter.
    let next_book = {
        let mut sessions = state.focus_sessions.write().await;
        if let Some(session) = sessions
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id && s.status == FocusSessionStatus::Active)
        {
            session.distraction_count = session.distraction_count.saturating_add(1);
            if let Err(error) = state.storage.save_focus_sessions(&sessions) {
                tracing::warn!("failed to save focus sessions on distraction: {error}");
            }
        }
        sessions.clone()
    };

    if let Err(error) = app.emit(FOCUS_SESSION_UPDATED, next_book) {
        tracing::warn!("failed to emit {FOCUS_SESSION_UPDATED}: {error}");
    }
}

async fn update_workshop_for_snapshot(app: &AppHandle, snapshot: &HardwareSnapshot) {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await.clone();
    let (updated_workshop, online_delta, parts_delta, insight_delta) = {
        let mut workshop = state.workshop.write().await;
        let previous_online_seconds = workshop.total_online_seconds;
        let previous_parts = workshop.parts;
        let previous_insight = workshop.insight;
        let changed = ProductionService::apply_tick(
            &mut workshop,
            &settings,
            snapshot,
            current_timestamp_ms(),
        );

        if !changed {
            return;
        }

        let online_delta = workshop
            .total_online_seconds
            .saturating_sub(previous_online_seconds);
        let parts_delta = (workshop.parts - previous_parts).max(0.0);
        let insight_delta = (workshop.insight - previous_insight).max(0.0);

        if let Err(error) = state.storage.save_workshop(&workshop) {
            tracing::warn!("failed to save workshop state: {error}");
        }

        (workshop.clone(), online_delta, parts_delta, insight_delta)
    };

    if online_delta > 0 {
        let idempotency_key = format!("app.active_minute:{}:{online_delta}", snapshot.timestamp);
        if let Err(error) = record_internal_achievement_event(
            app,
            "app.active_minute",
            idempotency_key,
            serde_json::json!({ "seconds": online_delta }),
        )
        .await
        {
            tracing::warn!("failed to record app.active_minute achievement event: {error}");
        }
    }

    if parts_delta > 0.0 || insight_delta > 0.0 {
        let idempotency_key = format!("workshop.production_tick:{}", snapshot.timestamp);
        if let Err(error) = record_internal_achievement_event(
            app,
            "workshop.production_tick",
            idempotency_key,
            serde_json::json!({
                "partsDelta": parts_delta,
                "insightDelta": insight_delta,
            }),
        )
        .await
        {
            tracing::warn!("failed to record workshop production achievement event: {error}");
        }
    }

    if let Err(error) = app.emit(WORKSHOP_UPDATED, updated_workshop) {
        tracing::warn!("failed to emit {WORKSHOP_UPDATED}: {error}");
    }
}
