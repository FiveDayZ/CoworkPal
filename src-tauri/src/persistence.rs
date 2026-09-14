use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{
    app_state::AppState, commands::record_internal_achievement_event_buffered,
    models::date_key_from_timestamp,
};

const FLUSH_INTERVAL: Duration = Duration::from_secs(30);
const SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Default, Clone)]
pub struct PeriodicAchievementDelta {
    pub occurred_at: i64,
    pub online_seconds: u64,
    pub parts_delta: f64,
    pub insight_delta: f64,
    pub high_load_seconds: u64,
    pub thermal_warning_seconds: u64,
    pub disk_bytes_total: u64,
    pub network_bytes_total: u64,
    pub cpu_over_50_seconds: u64,
    pub memory_over_70_seconds: u64,
    pub gpu_over_70_seconds: u64,
    pub low_power_mode_enabled_seconds: u64,
    pub mouse_click_count: u64,
    pub keyboard_press_count: u64,
}

impl PeriodicAchievementDelta {
    fn merge(&mut self, other: Self) {
        self.occurred_at = self.occurred_at.max(other.occurred_at);
        self.online_seconds = self.online_seconds.saturating_add(other.online_seconds);
        self.parts_delta += other.parts_delta;
        self.insight_delta += other.insight_delta;
        self.high_load_seconds = self
            .high_load_seconds
            .saturating_add(other.high_load_seconds);
        self.thermal_warning_seconds = self
            .thermal_warning_seconds
            .saturating_add(other.thermal_warning_seconds);
        self.disk_bytes_total = self.disk_bytes_total.saturating_add(other.disk_bytes_total);
        self.network_bytes_total = self
            .network_bytes_total
            .saturating_add(other.network_bytes_total);
        self.cpu_over_50_seconds = self
            .cpu_over_50_seconds
            .saturating_add(other.cpu_over_50_seconds);
        self.memory_over_70_seconds = self
            .memory_over_70_seconds
            .saturating_add(other.memory_over_70_seconds);
        self.gpu_over_70_seconds = self
            .gpu_over_70_seconds
            .saturating_add(other.gpu_over_70_seconds);
        self.low_power_mode_enabled_seconds = self
            .low_power_mode_enabled_seconds
            .saturating_add(other.low_power_mode_enabled_seconds);
        self.mouse_click_count = self
            .mouse_click_count
            .saturating_add(other.mouse_click_count);
        self.keyboard_press_count = self
            .keyboard_press_count
            .saturating_add(other.keyboard_press_count);
    }

    fn has_online_time(&self) -> bool {
        self.online_seconds > 0
    }

    fn has_production(&self) -> bool {
        self.parts_delta > 0.0 || self.insight_delta > 0.0
    }

    fn has_hardware_activity(&self) -> bool {
        self.high_load_seconds > 0
            || self.thermal_warning_seconds > 0
            || self.disk_bytes_total > 0
            || self.network_bytes_total > 0
            || self.cpu_over_50_seconds > 0
            || self.memory_over_70_seconds > 0
            || self.gpu_over_70_seconds > 0
            || self.low_power_mode_enabled_seconds > 0
            || self.mouse_click_count > 0
            || self.keyboard_press_count > 0
    }
}

#[derive(Default)]
pub struct PersistenceCoordinator {
    workshop_dirty: AtomicBool,
    work_logs_dirty: AtomicBool,
    achievements_dirty: AtomicBool,
    pending_achievements: Mutex<BTreeMap<String, PeriodicAchievementDelta>>,
    flush_lock: Mutex<()>,
}

impl PersistenceCoordinator {
    pub fn mark_workshop_dirty(&self) {
        self.workshop_dirty.store(true, Ordering::Release);
    }

    pub fn mark_work_logs_dirty(&self) {
        self.work_logs_dirty.store(true, Ordering::Release);
    }

    pub fn mark_achievements_dirty(&self) {
        self.achievements_dirty.store(true, Ordering::Release);
    }

    pub async fn queue_periodic_achievement(&self, delta: PeriodicAchievementDelta) {
        let date = date_key_from_timestamp(delta.occurred_at);
        self.pending_achievements
            .lock()
            .await
            .entry(date)
            .or_default()
            .merge(delta);
    }

    async fn take_pending_achievements(&self) -> BTreeMap<String, PeriodicAchievementDelta> {
        std::mem::take(&mut *self.pending_achievements.lock().await)
    }

    async fn restore_pending_achievement(&self, date: String, delta: PeriodicAchievementDelta) {
        self.pending_achievements
            .lock()
            .await
            .entry(date)
            .or_default()
            .merge(delta);
    }
}

pub fn start_persistence_pump(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(FLUSH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;

        loop {
            interval.tick().await;
            if crate::IS_EXITING.load(Ordering::Acquire) {
                break;
            }
            if let Err(error) = flush_periodic_state(&app).await {
                tracing::warn!("periodic persistence flush failed: {error}");
            }
        }
    });
}

pub async fn flush_periodic_state(app: &AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "app state is not available".to_string())?;
    let _flush_guard = state.persistence.flush_lock.lock().await;
    flush_pending_achievements(app, state.inner()).await;
    flush_dirty_locked(state.inner()).await
}

pub async fn flush_achievements(state: &AppState) -> Result<(), String> {
    let _flush_guard = state.persistence.flush_lock.lock().await;
    flush_achievements_locked(state).await
}

pub async fn flush_before_shutdown(app: &AppHandle) -> Result<(), String> {
    tokio::time::timeout(SHUTDOWN_FLUSH_TIMEOUT, flush_periodic_state(app))
        .await
        .map_err(|_| "timed out while saving pending application data".to_string())?
}

