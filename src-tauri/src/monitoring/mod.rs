mod fake;
mod integrated_hardware_monitor;
mod sysinfo_adapter;
#[cfg(windows)]
mod windows_perf;

#[cfg(windows)]
pub(crate) fn webview_software_rendering_recommended() -> bool {
    use std::sync::OnceLock;

    static RECOMMENDED: OnceLock<bool> = OnceLock::new();
    *RECOMMENDED.get_or_init(|| {
        if std::env::var("COWORKPAL_DISABLE_WEBVIEW_GPU")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return true;
        }
        let gpu = windows_perf::query_primary_gpu_info();
        low_memory_intel_gpu(gpu.name.as_deref(), gpu.dedicated_memory_bytes)
    })
}

#[cfg(windows)]
fn low_memory_intel_gpu(name: Option<&str>, dedicated_memory_bytes: Option<u64>) -> bool {
    const MAX_HARDWARE_RENDERING_MEMORY: u64 = 512 * 1024 * 1024;

    let Some(name) = name else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    let is_intel_integrated =
        name.contains("intel") && (name.contains("uhd graphics") || name.contains("hd graphics"));

    is_intel_integrated
        && dedicated_memory_bytes
            .map(|bytes| bytes <= MAX_HARDWARE_RENDERING_MEMORY)
            .unwrap_or(true)
}

use std::time::{Duration, Instant};

