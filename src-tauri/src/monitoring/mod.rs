mod fake;
mod libre_hardware_monitor;
mod sysinfo_adapter;
#[cfg(windows)]
mod windows_perf;

use std::time::{Duration, Instant};

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
    match std::env::var("COWORKPAL_HARDWARE_ADAPTER") {
        Ok(value) if value.eq_ignore_ascii_case("fake") => {
            Box::new(FakeHardwareSensorAdapter::new())
        }
        _ => Box::new(SysinfoAdapter::new()),
    }
}

pub fn start_hardware_snapshot_pump(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Record the tick start so the sleep at the end accounts for the
            // time already spent sampling + updating. Without this the real
            // period was (processing time + interval), so on slow machines the
            // actual sampling cadence drifted well above the configured value.
            let tick_start = Instant::now();
            let (snapshot, interval_ms) = {
                let state = app.state::<AppState>();
                let settings = state.settings.read().await.clone();
                let mut interval_ms = resolve_sampling_interval_ms(&app, &settings);

                // While a focus session is active, force a foreground-grade
                // sampling interval regardless of window visibility / low-power
                // mode. Distraction detection (record_focus_distraction_if_needed)
                // relies on tick frequency; a 60s background interval would merge
                // multiple distraction windows into one and under-count them,
                // skewing the session's focus quality score. Cap at an absolute
                // upper bound so even a user-configured large sampling_interval_ms
                // can't starve distraction detection (whose silence threshold is
                // 90s — a 10s cap gives comfortable resolution).
                {
                    let runtime = state.cat_runtime.read().await;
                    if runtime.active_focus_session_id.is_some() {
                        const FOCUS_MAX_SAMPLING_INTERVAL_MS: u64 = 10_000;
                        let focus_interval = settings
                            .sampling_interval_ms
                            .max(1000)
                            .min(FOCUS_MAX_SAMPLING_INTERVAL_MS);
                        if interval_ms > focus_interval {
                            interval_ms = focus_interval;
                        }
                    }
                }

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

            // Sleep for the remaining time toward the target interval, so the
            // effective cadence stays close to the configured value even when a
            // tick's processing (subprocess sampling + disk writes) took a while.
            let elapsed = tick_start.elapsed();
            let target = Duration::from_millis(interval_ms.max(1000));
            if let Some(remaining) = target.checked_sub(elapsed) {
                tokio::time::sleep(remaining).await;
            }
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

    // Returns (report, deltas). On save failure the entry is rolled back and
    // deltas are zeroed so the achievement system isn't credited for a tick
    // that didn't persist.
    let (updated_report, high_load_delta, thermal_warning_delta, disk_delta, network_delta, cpu_over_50_delta, memory_over_70_delta, gpu_over_70_delta, tick_seconds) = {
        let mut work_logs = state.work_logs.write().await;
        let entry = work_logs
            .entries
            .entry(date.clone())
            .or_insert_with(|| crate::models::WorkLogEntry::new(date.clone(), snapshot.timestamp));

        // Snapshot the entry before mutating so we can roll back if persistence
        // fails — otherwise the in-memory work log would diverge from disk and
        // every subsequent tick would accumulate on top of unsaved data.
        let entry_before = entry.clone();
        entry.record_snapshot(snapshot, snapshot.timestamp);
        // Compute per-tick deltas for the achievement system. These power
        // lifetime counters (high_load_seconds, disk/network bytes, thermal
        // warnings, cpu/memory/gpu over-threshold seconds) that many achievements
        // depend on. Without this event, those lifetime counters stayed at 0
        // forever — the achievements were unreachable.
        let high_load_delta =
            entry.high_load_seconds.saturating_sub(entry_before.high_load_seconds);
        let thermal_warning_delta = entry
            .cpu_over_80c_seconds
            .saturating_sub(entry_before.cpu_over_80c_seconds);
        let disk_delta = entry
            .disk_read_bytes_total
            .saturating_add(entry.disk_write_bytes_total)
            .saturating_sub(
                entry_before.disk_read_bytes_total + entry_before.disk_write_bytes_total,
            );
        let network_delta = entry
            .network_download_bytes_total
            .saturating_add(entry.network_upload_bytes_total)
            .saturating_sub(
                entry_before.network_download_bytes_total
                    + entry_before.network_upload_bytes_total,
            );
        let cpu_over_50_delta =
            entry.cpu_over_50_seconds.saturating_sub(entry_before.cpu_over_50_seconds);
        let memory_over_70_delta = entry
            .memory_over_70_seconds
            .saturating_sub(entry_before.memory_over_70_seconds);
        let gpu_over_70_delta =
            entry.gpu_over_70_seconds.saturating_sub(entry_before.gpu_over_70_seconds);
        // Tick duration in seconds — used to accrue low-power-mode enabled time
        // for the achievement system (A100 低功耗守护者). Derived from the
        // entry's updated_at delta, clamped the same way record_snapshot clamps.
        let tick_seconds = ((entry.updated_at.saturating_sub(entry_before.updated_at)) / 1000)
            .clamp(0, 60) as u64;
        let report_after = crate::models::WorkLogReport::from_entry(entry.clone());
        // The mutable borrow of `entry` (which borrows `work_logs`) ends here:
        // its last use was the clone above, so NLL releases it before the
        // immutable borrow in save_work_logs below. The rollback path re-acquires
        // the entry by key if the save fails.

        if let Err(error) = state.storage.save_work_logs(&work_logs) {
            tracing::warn!("failed to save work logs: {error}");
            // Restore the entry to its pre-tick state so memory matches disk.
            if let Some(entry) = work_logs.entries.get_mut(&date) {
                *entry = entry_before;
            }
            // Re-derive the report from the restored entry; zero the deltas so
            // no achievement credit is given for the rolled-back tick.
            let report = work_logs
                .entries
                .get(&date)
                .map(|e| crate::models::WorkLogReport::from_entry(e.clone()))
                .unwrap_or(report_after);
            (report, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64)
        } else {
            (report_after, high_load_delta, thermal_warning_delta, disk_delta, network_delta, cpu_over_50_delta, memory_over_70_delta, gpu_over_70_delta, tick_seconds)
        }
    };

    if let Err(error) = app.emit(WORKLOG_UPDATED, updated_report) {
        tracing::warn!("failed to emit {WORKLOG_UPDATED}: {error}");
    }

    // Send the hardware segment rollup to the achievement system so lifetime
    // counters (high_load_seconds, disk/network bytes, thermal warnings,
    // cpu/memory/gpu over-threshold seconds, low-power-mode seconds) accrue.
    // Previously this event was only sent from the input-activity pump with just
    // mouse/keyboard counts — the hardware fields were missing, so achievements
    // like "1小时高负载", "10GiB数据流", "CPU推进10小时", "低功耗守护者" etc.
    // were stuck at 0.
    let low_power_enabled = state
        .settings
        .read()
        .await
        .enable_low_power_mode;
    let low_power_seconds = if low_power_enabled { tick_seconds } else { 0 };

    if high_load_delta > 0
        || disk_delta > 0
        || network_delta > 0
        || thermal_warning_delta > 0
        || cpu_over_50_delta > 0
        || memory_over_70_delta > 0
        || gpu_over_70_delta > 0
        || low_power_seconds > 0
    {
        let idempotency_key = format!("hardware.segment_rollup:{}", snapshot.timestamp);
        if let Err(error) = record_internal_achievement_event(
            app,
            "hardware.segment_rollup",
            idempotency_key,
            serde_json::json!({
                "highLoadSeconds": high_load_delta,
                "thermalWarningSeconds": thermal_warning_delta,
                "diskBytesTotal": disk_delta,
                "networkBytesTotal": network_delta,
                "cpuOver50Seconds": cpu_over_50_delta,
                "memoryOver70Seconds": memory_over_70_delta,
                "gpuOver70Seconds": gpu_over_70_delta,
                "lowPowerModeEnabledSeconds": low_power_seconds,
            }),
        )
        .await
        {
            tracing::warn!("failed to record hardware segment achievement event: {error}");
        }
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
        // Snapshot the WHOLE workshop before apply_tick: the NaN guard below may
        // need to restore every numeric field (parts, insight, today_parts,
        // today_insight, ...). An earlier version captured only parts/insight
        // and rolled back just those two, leaving today_parts/today_insight as
        // NaN in memory — which a later successful tick would then persist.
        let workshop_before = workshop.clone();
        let changed = ProductionService::apply_tick(
            &mut workshop,
            &settings,
            snapshot,
            current_timestamp_ms(),
        );

        if !changed {
            return;
        }

        // Guard against NaN/Infinity polluting the workshop state. If a NaN
        // somehow entered (e.g. a non-finite temperature or load feeding
        // ProductionService::apply_tick), every arithmetic result downstream
        // becomes NaN and would be persisted + rendered, corrupting the state
        // permanently. On such a tick we restore the FULL prior workshop and
        // skip persistence so the bad frame cannot stick in any field.
        if !workshop.parts.is_finite()
            || !workshop.insight.is_finite()
            || !workshop.today_parts.is_finite()
            || !workshop.today_insight.is_finite()
        {
            tracing::warn!(
                "non-finite workshop values detected (parts={}, insight={}); discarding tick",
                workshop.parts,
                workshop.insight
            );
            *workshop = workshop_before;
            return;
        }

        let online_delta = workshop
            .total_online_seconds
            .saturating_sub(previous_online_seconds);
        let parts_delta = (workshop.parts - previous_parts).max(0.0);
        let insight_delta = (workshop.insight - previous_insight).max(0.0);

        if let Err(error) = state.storage.save_workshop(&workshop) {
            // Roll back the in-memory workshop so it does not diverge from disk.
            // Previously this only warned, leaving the tick's production added to
            // memory while unsaved — under a persistently unwritable disk the
            // in-memory workshop would grow unboundedly and be lost on restart.
            // Restoring workshop_before also zeroes the effect of this tick; we
            // null the deltas so no achievement/reward is credited for a tick
            // that didn't persist.
            tracing::warn!("failed to save workshop state: {error}");
            *workshop = workshop_before;
            (workshop.clone(), 0u64, 0.0f64, 0.0f64)
        } else {
            (workshop.clone(), online_delta, parts_delta, insight_delta)
        }
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