async fn flush_pending_achievements(app: &AppHandle, state: &AppState) {
    let batches = state.persistence.take_pending_achievements().await;

    for (date, delta) in batches {
        if let Err(error) = record_periodic_achievement_batch(app, &date, &delta).await {
            tracing::warn!("failed to aggregate periodic achievement events: {error}");
            state
                .persistence
                .restore_pending_achievement(date, delta)
                .await;
        }
    }
}

async fn record_periodic_achievement_batch(
    app: &AppHandle,
    date: &str,
    delta: &PeriodicAchievementDelta,
) -> Result<(), String> {
    if delta.has_online_time() {
        record_internal_achievement_event_buffered(
            app,
            "app.active_minute",
            format!("app.active_period:{date}:{}", delta.occurred_at),
            delta.occurred_at,
            serde_json::json!({ "seconds": delta.online_seconds }),
        )
        .await?;
    }

    if delta.has_production() {
        record_internal_achievement_event_buffered(
            app,
            "workshop.production_tick",
            format!("workshop.production_period:{date}:{}", delta.occurred_at),
            delta.occurred_at,
            serde_json::json!({
                "partsDelta": delta.parts_delta,
                "insightDelta": delta.insight_delta,
            }),
        )
        .await?;
    }

    if delta.has_hardware_activity() {
        record_internal_achievement_event_buffered(
            app,
            "hardware.segment_rollup",
            format!("hardware.segment_period:{date}:{}", delta.occurred_at),
            delta.occurred_at,
            serde_json::json!({
                "highLoadSeconds": delta.high_load_seconds,
                "thermalWarningSeconds": delta.thermal_warning_seconds,
                "diskBytesTotal": delta.disk_bytes_total,
                "networkBytesTotal": delta.network_bytes_total,
                "cpuOver50Seconds": delta.cpu_over_50_seconds,
                "memoryOver70Seconds": delta.memory_over_70_seconds,
                "gpuOver70Seconds": delta.gpu_over_70_seconds,
                "lowPowerModeEnabledSeconds": delta.low_power_mode_enabled_seconds,
                "mouseClickCount": delta.mouse_click_count,
                "keyboardPressCount": delta.keyboard_press_count,
            }),
        )
        .await?;
    }

    Ok(())
}

async fn flush_dirty_locked(state: &AppState) -> Result<(), String> {
    let mut errors = Vec::new();

    if state
        .persistence
        .workshop_dirty
        .swap(false, Ordering::AcqRel)
    {
        let workshop = state.workshop.read().await;
        let snapshot = workshop.clone();
        let storage = state.storage.clone();
        let result = tokio::task::spawn_blocking(move || storage.save_workshop(&snapshot)).await;
        drop(workshop);
        if let Err(error) = flatten_save_result(result) {
            state.persistence.mark_workshop_dirty();
            errors.push(format!("save.json: {error}"));
        }
    }

    if state
        .persistence
        .work_logs_dirty
        .swap(false, Ordering::AcqRel)
    {
        let work_logs = state.work_logs.read().await;
        let snapshot = work_logs.clone();
        let storage = state.storage.clone();
        let result = tokio::task::spawn_blocking(move || storage.save_work_logs(&snapshot)).await;
        drop(work_logs);
        if let Err(error) = flatten_save_result(result) {
            state.persistence.mark_work_logs_dirty();
            errors.push(format!("work_logs.json: {error}"));
        }
    }

    if errors.is_empty() {
        if let Err(error) = flush_achievements_locked(state).await {
            errors.push(format!("achievements.json: {error}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

async fn flush_achievements_locked(state: &AppState) -> Result<(), String> {
    if !state
        .persistence
        .achievements_dirty
        .swap(false, Ordering::AcqRel)
    {
        return Ok(());
    }

    let achievements = state.achievements.read().await;
    let snapshot = achievements.clone();
    let storage = state.storage.clone();
    let result = tokio::task::spawn_blocking(move || storage.save_achievements(&snapshot)).await;
    drop(achievements);
    if let Err(error) = flatten_save_result(result) {
        state.persistence.mark_achievements_dirty();
        return Err(error);
    }
    Ok(())
}

fn flatten_save_result(
    result: Result<Result<(), String>, tokio::task::JoinError>,
) -> Result<(), String> {
    result.map_err(|error| format!("persistence task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_deltas_merge_without_losing_counts() {
        let mut total = PeriodicAchievementDelta {
            occurred_at: 10,
            online_seconds: 2,
            parts_delta: 1.25,
            mouse_click_count: 3,
            ..Default::default()
        };
        total.merge(PeriodicAchievementDelta {
            occurred_at: 20,
            online_seconds: 5,
            parts_delta: 2.75,
            mouse_click_count: 4,
            ..Default::default()
        });

        assert_eq!(total.occurred_at, 20);
        assert_eq!(total.online_seconds, 7);
        assert_eq!(total.parts_delta, 4.0);
        assert_eq!(total.mouse_click_count, 7);
    }

    #[test]
    fn dirty_flag_keeps_mutations_that_arrive_during_a_flush() {
        let coordinator = PersistenceCoordinator::default();
        coordinator.mark_work_logs_dirty();
        assert!(coordinator.work_logs_dirty.swap(false, Ordering::AcqRel));
        coordinator.mark_work_logs_dirty();
        assert!(coordinator.work_logs_dirty.load(Ordering::Acquire));
    }
}