pub use fake::FakeHardwareSensorAdapter;
pub use sysinfo_adapter::SysinfoAdapter;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    app_state::AppState,
    events::{FOCUS_SESSION_UPDATED, HARDWARE_METRICS, WORKLOG_UPDATED, WORKSHOP_UPDATED},
    models::{current_timestamp_ms, FocusSessionStatus, HardwareMetricsSnapshot, HardwareSnapshot},
    persistence::PeriodicAchievementDelta,
    pet::{is_focus_distracted, PetStateService},
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
                            .clamp(1000, FOCUS_MAX_SAMPLING_INTERVAL_MS);
                        if interval_ms > focus_interval {
                            interval_ms = focus_interval;
                        }
                    }
                }

                // `sample()` may spawn subprocesses (nvidia-smi, powershell)
                // that block for hundreds of ms. Run it on a blocking thread
                // so the Tauri async runtime is not stalled on every tick.
                let adapter = state.hardware_adapter.clone();
                let snapshot = tokio::task::spawn_blocking(move || match adapter.lock() {
                    Ok(mut adapter) => adapter.sample(),
                    Err(error) => {
                        tracing::warn!("hardware adapter lock failed: {error}");
                        HardwareSnapshot::default()
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

            if let Err(error) = app.emit(HARDWARE_METRICS, HardwareMetricsSnapshot::from(&snapshot))
            {
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

#[cfg(all(test, windows))]
mod compatibility_tests {
    use super::low_memory_intel_gpu;

    #[test]
    fn low_memory_intel_uhd_uses_software_webview_rendering() {
        assert!(low_memory_intel_gpu(
            Some("Intel(R) UHD Graphics"),
            Some(128 * 1024 * 1024),
        ));
    }

    #[test]
    fn discrete_gpu_keeps_hardware_webview_rendering() {
        assert!(!low_memory_intel_gpu(
            Some("NVIDIA GeForce RTX 4060"),
            Some(8 * 1024 * 1024 * 1024),
        ));
    }
}

async fn update_work_log_for_snapshot(app: &AppHandle, snapshot: &HardwareSnapshot) {
    let state = app.state::<AppState>();
    let date = crate::models::date_key_from_timestamp(snapshot.timestamp);

    let (
        updated_report,
        high_load_delta,
        thermal_warning_delta,
        disk_delta,
        network_delta,
        cpu_over_50_delta,
        memory_over_70_delta,
        gpu_over_70_delta,
        tick_seconds,
    ) = {
        let mut work_logs = state.work_logs.write().await;
        let entry = work_logs
            .entries
            .entry(date.clone())
            .or_insert_with(|| crate::models::WorkLogEntry::new(date.clone(), snapshot.timestamp));

        let entry_before = entry.clone();
        entry.record_snapshot(snapshot, snapshot.timestamp);
        // Compute per-tick deltas for the achievement system. These power
        // lifetime counters (high_load_seconds, disk/network bytes, thermal
        // warnings, cpu/memory/gpu over-threshold seconds) that many achievements
        // depend on. Without this event, those lifetime counters stayed at 0
        // forever — the achievements were unreachable.
        let high_load_delta = entry
            .high_load_seconds
            .saturating_sub(entry_before.high_load_seconds);
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
                entry_before.network_download_bytes_total + entry_before.network_upload_bytes_total,
            );
        let cpu_over_50_delta = entry
            .cpu_over_50_seconds
            .saturating_sub(entry_before.cpu_over_50_seconds);
        let memory_over_70_delta = entry
            .memory_over_70_seconds
            .saturating_sub(entry_before.memory_over_70_seconds);
        let gpu_over_70_delta = entry
            .gpu_over_70_seconds
            .saturating_sub(entry_before.gpu_over_70_seconds);
        // Tick duration in seconds — used to accrue low-power-mode enabled time
        // for the achievement system (A100 低功耗守护者). Derived from the
        // entry's updated_at delta, clamped the same way record_snapshot clamps.
        let tick_seconds =
            ((entry.updated_at.saturating_sub(entry_before.updated_at)) / 1000).clamp(0, 60) as u64;
        let report = crate::models::WorkLogReport::from_entry(entry.clone());
        state.persistence.mark_work_logs_dirty();
        (
            report,
            high_load_delta,
            thermal_warning_delta,
            disk_delta,
            network_delta,
            cpu_over_50_delta,
            memory_over_70_delta,
            gpu_over_70_delta,
            tick_seconds,
        )
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
    let low_power_enabled = state.settings.read().await.enable_low_power_mode;
    let low_power_seconds = if low_power_enabled { tick_seconds } else { 0 };

    state
        .persistence
        .queue_periodic_achievement(PeriodicAchievementDelta {
            occurred_at: snapshot.timestamp,
            high_load_seconds: high_load_delta,
            thermal_warning_seconds: thermal_warning_delta,
            disk_bytes_total: disk_delta,
            network_bytes_total: network_delta,
            cpu_over_50_seconds: cpu_over_50_delta,
            memory_over_70_seconds: memory_over_70_delta,
            gpu_over_70_seconds: gpu_over_70_delta,
            low_power_mode_enabled_seconds: low_power_seconds,
            ..Default::default()
        })
        .await;

    // Focus-session distraction detection: if a session is active and input has
    // been silent past the threshold, count one distraction (deduped per
    // continuous silent stretch via last_distraction_at).
    record_focus_distraction_if_needed(app, snapshot).await;
}

async fn record_focus_distraction_if_needed(app: &AppHandle, snapshot: &HardwareSnapshot) {
    let state = app.state::<AppState>();
    let now_ms = snapshot.timestamp;

    // Step 1: under the cat_runtime lock, decide whether this tick should count
    // a distraction and capture the target session id.
    let session_id = {
        let mut runtime = state.cat_runtime.write().await;
        let Some(session_id) = runtime.active_focus_session_id.clone() else {
            return;
        };
        if !should_record_focus_distraction(
            snapshot,
            now_ms,
            runtime.last_input_at,
            runtime.last_distraction_at,
        ) {
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
            session.record_distraction();
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

fn should_record_focus_distraction(
    snapshot: &HardwareSnapshot,
    now_ms: i64,
    last_input_at: Option<i64>,
    last_distraction_at: Option<i64>,
) -> bool {
    last_distraction_at.is_none() && is_focus_distracted(snapshot, now_ms, last_input_at)
}

async fn update_workshop_for_snapshot(app: &AppHandle, snapshot: &HardwareSnapshot) {
    let state = app.state::<AppState>();
    let focus_multiplier = state
        .focus_sessions
        .read()
        .await
        .active_production_multiplier();
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
            focus_multiplier,
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

        state.persistence.mark_workshop_dirty();
        (workshop.clone(), online_delta, parts_delta, insight_delta)
    };

    state
        .persistence
        .queue_periodic_achievement(PeriodicAchievementDelta {
            occurred_at: snapshot.timestamp,
            online_seconds: online_delta,
            parts_delta,
            insight_delta,
            ..Default::default()
        })
        .await;

    if let Err(error) = app.emit(WORKSHOP_UPDATED, updated_workshop) {
        tracing::warn!("failed to emit {WORKSHOP_UPDATED}: {error}");
    }
}

#[cfg(test)]
mod focus_distraction_tests {
    use super::should_record_focus_distraction;
    use crate::models::HardwareSnapshot;

    #[test]
    fn one_continuous_silent_stretch_is_counted_once() {
        let snapshot = HardwareSnapshot {
            timestamp: 300_000,
            foreground_process_name: Some("steam.exe".to_string()),
            ..Default::default()
        };

        assert!(should_record_focus_distraction(
            &snapshot,
            snapshot.timestamp,
            Some(180_000),
            None,
        ));
        assert!(!should_record_focus_distraction(
            &snapshot,
            snapshot.timestamp + 180_000,
            Some(180_000),
            Some(snapshot.timestamp),
        ));
    }

    #[test]
    fn recent_input_and_productive_foreground_do_not_count() {
        let recent_input = HardwareSnapshot {
            timestamp: 300_000,
            foreground_process_name: Some("steam.exe".to_string()),
            ..Default::default()
        };
        assert!(!should_record_focus_distraction(
            &recent_input,
            recent_input.timestamp,
            Some(295_000),
            None,
        ));

        let productive = HardwareSnapshot {
            timestamp: 300_000,
            foreground_process_name: Some("Code.exe".to_string()),
            ..Default::default()
        };
        assert!(!should_record_focus_distraction(
            &productive,
            productive.timestamp,
            Some(180_000),
            None,
        ));
    }
}
