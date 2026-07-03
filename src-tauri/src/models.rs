use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};

const WORK_LOG_TIME_SLICE_MS: i64 = 15 * 60 * 1000;
const MAX_WORK_LOG_TIME_SLICES: usize = 128;
const WORK_LOG_DAY_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareSnapshot {
    pub timestamp: i64,
    pub cpu_usage_percent: Option<f32>,
    pub gpu_usage_percent: Option<f32>,
    pub memory_usage_percent: Option<f32>,
    pub cpu_temperature_celsius: Option<f32>,
    pub gpu_temperature_celsius: Option<f32>,
    pub disk_read_bytes_per_second: Option<f32>,
    pub disk_write_bytes_per_second: Option<f32>,
    pub network_download_bytes_per_second: Option<f32>,
    pub network_upload_bytes_per_second: Option<f32>,
    pub cpu_name: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_memory_used_bytes: Option<u64>,
    pub gpu_memory_total_bytes: Option<u64>,
    pub total_memory_bytes: Option<u64>,
    pub used_memory_bytes: Option<u64>,
    pub cpu_physical_core_count: Option<u32>,
    pub cpu_logical_core_count: Option<u32>,
    pub device_inventory: HardwareDeviceInventory,
    pub processes: Vec<ProcessUsageSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareMetricsSnapshot {
    pub timestamp: i64,
    pub cpu_usage_percent: Option<f32>,
    pub gpu_usage_percent: Option<f32>,
    pub memory_usage_percent: Option<f32>,
    pub cpu_temperature_celsius: Option<f32>,
    pub gpu_temperature_celsius: Option<f32>,
    pub disk_read_bytes_per_second: Option<f32>,
    pub disk_write_bytes_per_second: Option<f32>,
    pub network_download_bytes_per_second: Option<f32>,
    pub network_upload_bytes_per_second: Option<f32>,
    pub cpu_name: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_memory_used_bytes: Option<u64>,
    pub gpu_memory_total_bytes: Option<u64>,
    pub total_memory_bytes: Option<u64>,
    pub used_memory_bytes: Option<u64>,
    pub cpu_physical_core_count: Option<u32>,
    pub cpu_logical_core_count: Option<u32>,
}

impl Default for HardwareMetricsSnapshot {
    fn default() -> Self {
        Self {
            timestamp: current_timestamp_ms(),
            cpu_usage_percent: None,
            gpu_usage_percent: None,
            memory_usage_percent: None,
            cpu_temperature_celsius: None,
            gpu_temperature_celsius: None,
            disk_read_bytes_per_second: None,
            disk_write_bytes_per_second: None,
            network_download_bytes_per_second: None,
            network_upload_bytes_per_second: None,
            cpu_name: None,
            gpu_name: None,
            gpu_memory_used_bytes: None,
            gpu_memory_total_bytes: None,
            total_memory_bytes: None,
            used_memory_bytes: None,
            cpu_physical_core_count: None,
            cpu_logical_core_count: None,
        }
    }
}

impl From<&HardwareSnapshot> for HardwareMetricsSnapshot {
    fn from(snapshot: &HardwareSnapshot) -> Self {
        Self {
            timestamp: snapshot.timestamp,
            cpu_usage_percent: snapshot.cpu_usage_percent,
            gpu_usage_percent: snapshot.gpu_usage_percent,
            memory_usage_percent: snapshot.memory_usage_percent,
            cpu_temperature_celsius: snapshot.cpu_temperature_celsius,
            gpu_temperature_celsius: snapshot.gpu_temperature_celsius,
            disk_read_bytes_per_second: snapshot.disk_read_bytes_per_second,
            disk_write_bytes_per_second: snapshot.disk_write_bytes_per_second,
            network_download_bytes_per_second: snapshot.network_download_bytes_per_second,
            network_upload_bytes_per_second: snapshot.network_upload_bytes_per_second,
            cpu_name: snapshot.cpu_name.clone(),
            gpu_name: snapshot.gpu_name.clone(),
            gpu_memory_used_bytes: snapshot.gpu_memory_used_bytes,
            gpu_memory_total_bytes: snapshot.gpu_memory_total_bytes,
            total_memory_bytes: snapshot.total_memory_bytes,
            used_memory_bytes: snapshot.used_memory_bytes,
            cpu_physical_core_count: snapshot.cpu_physical_core_count,
            cpu_logical_core_count: snapshot.cpu_logical_core_count,
        }
    }
}

impl Default for HardwareSnapshot {
    fn default() -> Self {
        Self {
            timestamp: current_timestamp_ms(),
            cpu_usage_percent: None,
            gpu_usage_percent: None,
            memory_usage_percent: None,
            cpu_temperature_celsius: None,
            gpu_temperature_celsius: None,
            disk_read_bytes_per_second: None,
            disk_write_bytes_per_second: None,
            network_download_bytes_per_second: None,
            network_upload_bytes_per_second: None,
            cpu_name: None,
            gpu_name: None,
            gpu_memory_used_bytes: None,
            gpu_memory_total_bytes: None,
            total_memory_bytes: None,
            used_memory_bytes: None,
            cpu_physical_core_count: None,
            cpu_logical_core_count: None,
            device_inventory: HardwareDeviceInventory::default(),
            processes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProcessUsageSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_usage_percent: f32,
    pub memory_bytes: u64,
    pub disk_read_bytes_per_second: Option<f32>,
    pub disk_write_bytes_per_second: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareDeviceInventory {
    pub motherboard: Vec<HardwareDeviceInfo>,
    pub memory_modules: Vec<MemoryModuleInfo>,
    pub gpus: Vec<HardwareDeviceInfo>,
    pub displays: Vec<HardwareDeviceInfo>,
    pub disks: Vec<HardwareDeviceInfo>,
    pub audio_devices: Vec<HardwareDeviceInfo>,
    pub network_adapters: Vec<HardwareDeviceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HardwareDeviceInfo {
    pub name: String,
    pub detail: Option<String>,
    pub vendor: Option<String>,
    pub capacity_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MemoryModuleInfo {
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub capacity_bytes: Option<u64>,
    pub speed_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatState {
    Idle,
    RepairLight,
    RepairHeavy,
    TemperatureCheck,
    MemoryCrowded,
    DataSorting,
    Sleep,
    Interactive,
    Celebrate,
    Hidden,
    /// Continuously working too long without a real break. Reuses existing
    /// animation; surfaced via the speech bubble so the user is nudged to pause.
    Fatigued,
    /// No input for an extended period while the machine stayed "busy" — a
    /// soft prompt to step away from the desk. Reuses existing animation.
    NeedsBreak,
    /// An active focus session is underway and the user is engaged.
    DeepWork,
    /// An active focus session is underway but the user appears distracted.
    Distracted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonitorMetric {
    Cpu,
    Ram,
    Disk,
    Network,
    Gpu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonitorBarMode {
    Micro,
    Default,
    Expanded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatStateChangedEvent {
    pub timestamp: i64,
    pub cat_state: CatState,
    pub cat_message: String,
}

/// Mutable per-process CoreCat runtime state, gathered behind a single lock so
/// that the read → decide → write cycle in `PetStateService::update_for_snapshot`
/// is atomic and cannot be interleaved by a concurrent settings command or
/// snapshot tick. Previously these were five separate `RwLock` fields, which left
/// a window for stale-state decisions and lost updates.
#[derive(Debug, Clone)]
pub struct CatRuntimeState {
    pub cat_state: CatState,
    pub cat_message: String,
    pub last_cat_state_changed_at: i64,
    /// When the temperature first became "safe" (below the exit threshold), if ever.
    pub temperature_safe_since: Option<i64>,
    /// Whether at least one pet state has been emitted since launch.
    pub has_emitted_cat_state: bool,
    /// Timestamp of the most recent user input (mouse click / key press).
    /// Drives distraction detection for focus sessions and break reminders.
    pub last_input_at: Option<i64>,
    /// When the current stretch of continuous work began (input present and not
    /// in a break). Cleared after a sufficiently long idle gap so we only nudge
    /// the user when they have genuinely been at it for a long stretch.
    pub continuous_work_since: Option<i64>,
    /// id of the currently active focus session, if any. Drives the DeepWork /
    /// Distracted pet states.
    pub active_focus_session_id: Option<String>,
    /// Focus session timing copied into the runtime so pet ticks can decide
    /// focus fatigue without locking the persisted focus session book.
    pub active_focus_started_at: Option<i64>,
    pub active_focus_planned_duration_ms: Option<i64>,
    /// Short post-focus wellness nudge. Completed focus asks for a break;
    /// abandoned focus marks fatigue for a brief period.
    pub focus_nudge_state: Option<CatState>,
    pub focus_nudge_until: Option<i64>,
    /// Timestamp of the most recently counted distraction, so a single
    /// continuous distracted stretch only increments distraction_count once.
    pub last_distraction_at: Option<i64>,
}

impl Default for CatRuntimeState {
    fn default() -> Self {
        Self {
            cat_state: CatState::Idle,
            cat_message: "CoreCat 正在待命。".to_string(),
            last_cat_state_changed_at: current_timestamp_ms(),
            temperature_safe_since: None,
            has_emitted_cat_state: false,
            last_input_at: None,
            continuous_work_since: None,
            active_focus_session_id: None,
            active_focus_started_at: None,
            active_focus_planned_duration_ms: None,
            focus_nudge_state: None,
            focus_nudge_until: None,
            last_distraction_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,
    pub launch_at_startup: bool,
    pub is_cat_visible: bool,
    pub is_monitor_bar_visible: bool,
    pub enable_sleep_mode: bool,
    pub enable_sound: bool,
    pub enable_notifications: bool,
    pub is_production_paused: bool,
    pub enable_low_power_mode: bool,
    pub enable_static_cat_mode: bool,
    pub enable_pet_bubble: bool,
    pub show_monitor_data_in_taskbar: bool,
    pub sampling_interval_ms: u64,
    pub background_sampling_interval_ms: u64,
    pub data_sorting_cpu_threshold: f32,
    pub cat_size: f64,
    pub cat_opacity: f64,
    pub cat_window_x: f64,
    pub cat_window_y: f64,
    pub monitor_bar_x: f64,
    pub monitor_bar_y: f64,
    pub cpu_temperature_warning: f32,
    pub gpu_temperature_warning: f32,
    pub memory_crowded_threshold: f32,
    pub error_glitch_cpu_threshold: f32,
    pub theme_name: String,
    pub visible_monitor_metrics: Vec<MonitorMetric>,
    pub monitor_bar_mode: MonitorBarMode,
    pub visible_taskbar_metrics: Vec<MonitorMetric>,
    pub taskbar_monitor_mode: MonitorBarMode,
    pub cat_id: String,
}

pub const APP_SETTINGS_SCHEMA_VERSION: u32 = 2;

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: APP_SETTINGS_SCHEMA_VERSION,
            launch_at_startup: false,
            is_cat_visible: true,
            is_monitor_bar_visible: false,
            enable_sleep_mode: true,
            enable_sound: false,
            enable_notifications: false,
            is_production_paused: false,
            enable_low_power_mode: false,
            enable_static_cat_mode: false,
            enable_pet_bubble: true,
            show_monitor_data_in_taskbar: false,
            sampling_interval_ms: 2000,
            background_sampling_interval_ms: 5000,
            data_sorting_cpu_threshold: 40.0,
            cat_size: 1.0,
            cat_opacity: 0.95,
            cat_window_x: 1200.0,
            cat_window_y: 520.0,
            monitor_bar_x: 580.0,
            monitor_bar_y: 24.0,
            cpu_temperature_warning: 80.0,
            gpu_temperature_warning: 82.0,
            memory_crowded_threshold: 82.0,
            error_glitch_cpu_threshold: 96.0,
            theme_name: "coreworkpal".to_string(),
            visible_monitor_metrics: vec![
                MonitorMetric::Cpu,
                MonitorMetric::Ram,
                MonitorMetric::Gpu,
                MonitorMetric::Network,
            ],
            monitor_bar_mode: MonitorBarMode::Default,
            visible_taskbar_metrics: vec![
                MonitorMetric::Cpu,
                MonitorMetric::Ram,
                MonitorMetric::Gpu,
                MonitorMetric::Network,
            ],
            taskbar_monitor_mode: MonitorBarMode::Default,
            cat_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub launch_at_startup: Option<bool>,
    pub is_cat_visible: Option<bool>,
    pub is_monitor_bar_visible: Option<bool>,
    pub enable_sleep_mode: Option<bool>,
    pub enable_sound: Option<bool>,
    pub enable_notifications: Option<bool>,
    pub is_production_paused: Option<bool>,
    pub enable_low_power_mode: Option<bool>,
    pub enable_static_cat_mode: Option<bool>,
    pub enable_pet_bubble: Option<bool>,
    pub show_monitor_data_in_taskbar: Option<bool>,
    pub sampling_interval_ms: Option<u64>,
    pub background_sampling_interval_ms: Option<u64>,
    pub data_sorting_cpu_threshold: Option<f32>,
    pub cat_size: Option<f64>,
    pub cat_opacity: Option<f64>,
    pub cat_window_x: Option<f64>,
    pub cat_window_y: Option<f64>,
    pub monitor_bar_x: Option<f64>,
    pub monitor_bar_y: Option<f64>,
    pub cpu_temperature_warning: Option<f32>,
    pub gpu_temperature_warning: Option<f32>,
    pub memory_crowded_threshold: Option<f32>,
    pub error_glitch_cpu_threshold: Option<f32>,
    pub theme_name: Option<String>,
    pub visible_monitor_metrics: Option<Vec<MonitorMetric>>,
    pub monitor_bar_mode: Option<MonitorBarMode>,
    pub visible_taskbar_metrics: Option<Vec<MonitorMetric>>,
    pub taskbar_monitor_mode: Option<MonitorBarMode>,
    pub cat_id: Option<String>,
}

impl AppSettings {
    pub fn apply_patch(&mut self, patch: AppSettingsPatch) {
        if let Some(value) = patch.launch_at_startup {
            self.launch_at_startup = value;
        }
        if let Some(value) = patch.is_cat_visible {
            self.is_cat_visible = value;
        }
        if let Some(value) = patch.is_monitor_bar_visible {
            self.is_monitor_bar_visible = value;
        }
        if let Some(value) = patch.enable_sleep_mode {
            self.enable_sleep_mode = value;
        }
        if let Some(value) = patch.enable_sound {
            self.enable_sound = value;
        }
        if let Some(value) = patch.enable_notifications {
            self.enable_notifications = value;
        }
        if let Some(value) = patch.is_production_paused {
            self.is_production_paused = value;
        }
        if let Some(value) = patch.enable_low_power_mode {
            self.enable_low_power_mode = value;
        }
        if let Some(value) = patch.enable_static_cat_mode {
            self.enable_static_cat_mode = value;
        }
        if let Some(value) = patch.enable_pet_bubble {
            self.enable_pet_bubble = value;
        }
        if let Some(value) = patch.show_monitor_data_in_taskbar {
            self.show_monitor_data_in_taskbar = value;
        }
        if let Some(value) = patch.sampling_interval_ms {
            self.sampling_interval_ms = value.max(1000);
        }
        if let Some(value) = patch.background_sampling_interval_ms {
            self.background_sampling_interval_ms = value.max(3000);
        }
        if let Some(value) = patch.data_sorting_cpu_threshold {
            self.data_sorting_cpu_threshold = value.clamp(10.0, 95.0);
        }
        if let Some(value) = patch.cat_size {
            self.cat_size = value.clamp(0.5, 2.0);
        }
        if let Some(value) = patch.cat_opacity {
            self.cat_opacity = value.clamp(0.1, 1.0);
        }
        if let Some(value) = patch.cat_window_x {
            self.cat_window_x = value;
        }
        if let Some(value) = patch.cat_window_y {
            self.cat_window_y = value;
        }
        if let Some(value) = patch.monitor_bar_x {
            self.monitor_bar_x = value;
        }
        if let Some(value) = patch.monitor_bar_y {
            self.monitor_bar_y = value;
        }
        if let Some(value) = patch.cpu_temperature_warning {
            self.cpu_temperature_warning = value.clamp(50.0, 110.0);
        }
        if let Some(value) = patch.gpu_temperature_warning {
            self.gpu_temperature_warning = value.clamp(50.0, 110.0);
        }
        if let Some(value) = patch.memory_crowded_threshold {
            self.memory_crowded_threshold = value.clamp(50.0, 98.0);
        }
        if let Some(value) = patch.error_glitch_cpu_threshold {
            self.error_glitch_cpu_threshold = value.clamp(50.0, 100.0);
        }
        if let Some(value) = patch.theme_name {
            self.theme_name = value;
        }
        if let Some(value) = patch.visible_monitor_metrics {
            self.visible_monitor_metrics = value;
        }
        if let Some(value) = patch.monitor_bar_mode {
            self.monitor_bar_mode = value;
        }
        if let Some(value) = patch.visible_taskbar_metrics {
            self.visible_taskbar_metrics = value;
        }
        if let Some(value) = patch.taskbar_monitor_mode {
            self.taskbar_monitor_mode = value;
        }
        if let Some(value) = patch.cat_id {
            self.cat_id = value;
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModuleUpgradeLevels {
    pub parts: u32,
    pub process: u32,
}

impl Default for ModuleUpgradeLevels {
    fn default() -> Self {
        Self {
            parts: 1,
            process: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkshopModuleLevels {
    pub cpu: ModuleUpgradeLevels,
    pub gpu: ModuleUpgradeLevels,
    pub ram: ModuleUpgradeLevels,
    pub network: ModuleUpgradeLevels,
    pub temperature: ModuleUpgradeLevels,
    pub disk: ModuleUpgradeLevels,
}

impl Default for WorkshopModuleLevels {
    fn default() -> Self {
        Self {
            cpu: ModuleUpgradeLevels::default(),
            gpu: ModuleUpgradeLevels::default(),
            ram: ModuleUpgradeLevels::default(),
            network: ModuleUpgradeLevels::default(),
            temperature: ModuleUpgradeLevels::default(),
            disk: ModuleUpgradeLevels::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkshopState {
    pub schema_version: u32,
    pub parts: f64,
    pub insight: f64,
    pub workshop_level: u32,
    pub cat_affinity_level: u32,
    pub module_levels: WorkshopModuleLevels,
    pub last_production_time: i64,
    pub total_online_seconds: u64,
    pub today_parts: f64,
    pub today_insight: f64,
    pub last_daily_reset_date: String,
}

impl Default for WorkshopState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            parts: 280.0,
            insight: 12.0,
            workshop_level: 1,
            cat_affinity_level: 1,
            module_levels: WorkshopModuleLevels::default(),
            last_production_time: current_timestamp_ms(),
            total_online_seconds: 0,
            today_parts: 0.0,
            today_insight: 0.0,
            last_daily_reset_date: Local::now().format("%Y-%m-%d").to_string(),
        }
    }
}

/// Lifecycle of a single focus ritual session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusSessionStatus {
    /// Running now; the user is (nominally) heads-down on the task.
    Active,
    /// Completed normally; focus_quality has been computed.
    Completed,
    /// The user ended it early.
    Abandoned,
}

/// One "deliver a task to CoreCat and stay focused" ritual session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FocusSession {
    /// Stable id (timestamp-derived) so the frontend can track the live session.
    pub id: String,
    /// What the user said they'd focus on.
    pub task_label: String,
    /// Planned duration the user committed to (25/50 min, etc.).
    pub planned_duration_seconds: u64,
    pub started_at: i64,
    /// Set when the session leaves the Active state.
    pub ended_at: Option<i64>,
    pub status: FocusSessionStatus,
    /// Times the user was flagged as distracted during this session.
    pub distraction_count: u32,
    /// 0.0..=1.0, computed on completion from distraction_count.
    pub focus_quality: f64,
}

impl Default for FocusSession {
    fn default() -> Self {
        Self {
            id: String::new(),
            task_label: String::new(),
            planned_duration_seconds: 25 * 60,
            started_at: 0,
            ended_at: None,
            status: FocusSessionStatus::Active,
            distraction_count: 0,
            focus_quality: 0.0,
        }
    }
}

/// Persisted history of focus sessions (only completed/abandoned matter for
/// stats, but Active is kept too so a crash mid-session can be recovered).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FocusSessionBook {
    pub schema_version: u32,
    pub sessions: Vec<FocusSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkLogBook {
    pub schema_version: u32,
    pub entries: BTreeMap<String, WorkLogEntry>,
}

impl Default for WorkLogBook {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkLogEntry {
    pub date: String,
    pub active_seconds: u64,
    pub sample_count: u64,
    pub cpu_load_points: f64,
    pub gpu_load_points: f64,
    pub memory_load_points: f64,
    pub thermal_pressure_points: f64,
    pub io_activity_points: f64,
    pub high_load_seconds: u64,
    pub cpu_over_50_seconds: u64,
    pub memory_over_70_seconds: u64,
    pub gpu_over_70_seconds: u64,
    pub cpu_over_80c_seconds: u64,
    pub gpu_over_80c_seconds: u64,
    pub disk_read_bytes_total: u64,
    pub disk_write_bytes_total: u64,
    pub network_download_bytes_total: u64,
    pub network_upload_bytes_total: u64,
    pub mouse_click_count: u64,
    pub keyboard_press_count: u64,
    pub process_usage: BTreeMap<String, WorkLogProcessUsage>,
    pub time_slices: Vec<WorkLogTimeSlice>,
    pub started_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkLogProcessUsage {
    pub name: String,
    pub sample_count: u64,
    pub active_sample_count: u64,
    pub observed_seconds: u64,
    pub active_seconds: u64,
    pub cpu_pressure_points: f64,
    pub memory_bytes_points: u64,
    pub memory_bytes_peak: u64,
    pub disk_read_bytes_total: u64,
    pub disk_write_bytes_total: u64,
}

#[derive(Debug, Clone, Default)]
struct ProcessUsageSampleAggregate {
    name: String,
    cpu_pressure_percent: f64,
    memory_bytes_total: u64,
    disk_read_bytes_total: u64,
    disk_write_bytes_total: u64,
    disk_rate_bytes_per_second: f64,
}

impl Default for WorkLogProcessUsage {
    fn default() -> Self {
        Self {
            name: String::new(),
            sample_count: 0,
            active_sample_count: 0,
            observed_seconds: 0,
            active_seconds: 0,
            cpu_pressure_points: 0.0,
            memory_bytes_points: 0,
            memory_bytes_peak: 0,
            disk_read_bytes_total: 0,
            disk_write_bytes_total: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkLogTimeSlice {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub active_seconds: u64,
    pub sample_count: u64,
    pub cpu_load_points: f64,
    pub gpu_load_points: f64,
    pub memory_load_points: f64,
    pub thermal_pressure_points: f64,
    pub io_activity_points: f64,
    pub disk_read_bytes_total: u64,
    pub disk_write_bytes_total: u64,
    pub network_download_bytes_total: u64,
    pub network_upload_bytes_total: u64,
    pub mouse_click_count: u64,
    pub keyboard_press_count: u64,
}

impl Default for WorkLogTimeSlice {
    fn default() -> Self {
        Self {
            start_timestamp: 0,
            end_timestamp: 0,
            active_seconds: 0,
            sample_count: 0,
            cpu_load_points: 0.0,
            gpu_load_points: 0.0,
            memory_load_points: 0.0,
            thermal_pressure_points: 0.0,
            io_activity_points: 0.0,
            disk_read_bytes_total: 0,
            disk_write_bytes_total: 0,
            network_download_bytes_total: 0,
            network_upload_bytes_total: 0,
            mouse_click_count: 0,
            keyboard_press_count: 0,
        }
    }
}

impl Default for WorkLogEntry {
    fn default() -> Self {
        Self {
            date: today_key(),
            active_seconds: 0,
            sample_count: 0,
            cpu_load_points: 0.0,
            gpu_load_points: 0.0,
            memory_load_points: 0.0,
            thermal_pressure_points: 0.0,
            io_activity_points: 0.0,
            high_load_seconds: 0,
            cpu_over_50_seconds: 0,
            memory_over_70_seconds: 0,
            gpu_over_70_seconds: 0,
            cpu_over_80c_seconds: 0,
            gpu_over_80c_seconds: 0,
            disk_read_bytes_total: 0,
            disk_write_bytes_total: 0,
            network_download_bytes_total: 0,
            network_upload_bytes_total: 0,
            mouse_click_count: 0,
            keyboard_press_count: 0,
            process_usage: BTreeMap::new(),
            time_slices: Vec::new(),
            started_at: 0,
            updated_at: 0,
        }
    }
}

impl WorkLogEntry {
    pub fn new(date: String, timestamp: i64) -> Self {
        Self {
            date,
            started_at: timestamp,
            updated_at: timestamp,
            ..Default::default()
        }
    }

    pub fn record_snapshot(&mut self, snapshot: &HardwareSnapshot, timestamp: i64) {
        if self.started_at == 0 {
            self.started_at = timestamp;
        }

        let delta_seconds = if self.updated_at > 0 {
            timestamp
                .saturating_sub(self.updated_at)
                .clamp(0, 60_000) as u64
                / 1000
        } else {
            0
        };

        let cpu = bounded_metric(snapshot.cpu_usage_percent);
        let gpu = bounded_metric(snapshot.gpu_usage_percent);
        let memory = bounded_metric(snapshot.memory_usage_percent);
        let thermal = thermal_pressure(snapshot);
        let io = io_activity(snapshot);
        let disk_read_delta = rate_total_bytes(snapshot.disk_read_bytes_per_second, delta_seconds);
        let disk_write_delta = rate_total_bytes(snapshot.disk_write_bytes_per_second, delta_seconds);
        let network_download_delta =
            rate_total_bytes(snapshot.network_download_bytes_per_second, delta_seconds);
        let network_upload_delta =
            rate_total_bytes(snapshot.network_upload_bytes_per_second, delta_seconds);

        self.active_seconds = self.active_seconds.saturating_add(delta_seconds);
        self.sample_count = self.sample_count.saturating_add(1);
        self.cpu_load_points += cpu;
        self.gpu_load_points += gpu;
        self.memory_load_points += memory;
        self.thermal_pressure_points += thermal;
        self.io_activity_points += io;

        if cpu > 50.0 {
            self.cpu_over_50_seconds = self.cpu_over_50_seconds.saturating_add(delta_seconds);
        }
        if memory > 70.0 {
            self.memory_over_70_seconds =
                self.memory_over_70_seconds.saturating_add(delta_seconds);
        }
        if gpu > 70.0 {
            self.gpu_over_70_seconds = self.gpu_over_70_seconds.saturating_add(delta_seconds);
        }
        if snapshot.cpu_temperature_celsius.unwrap_or(0.0) > 80.0 {
            self.cpu_over_80c_seconds =
                self.cpu_over_80c_seconds.saturating_add(delta_seconds);
        }
        if snapshot.gpu_temperature_celsius.unwrap_or(0.0) > 80.0 {
            self.gpu_over_80c_seconds =
                self.gpu_over_80c_seconds.saturating_add(delta_seconds);
        }

        self.disk_read_bytes_total = self.disk_read_bytes_total.saturating_add(disk_read_delta);
        self.disk_write_bytes_total = self.disk_write_bytes_total.saturating_add(disk_write_delta);
        self.network_download_bytes_total = self
            .network_download_bytes_total
            .saturating_add(network_download_delta);
        self.network_upload_bytes_total = self
            .network_upload_bytes_total
            .saturating_add(network_upload_delta);

        if cpu > 50.0 || memory > 70.0 || gpu > 70.0 || thermal >= 55.0 || io >= 55.0 {
            self.high_load_seconds = self.high_load_seconds.saturating_add(delta_seconds);
        }

        self.record_snapshot_time_slice(
            timestamp,
            delta_seconds,
            cpu,
            gpu,
            memory,
            thermal,
            io,
            disk_read_delta,
            disk_write_delta,
            network_download_delta,
            network_upload_delta,
        );
        self.record_process_usage(snapshot, delta_seconds);

        self.updated_at = timestamp;
    }

    fn record_process_usage(&mut self, snapshot: &HardwareSnapshot, delta_seconds: u64) {
        if delta_seconds == 0 || snapshot.processes.is_empty() {
            return;
        }

        let mut process_samples: BTreeMap<String, ProcessUsageSampleAggregate> = BTreeMap::new();

        for process in &snapshot.processes {
            let Some(name) = normalize_process_name(&process.name) else {
                continue;
            };
            if should_ignore_process_name(&name) {
                continue;
            }

            let disk_read_delta =
                rate_total_bytes(process.disk_read_bytes_per_second, delta_seconds);
            let disk_write_delta =
                rate_total_bytes(process.disk_write_bytes_per_second, delta_seconds);
            let disk_rate = process.disk_read_bytes_per_second.unwrap_or(0.0).max(0.0)
                + process.disk_write_bytes_per_second.unwrap_or(0.0).max(0.0);
            let cpu = process.cpu_usage_percent.clamp(0.0, 1000.0) as f64;

            let aggregate = process_samples
                .entry(name.to_ascii_lowercase())
                .or_insert_with(|| ProcessUsageSampleAggregate {
                    name: name.clone(),
                    ..Default::default()
                });
            aggregate.cpu_pressure_percent += cpu;
            aggregate.memory_bytes_total = aggregate
                .memory_bytes_total
                .saturating_add(process.memory_bytes);
            aggregate.disk_read_bytes_total = aggregate
                .disk_read_bytes_total
                .saturating_add(disk_read_delta);
            aggregate.disk_write_bytes_total = aggregate
                .disk_write_bytes_total
                .saturating_add(disk_write_delta);
            aggregate.disk_rate_bytes_per_second += disk_rate as f64;
        }

        for (key, aggregate) in process_samples {
            let cpu_pressure_percent = aggregate.cpu_pressure_percent.clamp(0.0, 1000.0);
            let is_active = cpu_pressure_percent >= 1.0
                || aggregate.disk_rate_bytes_per_second >= 32.0 * 1024.0;

            let usage = self
                .process_usage
                .entry(key)
                .or_insert_with(|| WorkLogProcessUsage {
                    name: aggregate.name.clone(),
                    ..Default::default()
                });
            usage.sample_count = usage.sample_count.saturating_add(1);
            usage.observed_seconds = usage.observed_seconds.saturating_add(delta_seconds);
            usage.cpu_pressure_points += cpu_pressure_percent * delta_seconds as f64;
            usage.memory_bytes_points = usage
                .memory_bytes_points
                .saturating_add(aggregate.memory_bytes_total.saturating_mul(delta_seconds));
            usage.memory_bytes_peak = usage.memory_bytes_peak.max(aggregate.memory_bytes_total);
            usage.disk_read_bytes_total =
                usage.disk_read_bytes_total.saturating_add(aggregate.disk_read_bytes_total);
            usage.disk_write_bytes_total = usage
                .disk_write_bytes_total
                .saturating_add(aggregate.disk_write_bytes_total);

            if is_active {
                usage.active_sample_count = usage.active_sample_count.saturating_add(1);
                usage.active_seconds = usage.active_seconds.saturating_add(delta_seconds);
            }
        }
    }

    pub fn record_input_activity_at(
        &mut self,
        mouse_clicks: u64,
        keyboard_presses: u64,
        timestamp: i64,
    ) {
        self.mouse_click_count = self.mouse_click_count.saturating_add(mouse_clicks);
        self.keyboard_press_count = self
            .keyboard_press_count
            .saturating_add(keyboard_presses);
        let slice = self.ensure_time_slice(timestamp);
        slice.mouse_click_count = slice.mouse_click_count.saturating_add(mouse_clicks);
        slice.keyboard_press_count = slice
            .keyboard_press_count
            .saturating_add(keyboard_presses);
    }

    #[allow(clippy::too_many_arguments)]
    fn record_snapshot_time_slice(
        &mut self,
        timestamp: i64,
        delta_seconds: u64,
        cpu: f64,
        gpu: f64,
        memory: f64,
        thermal: f64,
        io: f64,
        disk_read_delta: u64,
        disk_write_delta: u64,
        network_download_delta: u64,
        network_upload_delta: u64,
    ) {
        let slice = self.ensure_time_slice(timestamp);
        slice.active_seconds = slice.active_seconds.saturating_add(delta_seconds);
        slice.sample_count = slice.sample_count.saturating_add(1);
        slice.cpu_load_points += cpu;
        slice.gpu_load_points += gpu;
        slice.memory_load_points += memory;
        slice.thermal_pressure_points += thermal;
        slice.io_activity_points += io;
        slice.disk_read_bytes_total = slice.disk_read_bytes_total.saturating_add(disk_read_delta);
        slice.disk_write_bytes_total = slice
            .disk_write_bytes_total
            .saturating_add(disk_write_delta);
        slice.network_download_bytes_total = slice
            .network_download_bytes_total
            .saturating_add(network_download_delta);
        slice.network_upload_bytes_total = slice
            .network_upload_bytes_total
            .saturating_add(network_upload_delta);
    }

    fn ensure_time_slice(&mut self, timestamp: i64) -> &mut WorkLogTimeSlice {
        let start_timestamp = align_work_log_time_slice(timestamp);
        if self
            .time_slices
            .iter()
            .all(|slice| slice.start_timestamp != start_timestamp)
        {
            self.time_slices.push(WorkLogTimeSlice {
                start_timestamp,
                end_timestamp: start_timestamp + WORK_LOG_TIME_SLICE_MS,
                ..Default::default()
            });
            self.time_slices
                .sort_by_key(|slice| slice.start_timestamp);
            if self.time_slices.len() > MAX_WORK_LOG_TIME_SLICES {
                let overflow = self.time_slices.len() - MAX_WORK_LOG_TIME_SLICES;
                self.time_slices.drain(0..overflow);
            }
        }

        self.time_slices
            .iter_mut()
            .find(|slice| slice.start_timestamp == start_timestamp)
            .expect("time slice exists after insertion")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogReport {
    pub date: String,
    pub total_score: u32,
    pub summary: String,
    pub active_seconds: u64,
    pub sample_count: u64,
    pub dimensions: Vec<WorkLogScoreDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogScoreDimension {
    pub key: String,
    pub title: String,
    pub score: u32,
    pub max_score: u32,
    pub value: String,
    pub explanation: String,
    pub facts: Vec<WorkLogMetricFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogMetricFact {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWorkAssessment {
    pub date: String,
    pub day_type: WorkDayType,
    pub day_type_title: String,
    pub rarity: WorkCardRarity,
    pub title: WorkDayTitle,
    pub corecat_commentary: CoreCatCommentary,
    pub workprint: WorkprintSummary,
    pub baseline: BaselineComparison,
    pub timeline: Vec<WorkTimelineSegment>,
    pub mvp_segments: Vec<WorkTimelineSegment>,
    pub highlights: Vec<AssessmentInsight>,
    pub risks: Vec<AssessmentInsight>,
    pub suggestions: Vec<AssessmentInsight>,
    pub process_insights: Vec<ProcessUsageInsight>,
    pub dimensions: Vec<WorkLogScoreDimension>,
    pub score: u32,
    pub corecat_summary: String,
    pub badge_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessUsageInsight {
    pub name: String,
    pub observed_seconds: u64,
    pub active_seconds: u64,
    pub sample_count: u64,
    pub active_sample_count: u64,
    pub cpu_pressure_percent: f64,
    pub average_memory_bytes: u64,
    pub memory_bytes_peak: u64,
    pub disk_read_bytes_total: u64,
    pub disk_write_bytes_total: u64,
    pub rank_label: String,
    pub summary: String,
    pub severity: InsightSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWorkAssessmentSummary {
    pub date: String,
    pub day_type: WorkDayType,
    pub day_type_title: String,
    pub rarity: WorkCardRarity,
    pub title: WorkDayTitle,
    pub workprint: WorkprintSummary,
    pub score: u32,
    pub corecat_summary: String,
    pub badge_ids: Vec<String>,
    pub has_timeline: bool,
    pub has_data: bool,
}

/// One cell of the circadian-rhythm heatmap: aggregate of all historical
/// 15-min slices that fell into a given wall-clock hour (0-23) or weekday (0-6).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RhythmBucket {
    /// 0-23 for hour buckets, 0-6 (Mon-Sun) for weekday buckets.
    pub index: u8,
    pub active_seconds: u64,
    /// Average daily work-print score for slices in this bucket, 0 if unknown.
    pub avg_focus_score: f64,
    /// How many distinct days contributed data to this bucket.
    pub sample_days: u32,
}

/// A user's personal activity rhythm, aggregated from work-log time slices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RhythmProfile {
    pub hour_buckets: Vec<RhythmBucket>,
    pub weekday_buckets: Vec<RhythmBucket>,
    /// Hours (0-23) ranked highest by combined activity + focus, top 3.
    pub peak_hours: Vec<u8>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWorkAssessmentTrend {
    pub sample_days: u32,
    pub average_score: u32,
    pub best_date: Option<String>,
    pub best_score: Option<u32>,
    pub best_day_type: WorkDayType,
    pub best_day_type_title: String,
    pub dominant_day_type: WorkDayType,
    pub dominant_day_type_title: String,
    pub timeline_days: u32,
    pub score_delta: i32,
    pub summary: String,
    pub insights: Vec<AssessmentInsight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCardRarity {
    pub tier: String,
    pub label: String,
    pub score: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkDayTitle {
    pub family: String,
    pub title: String,
    pub level: u32,
    pub progress: u32,
    pub next_level_at: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCatCommentary {
    pub tone: CoreCatCommentTone,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CoreCatCommentTone {
    Encouragement,
    Tease,
    Warning,
    Celebration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum WorkDayType {
    DeepFocus,
    BuildBurst,
    ArchiveFlow,
    PressureRepair,
    StableMaintenance,
    FragmentedSwitching,
    LowLoadCompanion,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkprintSummary {
    pub label: String,
    pub description: String,
    pub pixel_grid: Vec<u8>,
    pub width: u8,
    pub height: u8,
    pub load_shape: f64,
    pub input_rhythm: f64,
    pub io_intensity: f64,
    pub thermal_pressure: f64,
    pub continuity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineComparison {
    pub sample_days: u32,
    pub active_seconds_delta_ratio: f64,
    pub load_delta_ratio: f64,
    pub io_delta_ratio: f64,
    pub thermal_delta_ratio: f64,
    pub input_delta_ratio: f64,
    pub metrics: Vec<BaselineMetric>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineMetric {
    pub key: String,
    pub label: String,
    pub current_value: String,
    pub baseline_value: String,
    pub delta_ratio: f64,
    pub tone: InsightSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTimelineSegment {
    pub start_time: String,
    pub end_time: String,
    pub kind: WorkTimelineSegmentKind,
    pub intensity: f64,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkTimelineSegmentKind {
    IdleCompanion,
    SteadyProgress,
    DeepFocus,
    BuildPeak,
    ArchiveFlow,
    MemoryCrowded,
    TemperatureWarning,
    PressureRepair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentInsight {
    pub title: String,
    pub body: String,
    pub severity: InsightSeverity,
    pub metric_value: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InsightSeverity {
    Positive,
    Neutral,
    Warning,
}

impl WorkLogReport {
    pub fn from_entry(entry: WorkLogEntry) -> Self {
        let samples = entry.sample_count.max(1) as f64;
        let cpu_avg = entry.cpu_load_points / samples;
        let gpu_avg = entry.gpu_load_points / samples;
        let memory_avg = entry.memory_load_points / samples;
        let thermal_avg = entry.thermal_pressure_points / samples;
        let io_avg = entry.io_activity_points / samples;
        let hours = entry.active_seconds as f64 / 3600.0;
        let high_load_ratio = if entry.active_seconds > 0 {
            entry.high_load_seconds as f64 / entry.active_seconds as f64
        } else {
            0.0
        };
        let cpu_over_50_ratio = duration_ratio(entry.cpu_over_50_seconds, entry.active_seconds);
        let memory_over_70_ratio =
            duration_ratio(entry.memory_over_70_seconds, entry.active_seconds);
        let gpu_over_70_ratio = duration_ratio(entry.gpu_over_70_seconds, entry.active_seconds);
        let cpu_over_80c_ratio =
            duration_ratio(entry.cpu_over_80c_seconds, entry.active_seconds);
        let gpu_over_80c_ratio =
            duration_ratio(entry.gpu_over_80c_seconds, entry.active_seconds);
        let disk_total = entry
            .disk_read_bytes_total
            .saturating_add(entry.disk_write_bytes_total);
        let network_total = entry
            .network_download_bytes_total
            .saturating_add(entry.network_upload_bytes_total);

        let duration_score = ((hours / 6.0) * 30.0).clamp(0.0, 30.0);
        let sustained_load_index = cpu_over_50_ratio * 32.0
            + memory_over_70_ratio * 28.0
            + gpu_over_70_ratio * 24.0;
        let load_score = ((cpu_avg * 0.27
            + memory_avg * 0.22
            + gpu_avg * 0.20
            + io_avg * 0.08
            + sustained_load_index)
            / 100.0
            * 25.0)
            .clamp(0.0, 25.0);
        let disk_volume_score = volume_score(disk_total, 3.0 * 1024.0 * 1024.0 * 1024.0);
        let network_volume_score = volume_score(network_total, 1024.0 * 1024.0 * 1024.0);
        let complexity_score = (high_load_ratio * 5.0
            + disk_volume_score * 7.0
            + network_volume_score * 5.0
            + (io_avg / 100.0) * 3.0)
            .clamp(0.0, 20.0);
        let stability_score = (15.0
            - (thermal_avg / 100.0) * 5.0
            - cpu_over_80c_ratio * 5.0
            - gpu_over_80c_ratio * 5.0)
            .clamp(0.0, 15.0);
        let input_score = volume_score(entry.mouse_click_count, 1_500.0) * 3.0
            + volume_score(entry.keyboard_press_count, 5_000.0) * 3.0;
        let continuity_score = (((hours / 8.0) * 4.0) + input_score).clamp(0.0, 10.0);
        let total_score = (duration_score
            + load_score
            + complexity_score
            + stability_score
            + continuity_score)
            .round()
            .clamp(0.0, 100.0) as u32;

        Self {
            date: entry.date.clone(),
            total_score,
            summary: summarize_work_score(total_score),
            active_seconds: entry.active_seconds,
            sample_count: entry.sample_count,
            dimensions: vec![
                WorkLogScoreDimension {
                    key: "duration".to_string(),
                    title: "CoreCat 运行时长".to_string(),
                    score: duration_score.round() as u32,
                    max_score: 30,
                    value: format!("{hours:.1}h"),
                    explanation: "按当日 CoreCat 连续在线与生产观察时长折算，6 小时达到满额。"
                        .to_string(),
                    facts: vec![
                        metric_fact("运行时长", format_duration(entry.active_seconds)),
                        metric_fact("采样记录", format!("{} 次", entry.sample_count)),
                    ],
                },
                WorkLogScoreDimension {
                    key: "load".to_string(),
                    title: "硬件负载强度".to_string(),
                    score: load_score.round() as u32,
                    max_score: 25,
                    value: format!("CPU {cpu_avg:.0}% / RAM {memory_avg:.0}% / GPU {gpu_avg:.0}%"),
                    explanation: "综合平均负载与 CPU>50%、RAM>70%、GPU>70% 的持续时长。"
                        .to_string(),
                    facts: vec![
                        metric_fact("CPU>50%", format_duration(entry.cpu_over_50_seconds)),
                        metric_fact("RAM>70%", format_duration(entry.memory_over_70_seconds)),
                        metric_fact("GPU>70%", format_duration(entry.gpu_over_70_seconds)),
                    ],
                },
                WorkLogScoreDimension {
                    key: "complexity".to_string(),
                    title: "任务复杂度".to_string(),
                    score: complexity_score.round() as u32,
                    max_score: 20,
                    value: format!("高负载 {:.0}% / IO {:.0}%", high_load_ratio * 100.0, io_avg),
                    explanation: "结合高负载比例、磁盘读写总量、网络上传与访问流量。"
                        .to_string(),
                    facts: vec![
                        metric_fact("磁盘读", format_bytes(entry.disk_read_bytes_total)),
                        metric_fact("磁盘写", format_bytes(entry.disk_write_bytes_total)),
                        metric_fact("上传", format_bytes(entry.network_upload_bytes_total)),
                        metric_fact("访问", format_bytes(entry.network_download_bytes_total)),
                    ],
                },
                WorkLogScoreDimension {
                    key: "stability".to_string(),
                    title: "运行稳定性".to_string(),
                    score: stability_score.round() as u32,
                    max_score: 15,
                    value: format!("压力 {:.0}%", thermal_avg),
                    explanation: "高温持续时间越短，说明高强度工作下系统越稳定。".to_string(),
                    facts: vec![
                        metric_fact("CPU>80C", format_duration(entry.cpu_over_80c_seconds)),
                        metric_fact("GPU>80C", format_duration(entry.gpu_over_80c_seconds)),
                    ],
                },
                WorkLogScoreDimension {
                    key: "continuity".to_string(),
                    title: "连续工作投入".to_string(),
                    score: continuity_score.round() as u32,
                    max_score: 10,
                    value: format!(
                        "点击 {} / 按键 {}",
                        entry.mouse_click_count, entry.keyboard_press_count
                    ),
                    explanation: "结合持续观察窗口、鼠标点击和键盘按键次数，体现实际操作密度。"
                        .to_string(),
                    facts: vec![
                        metric_fact("鼠标点击", format!("{} 次", entry.mouse_click_count)),
                        metric_fact("键盘按键", format!("{} 次", entry.keyboard_press_count)),
                    ],
                },
            ],
        }
    }
}

impl DailyWorkAssessment {
    pub fn from_entry(entry: WorkLogEntry, history: &[WorkLogEntry]) -> Self {
        let report = WorkLogReport::from_entry(entry.clone());
        let features = WorkLogFeatures::from_entry(&entry);
        let day_type = resolve_work_day_type(&features);
        let baseline = build_baseline_comparison(&features, history);
        let workprint = build_workprint(day_type, &features);
        let timeline = build_work_timeline(&entry);
        let mvp_segments = build_mvp_segments(&timeline);
        let rarity = build_work_card_rarity(report.total_score, &features, history, &entry.date);
        let title = build_work_day_title(day_type, history);
        let corecat_commentary =
            build_corecat_commentary(day_type, &features, &baseline, report.total_score, &rarity, &title);
        let (highlights, risks, suggestions) =
            build_assessment_insights(&features, &baseline, day_type);
        let process_insights = build_process_insights(&entry);

        Self {
            date: entry.date,
            day_type,
            day_type_title: work_day_type_title(day_type).to_string(),
            rarity,
            title,
            corecat_commentary,
            workprint,
            baseline,
            timeline,
            mvp_segments,
            highlights,
            risks,
            suggestions,
            process_insights,
            dimensions: assessment_dimensions(report.dimensions),
            score: report.total_score,
            corecat_summary: corecat_daily_summary(day_type, &features),
            badge_ids: work_day_badges(day_type, &features),
        }
    }

    pub fn summary(&self, has_data: bool) -> DailyWorkAssessmentSummary {
        DailyWorkAssessmentSummary {
            date: self.date.clone(),
            day_type: self.day_type,
            day_type_title: self.day_type_title.clone(),
            rarity: self.rarity.clone(),
            title: self.title.clone(),
            workprint: self.workprint.clone(),
            score: self.score,
            corecat_summary: self.corecat_summary.clone(),
            badge_ids: self.badge_ids.clone(),
            has_timeline: !self.timeline.is_empty(),
            has_data,
        }
    }
}

impl DailyWorkAssessmentTrend {
    pub fn from_summaries(summaries: &[DailyWorkAssessmentSummary]) -> Self {
        if summaries.is_empty() {
            return Self {
                sample_days: 0,
                average_score: 0,
                best_date: None,
                best_score: None,
                best_day_type: WorkDayType::Unknown,
                best_day_type_title: work_day_type_title(WorkDayType::Unknown).to_string(),
                dominant_day_type: WorkDayType::Unknown,
                dominant_day_type_title: work_day_type_title(WorkDayType::Unknown).to_string(),
                timeline_days: 0,
                score_delta: 0,
                summary: "CoreCat 还没有足够的历史日报来判断近期节奏。".to_string(),
                insights: vec![insight(
                    "等待历史样本",
                    "保持 CoreCat 常驻几天后，这里会生成近 14 日工作画像趋势。",
                    InsightSeverity::Neutral,
                    None,
                )],
            };
        }

        let sample_days = summaries.len() as u32;
        let score_sum = summaries.iter().map(|summary| summary.score).sum::<u32>();
        let average_score = ((score_sum as f64) / summaries.len() as f64).round() as u32;
        let best_summary = summaries
            .iter()
            .max_by_key(|summary| summary.score)
            .expect("non-empty summaries have a best day");
        let newest_score = summaries.first().map(|summary| summary.score).unwrap_or(0);
        let oldest_score = summaries.last().map(|summary| summary.score).unwrap_or(newest_score);
        let score_delta = newest_score as i32 - oldest_score as i32;
        let timeline_days = summaries
            .iter()
            .filter(|summary| summary.has_timeline)
            .count() as u32;
        let dominant_day_type = dominant_work_day_type(summaries);
        let summary = trend_summary(
            sample_days,
            average_score,
            score_delta,
            dominant_day_type,
            timeline_days,
        );
        let insights = trend_insights(
            summaries,
            average_score,
            score_delta,
            dominant_day_type,
            best_summary,
            timeline_days,
        );

        Self {
            sample_days,
            average_score,
            best_date: Some(best_summary.date.clone()),
            best_score: Some(best_summary.score),
            best_day_type: best_summary.day_type,
            best_day_type_title: best_summary.day_type_title.clone(),
            dominant_day_type,
            dominant_day_type_title: work_day_type_title(dominant_day_type).to_string(),
            timeline_days,
            score_delta,
            summary,
            insights,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WorkLogFeatures {
    active_seconds: u64,
    sample_count: u64,
    cpu_avg: f64,
    gpu_avg: f64,
    memory_avg: f64,
    thermal_avg: f64,
    io_avg: f64,
    high_load_ratio: f64,
    cpu_over_50_ratio: f64,
    memory_over_70_ratio: f64,
    gpu_over_70_ratio: f64,
    cpu_over_80c_ratio: f64,
    gpu_over_80c_ratio: f64,
    disk_total: u64,
    network_total: u64,
    input_total: u64,
}

impl WorkLogFeatures {
    fn from_entry(entry: &WorkLogEntry) -> Self {
        let samples = entry.sample_count.max(1) as f64;

        Self {
            active_seconds: entry.active_seconds,
            sample_count: entry.sample_count,
            cpu_avg: entry.cpu_load_points / samples,
            gpu_avg: entry.gpu_load_points / samples,
            memory_avg: entry.memory_load_points / samples,
            thermal_avg: entry.thermal_pressure_points / samples,
            io_avg: entry.io_activity_points / samples,
            high_load_ratio: duration_ratio(entry.high_load_seconds, entry.active_seconds),
            cpu_over_50_ratio: duration_ratio(entry.cpu_over_50_seconds, entry.active_seconds),
            memory_over_70_ratio: duration_ratio(
                entry.memory_over_70_seconds,
                entry.active_seconds,
            ),
            gpu_over_70_ratio: duration_ratio(entry.gpu_over_70_seconds, entry.active_seconds),
            cpu_over_80c_ratio: duration_ratio(entry.cpu_over_80c_seconds, entry.active_seconds),
            gpu_over_80c_ratio: duration_ratio(entry.gpu_over_80c_seconds, entry.active_seconds),
            disk_total: entry
                .disk_read_bytes_total
                .saturating_add(entry.disk_write_bytes_total),
            network_total: entry
                .network_download_bytes_total
                .saturating_add(entry.network_upload_bytes_total),
            input_total: entry
                .mouse_click_count
                .saturating_add(entry.keyboard_press_count),
        }
    }

    fn active_hours(self) -> f64 {
        self.active_seconds as f64 / 3600.0
    }

    fn average_load(self) -> f64 {
        self.cpu_avg * 0.36 + self.memory_avg * 0.30 + self.gpu_avg * 0.22 + self.io_avg * 0.12
    }

    fn io_total(self) -> u64 {
        self.disk_total.saturating_add(self.network_total)
    }

    fn input_per_hour(self) -> f64 {
        if self.active_seconds == 0 {
            return 0.0;
        }

        self.input_total as f64 / self.active_hours().max(0.1)
    }

    fn thermal_warning_ratio(self) -> f64 {
        self.cpu_over_80c_ratio.max(self.gpu_over_80c_ratio)
    }
}

fn resolve_work_day_type(features: &WorkLogFeatures) -> WorkDayType {
    if features.sample_count == 0 || features.active_seconds < 60 {
        return WorkDayType::Unknown;
    }

    let input_per_hour = features.input_per_hour();
    let io_total = features.io_total();
    let pressure_risk = features.thermal_avg >= 42.0
        || features.thermal_warning_ratio() >= 0.03
        || features.memory_over_70_ratio >= 0.38
        || features.gpu_over_70_ratio >= 0.30;
    let build_like = (features.cpu_over_50_ratio >= 0.22 || features.cpu_avg >= 42.0)
        && (features.disk_total >= 384 * 1024 * 1024 || features.io_avg >= 28.0);
    let archive_like = io_total >= 900 * 1024 * 1024
        && features.io_avg >= 26.0
        && features.cpu_avg < 48.0
        && features.gpu_avg < 45.0;
    let focus_like = features.active_hours() >= 2.0
        && input_per_hour >= 850.0
        && features.high_load_ratio <= 0.52
        && features.thermal_warning_ratio() < 0.02;
    let fragmented_like = features.active_hours() >= 1.2
        && input_per_hour >= 1400.0
        && features.high_load_ratio >= 0.18
        && features.cpu_over_50_ratio < 0.18
        && features.io_avg < 35.0;

    if pressure_risk && (features.high_load_ratio >= 0.16 || features.average_load() >= 46.0) {
        WorkDayType::PressureRepair
    } else if build_like {
        WorkDayType::BuildBurst
    } else if archive_like {
        WorkDayType::ArchiveFlow
    } else if focus_like {
        WorkDayType::DeepFocus
    } else if fragmented_like {
        WorkDayType::FragmentedSwitching
    } else if features.active_hours() >= 1.0 && features.thermal_warning_ratio() < 0.02 {
        WorkDayType::StableMaintenance
    } else {
        WorkDayType::LowLoadCompanion
    }
}

fn build_baseline_comparison(
    features: &WorkLogFeatures,
    history: &[WorkLogEntry],
) -> BaselineComparison {
    let history_features: Vec<WorkLogFeatures> = history
        .iter()
        .take(7)
        .map(WorkLogFeatures::from_entry)
        .filter(|item| item.sample_count > 0 && item.active_seconds > 0)
        .collect();
    let sample_days = history_features.len() as u32;

    if history_features.is_empty() {
        return BaselineComparison {
            sample_days: 0,
            active_seconds_delta_ratio: 0.0,
            load_delta_ratio: 0.0,
            io_delta_ratio: 0.0,
            thermal_delta_ratio: 0.0,
            input_delta_ratio: 0.0,
            metrics: baseline_metrics(features, None),
            summary: "CoreCat 正在积累你的个人工作基线，连续使用几天后会给出更贴近你习惯的对比。"
                .to_string(),
        };
    }

    let average = |value: fn(WorkLogFeatures) -> f64| {
        history_features.iter().map(|item| value(*item)).sum::<f64>()
            / history_features.len() as f64
    };
    let active_avg = average(|item| item.active_seconds as f64);
    let load_avg = average(WorkLogFeatures::average_load);
    let io_avg = average(|item| item.io_avg);
    let thermal_avg = average(|item| item.thermal_avg);
    let input_avg = average(WorkLogFeatures::input_per_hour);

    let active_delta = ratio_delta(features.active_seconds as f64, active_avg);
    let load_delta = ratio_delta(features.average_load(), load_avg);
    let io_delta = ratio_delta(features.io_avg, io_avg);
    let thermal_delta = ratio_delta(features.thermal_avg, thermal_avg);
    let input_delta = ratio_delta(features.input_per_hour(), input_avg);

    BaselineComparison {
        sample_days,
        active_seconds_delta_ratio: active_delta,
        load_delta_ratio: load_delta,
        io_delta_ratio: io_delta,
        thermal_delta_ratio: thermal_delta,
        input_delta_ratio: input_delta,
        metrics: baseline_metrics(
            features,
            Some(BaselineAverages {
                active_seconds: active_avg,
                load: load_avg,
                io: io_avg,
                thermal: thermal_avg,
                input_per_hour: input_avg,
            }),
        ),
        summary: baseline_summary(sample_days, active_delta, load_delta, io_delta, thermal_delta, input_delta),
    }
}

#[derive(Debug, Clone, Copy)]
struct BaselineAverages {
    active_seconds: f64,
    load: f64,
    io: f64,
    thermal: f64,
    input_per_hour: f64,
}

fn baseline_metrics(
    features: &WorkLogFeatures,
    baseline: Option<BaselineAverages>,
) -> Vec<BaselineMetric> {
    let items = [
        (
            "active",
            "陪伴",
            features.active_seconds as f64,
            baseline.map(|item| item.active_seconds),
            format_duration(features.active_seconds),
            baseline.map(|item| format_duration(item.active_seconds.round().max(0.0) as u64)),
            false,
        ),
        (
            "load",
            "火力",
            features.average_load(),
            baseline.map(|item| item.load),
            format!("{:.0}%", features.average_load()),
            baseline.map(|item| format!("{:.0}%", item.load)),
            false,
        ),
        (
            "io",
            "IO",
            features.io_avg,
            baseline.map(|item| item.io),
            format!("{:.0}%", features.io_avg),
            baseline.map(|item| format!("{:.0}%", item.io)),
            false,
        ),
        (
            "thermal",
            "温度",
            features.thermal_avg,
            baseline.map(|item| item.thermal),
            format!("{:.0}%", features.thermal_avg),
            baseline.map(|item| format!("{:.0}%", item.thermal)),
            true,
        ),
        (
            "input",
            "输入",
            features.input_per_hour(),
            baseline.map(|item| item.input_per_hour),
            format!("{:.0}/h", features.input_per_hour()),
            baseline.map(|item| format!("{:.0}/h", item.input_per_hour)),
            false,
        ),
    ];

    items
        .into_iter()
        .map(
            |(key, label, current, baseline_value, current_text, baseline_text, inverse)| {
                let delta = baseline_value.map_or(0.0, |value| ratio_delta(current, value));
                let tone = if baseline_value.is_none() || delta.abs() < 0.12 {
                    InsightSeverity::Neutral
                } else if (inverse && delta < 0.0) || (!inverse && delta > 0.0) {
                    InsightSeverity::Positive
                } else {
                    InsightSeverity::Warning
                };

                BaselineMetric {
                    key: key.to_string(),
                    label: label.to_string(),
                    current_value: current_text,
                    baseline_value: baseline_text.unwrap_or_else(|| "--".to_string()),
                    delta_ratio: delta,
                    tone,
                }
            },
        )
        .collect()
}

fn build_workprint(day_type: WorkDayType, features: &WorkLogFeatures) -> WorkprintSummary {
    let load_shape = (features.average_load() / 100.0).clamp(0.0, 1.0);
    let input_rhythm = (features.input_per_hour() / 5200.0).clamp(0.0, 1.0);
    let io_intensity = (features.io_avg / 100.0).clamp(0.0, 1.0);
    let thermal_pressure = (features.thermal_avg / 100.0).clamp(0.0, 1.0);
    let continuity = ((features.active_hours() / 6.0) * 0.55 + input_rhythm * 0.45)
        .clamp(0.0, 1.0);

    let label = match day_type {
        WorkDayType::DeepFocus => "长时稳定型",
        WorkDayType::BuildBurst => "构建峰值型",
        WorkDayType::ArchiveFlow => "资料流动型",
        WorkDayType::PressureRepair => "高压承载型",
        WorkDayType::StableMaintenance => "平稳维护型",
        WorkDayType::FragmentedSwitching => "多段切换型",
        WorkDayType::LowLoadCompanion => "低负载陪伴型",
        WorkDayType::Unknown => "数据积累中",
    };
    let description = match day_type {
        WorkDayType::Unknown => {
            "今天的数据还不足以形成稳定指纹，CoreCat 会继续观察后续节奏。".to_string()
        }
        _ => format!(
            "今日 Workprint 呈现为{label}：负载 {:.0}%，输入节奏 {:.0}%，IO 强度 {:.0}%，热压力 {:.0}%。",
            load_shape * 100.0,
            input_rhythm * 100.0,
            io_intensity * 100.0,
            thermal_pressure * 100.0
        ),
    };

    WorkprintSummary {
        label: label.to_string(),
        description,
        pixel_grid: build_workprint_grid([
            load_shape,
            input_rhythm,
            io_intensity,
            thermal_pressure,
            continuity,
        ]),
        width: 8,
        height: 8,
        load_shape,
        input_rhythm,
        io_intensity,
        thermal_pressure,
        continuity,
    }
}

fn build_work_timeline(entry: &WorkLogEntry) -> Vec<WorkTimelineSegment> {
    let mut slices = entry
        .time_slices
        .iter()
        .filter(|slice| {
            slice.sample_count > 0 || slice.mouse_click_count > 0 || slice.keyboard_press_count > 0
        })
        .collect::<Vec<_>>();
    slices.sort_by_key(|slice| slice.start_timestamp);

    let mut segments: Vec<WorkTimelineSegment> = Vec::new();

    for slice in slices {
        let features = WorkTimeSliceFeatures::from_slice(slice);
        let kind = resolve_timeline_segment_kind(&features);
        let segment = WorkTimelineSegment {
            start_time: format_time_slice_label(slice.start_timestamp),
            end_time: format_time_slice_label(slice.end_timestamp),
            kind,
            intensity: features.intensity(),
            label: timeline_segment_label(kind).to_string(),
            description: timeline_segment_description(kind, &features),
        };

        if let Some(last) = segments.last_mut() {
            if last.kind == segment.kind && last.end_time == segment.start_time {
                last.end_time = segment.end_time;
                last.intensity = last.intensity.max(segment.intensity);
                continue;
            }
        }

        segments.push(segment);
    }

    segments
}

fn build_mvp_segments(timeline: &[WorkTimelineSegment]) -> Vec<WorkTimelineSegment> {
    let mut candidates = timeline
        .iter()
        .filter(|segment| segment.kind != WorkTimelineSegmentKind::IdleCompanion)
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .intensity
            .partial_cmp(&left.intensity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(3);
    candidates.sort_by(|left, right| left.start_time.cmp(&right.start_time));

    candidates
}

fn build_work_card_rarity(
    score: u32,
    features: &WorkLogFeatures,
    history: &[WorkLogEntry],
    date: &str,
) -> WorkCardRarity {
    let active_streak_days = active_streak_days(date, features, history);
    let load_index =
        (features.average_load() * 0.65 + features.high_load_ratio * 100.0 * 0.35).clamp(0.0, 100.0);
    let stability_index = (100.0
        - features.thermal_avg * 0.55
        - features.thermal_warning_ratio() * 160.0
        - features.memory_over_70_ratio * 45.0)
        .clamp(0.0, 100.0);
    let rarity_score = (score as f64 * 0.50
        + (active_streak_days.min(7) as f64 / 7.0) * 18.0
        + (load_index / 100.0) * 17.0
        + (stability_index / 100.0) * 15.0)
        .round()
        .clamp(0.0, 100.0) as u32;
    let tier = if rarity_score >= 85 {
        "SS"
    } else if rarity_score >= 72 {
        "S"
    } else if rarity_score >= 58 {
        "A"
    } else if rarity_score >= 40 {
        "B"
    } else {
        "C"
    };

    WorkCardRarity {
        tier: tier.to_string(),
        label: format!("{tier} 级工况卡"),
        score: rarity_score,
        reason: format!(
            "画像分 {score}，连续活跃 {active_streak_days} 天，负载指数 {:.0}，稳定指数 {:.0}",
            load_index, stability_index
        ),
    }
}

fn active_streak_days(date: &str, features: &WorkLogFeatures, history: &[WorkLogEntry]) -> u32 {
    if !features_has_signal(features) {
        return 0;
    }

    let Ok(mut expected_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|date| date - Duration::days(1))
    else {
        return 1;
    };
    let mut streak = 1;

    for entry in history {
        let Ok(entry_date) = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d") else {
            continue;
        };
        if entry_date > expected_date {
            continue;
        }
        if entry_date < expected_date {
            break;
        }

        let entry_features = WorkLogFeatures::from_entry(entry);
        if features_has_signal(&entry_features) {
            streak += 1;
            expected_date -= Duration::days(1);
        } else {
            break;
        }
    }

    streak
}

fn features_has_signal(features: &WorkLogFeatures) -> bool {
    features.sample_count > 0 || features.active_seconds > 0 || features.input_total > 0
}

fn build_work_day_title(day_type: WorkDayType, history: &[WorkLogEntry]) -> WorkDayTitle {
    let progress = 1
        + history
            .iter()
            .filter(|entry| resolve_work_day_type(&WorkLogFeatures::from_entry(entry)) == day_type)
            .count() as u32;
    let thresholds = [1_u32, 3, 7, 14, 30];
    let level = thresholds
        .iter()
        .filter(|threshold| progress >= **threshold)
        .count()
        .max(1) as u32;
    let next_level_at = thresholds
        .iter()
        .copied()
        .find(|threshold| *threshold > progress);
    let titles = work_day_title_pool(day_type);
    let title = titles[(level.saturating_sub(1) as usize).min(titles.len() - 1)];

    WorkDayTitle {
        family: work_day_title_family(day_type).to_string(),
        title: title.to_string(),
        level,
        progress,
        next_level_at,
    }
}

fn work_day_title_family(day_type: WorkDayType) -> &'static str {
    match day_type {
        WorkDayType::DeepFocus => "focus",
        WorkDayType::BuildBurst => "build",
        WorkDayType::ArchiveFlow => "archive",
        WorkDayType::PressureRepair => "pressure",
        WorkDayType::StableMaintenance => "steady",
        WorkDayType::FragmentedSwitching => "switch",
        WorkDayType::LowLoadCompanion => "quiet",
        WorkDayType::Unknown => "observe",
    }
}

fn work_day_title_pool(day_type: WorkDayType) -> [&'static str; 5] {
    match day_type {
        WorkDayType::DeepFocus => [
            "专注学徒",
            "深潜构筑师",
            "静流策士",
            "沉浸工程师",
            "深海领航员",
        ],
        WorkDayType::BuildBurst => [
            "火力装配员",
            "风暴构筑师",
            "熔炉调度官",
            "构建指挥官",
            "星火锻造师",
        ],
        WorkDayType::ArchiveFlow => [
            "资料整理员",
            "风暴归档者",
            "流转档案师",
            "数据航道长",
            "星库典藏师",
        ],
        WorkDayType::PressureRepair => [
            "压力观察员",
            "高压修复师",
            "热浪调停者",
            "故障压制官",
            "临界守望者",
        ],
        WorkDayType::StableMaintenance => [
            "平稳守护者",
            "冷静维护员",
            "秩序巡检员",
            "静态运维官",
            "长线守护者",
        ],
        WorkDayType::FragmentedSwitching => [
            "多线记录员",
            "切换游侠",
            "碎片调度师",
            "多任务编排官",
            "闪转指挥家",
        ],
        WorkDayType::LowLoadCompanion => [
            "安静陪伴员",
            "轻载看护者",
            "低频巡游者",
            "静默整理师",
            "休整守门人",
        ],
        WorkDayType::Unknown => [
            "观察记录员",
            "数据收集员",
            "初始观测者",
            "样本整理师",
            "未知星图员",
        ],
    }
}

fn build_corecat_commentary(
    day_type: WorkDayType,
    features: &WorkLogFeatures,
    baseline: &BaselineComparison,
    score: u32,
    rarity: &WorkCardRarity,
    title: &WorkDayTitle,
) -> CoreCatCommentary {
    let tone = if features.thermal_warning_ratio() >= 0.03 || day_type == WorkDayType::PressureRepair
    {
        CoreCatCommentTone::Warning
    } else if score >= 82 || rarity.tier == "SS" || rarity.tier == "S" {
        CoreCatCommentTone::Celebration
    } else if day_type == WorkDayType::Unknown || features.active_seconds < 900 {
        CoreCatCommentTone::Tease
    } else {
        CoreCatCommentTone::Encouragement
    };

    let diagnostic = work_condition_diagnostic(features, baseline);
    let (comment_title, body) = match tone {
        CoreCatCommentTone::Celebration => (
            "CoreCat 战报：这张卡有收藏价值",
            format!(
                "喵，今天的{}拿到 {}，{} 已经升到 Lv.{}。{}",
                work_day_type_title(day_type),
                rarity.label,
                title.title,
                title.level,
                diagnostic
            ),
        ),
        CoreCatCommentTone::Warning => (
            "CoreCat 警报：机器有点烫爪",
            format!(
                "今天压力信号偏高，热压力约 {:.0}%。{}",
                features.thermal_avg, diagnostic
            ),
        ),
        CoreCatCommentTone::Tease => (
            "CoreCat 吐槽：这张卡还在孵化",
            format!(
                "今天样本还不够厚，CoreCat 只抓到 {} 的轮廓。{}",
                title.title, diagnostic
            ),
        ),
        CoreCatCommentTone::Encouragement => (
            "CoreCat 点评：节奏已经成型",
            format!(
                "今天的{}比较清楚，负载 {:.0}、输入节奏 {:.0}/h。{}",
                work_day_type_title(day_type),
                features.average_load(),
                features.input_per_hour(),
                diagnostic
            ),
        ),
    };

    CoreCatCommentary {
        tone,
        title: comment_title.to_string(),
        body,
    }
}

fn work_condition_diagnostic(
    features: &WorkLogFeatures,
    baseline: &BaselineComparison,
) -> String {
    if features.sample_count == 0 || features.active_seconds < 60 {
        return "再多陪伴一会儿，明天的卡面会更像样。".to_string();
    }

    if features.thermal_warning_ratio() >= 0.03 || features.thermal_avg >= 52.0 {
        return format!(
            "工况解释：主要异常来自热压力，温度压力指数约 {:.0}%，高负载前先留散热余量。",
            features.thermal_avg
        );
    }

    if features.memory_over_70_ratio >= 0.35 || features.memory_avg >= 78.0 {
        return format!(
            "工况解释：主要瓶颈像是内存拥挤，RAM 均值约 {:.0}%，多任务切换前适合关掉闲置大应用。",
            features.memory_avg
        );
    }

    if features.gpu_over_70_ratio >= 0.30 {
        return format!(
            "工况解释：GPU 高占用持续约 {}，更像渲染、视频或图形任务在拉高压力。",
            format_duration((features.gpu_over_70_ratio * features.active_seconds as f64) as u64)
        );
    }

    if features.io_avg >= 45.0 || (baseline.sample_days > 0 && baseline.io_delta_ratio >= 0.35) {
        if baseline.sample_days > 0 {
            return format!(
                "工况解释：磁盘和网络流动偏活跃，资料流动强度比你的近期基线{}。",
                format_delta(baseline.io_delta_ratio)
            );
        }

        return "工况解释：磁盘和网络流动偏活跃，更像同步、下载、构建或归档任务。"
            .to_string();
    }

    if baseline.sample_days > 0 && baseline.load_delta_ratio >= 0.28 {
        return format!(
            "工况解释：今天整体火力比你的近期基线{}，更像构建、批处理或长任务日。",
            format_delta(baseline.load_delta_ratio)
        );
    }

    if baseline.sample_days > 0 && baseline.input_delta_ratio >= 0.28 {
        return format!(
            "工况解释：输入节奏比你的近期基线{}，更像连续操作或高频切换。",
            format_delta(baseline.input_delta_ratio)
        );
    }

    "工况解释：没有明显异常，今天更接近你的常规运行区间。".to_string()
}

#[derive(Debug, Clone, Copy)]
struct WorkTimeSliceFeatures {
    active_seconds: u64,
    sample_count: u64,
    cpu_avg: f64,
    gpu_avg: f64,
    memory_avg: f64,
    thermal_avg: f64,
    io_avg: f64,
    disk_total: u64,
    network_total: u64,
    input_total: u64,
}

impl WorkTimeSliceFeatures {
    fn from_slice(slice: &WorkLogTimeSlice) -> Self {
        let samples = slice.sample_count.max(1) as f64;

        Self {
            active_seconds: slice.active_seconds,
            sample_count: slice.sample_count,
            cpu_avg: slice.cpu_load_points / samples,
            gpu_avg: slice.gpu_load_points / samples,
            memory_avg: slice.memory_load_points / samples,
            thermal_avg: slice.thermal_pressure_points / samples,
            io_avg: slice.io_activity_points / samples,
            disk_total: slice
                .disk_read_bytes_total
                .saturating_add(slice.disk_write_bytes_total),
            network_total: slice
                .network_download_bytes_total
                .saturating_add(slice.network_upload_bytes_total),
            input_total: slice
                .mouse_click_count
                .saturating_add(slice.keyboard_press_count),
        }
    }

    fn average_load(self) -> f64 {
        self.cpu_avg * 0.38 + self.memory_avg * 0.28 + self.gpu_avg * 0.22 + self.io_avg * 0.12
    }

    fn io_total(self) -> u64 {
        self.disk_total.saturating_add(self.network_total)
    }

    fn input_per_hour(self) -> f64 {
        if self.active_seconds == 0 {
            return self.input_total as f64 * 4.0;
        }

        self.input_total as f64 / (self.active_seconds as f64 / 3600.0).max(0.1)
    }

    fn intensity(self) -> f64 {
        let input_score = (self.input_per_hour() / 5200.0 * 100.0).clamp(0.0, 100.0);
        self.average_load()
            .max(self.io_avg)
            .max(self.thermal_avg)
            .max(input_score)
            .clamp(0.0, 100.0)
            / 100.0
    }
}

fn resolve_timeline_segment_kind(features: &WorkTimeSliceFeatures) -> WorkTimelineSegmentKind {
    if features.sample_count == 0 && features.input_total == 0 {
        return WorkTimelineSegmentKind::IdleCompanion;
    }

    let load = features.average_load();
    let io_total = features.io_total();
    let pressure_repair =
        (features.thermal_avg >= 55.0 && load >= 45.0) || (features.memory_avg >= 82.0 && load >= 45.0);

    if pressure_repair {
        WorkTimelineSegmentKind::PressureRepair
    } else if features.thermal_avg >= 55.0 {
        WorkTimelineSegmentKind::TemperatureWarning
    } else if features.memory_avg >= 78.0 {
        WorkTimelineSegmentKind::MemoryCrowded
    } else if features.cpu_avg >= 58.0 && features.disk_total >= 32 * 1024 * 1024 {
        WorkTimelineSegmentKind::BuildPeak
    } else if features.io_avg >= 42.0 || io_total >= 128 * 1024 * 1024 {
        WorkTimelineSegmentKind::ArchiveFlow
    } else if features.input_per_hour() >= 1000.0 && load <= 62.0 {
        WorkTimelineSegmentKind::DeepFocus
    } else if features.active_seconds > 0 || features.input_total > 0 {
        WorkTimelineSegmentKind::SteadyProgress
    } else {
        WorkTimelineSegmentKind::IdleCompanion
    }
}

fn timeline_segment_label(kind: WorkTimelineSegmentKind) -> &'static str {
    match kind {
        WorkTimelineSegmentKind::IdleCompanion => "空闲陪伴",
        WorkTimelineSegmentKind::SteadyProgress => "稳定推进",
        WorkTimelineSegmentKind::DeepFocus => "深度工作",
        WorkTimelineSegmentKind::BuildPeak => "构建高峰",
        WorkTimelineSegmentKind::ArchiveFlow => "资料归档",
        WorkTimelineSegmentKind::MemoryCrowded => "内存拥挤",
        WorkTimelineSegmentKind::TemperatureWarning => "温度预警",
        WorkTimelineSegmentKind::PressureRepair => "高压抢修",
    }
}

fn timeline_segment_description(
    kind: WorkTimelineSegmentKind,
    features: &WorkTimeSliceFeatures,
) -> String {
    match kind {
        WorkTimelineSegmentKind::IdleCompanion => {
            "这一段系统负载与输入活动都较低，CoreCat 基本处于安静陪伴状态。".to_string()
        }
        WorkTimelineSegmentKind::SteadyProgress => format!(
            "这一段工况比较平稳，综合负载约 {:.0}%，适合归类为常规推进窗口。",
            features.average_load()
        ),
        WorkTimelineSegmentKind::DeepFocus => format!(
            "这一段键鼠节奏较连续，输入密度约 {:.0}/h，CoreCat 标记为深度工作片段。",
            features.input_per_hour()
        ),
        WorkTimelineSegmentKind::BuildPeak => format!(
            "CPU 与磁盘活动同时抬升，CPU 均值约 {:.0}%，更像构建、打包或批处理高峰。",
            features.cpu_avg
        ),
        WorkTimelineSegmentKind::ArchiveFlow => format!(
            "磁盘与网络流动明显，本段数据流量约 {}，CoreCat 将它归档为资料流动片段。",
            format_bytes(features.io_total())
        ),
        WorkTimelineSegmentKind::MemoryCrowded => format!(
            "内存压力偏高，平均占用约 {:.0}%，后续多任务切换可能会变慢。",
            features.memory_avg
        ),
        WorkTimelineSegmentKind::TemperatureWarning => format!(
            "热压力偏高，温度压力指数约 {:.0}%，这段需要留意散热。",
            features.thermal_avg
        ),
        WorkTimelineSegmentKind::PressureRepair => format!(
            "高负载与压力信号同时出现，综合负载约 {:.0}%，CoreCat 标记为高压抢修片段。",
            features.average_load()
        ),
    }
}

fn build_assessment_insights(
    features: &WorkLogFeatures,
    baseline: &BaselineComparison,
    day_type: WorkDayType,
) -> (
    Vec<AssessmentInsight>,
    Vec<AssessmentInsight>,
    Vec<AssessmentInsight>,
) {
    let mut highlights = Vec::new();
    let mut risks = Vec::new();
    let mut suggestions = Vec::new();

    if features.active_hours() >= 2.0 {
        highlights.push(insight(
            "陪伴时长稳定",
            format!(
                "CoreCat 今天陪你运行了 {}，已经形成一段可分析的工作节奏。",
                format_duration(features.active_seconds)
            ),
            InsightSeverity::Positive,
            Some(format_duration(features.active_seconds)),
        ));
    }

    if baseline.sample_days > 0 && baseline.io_delta_ratio >= 0.35 {
        highlights.push(insight(
            "资料流动更活跃",
            "今天的磁盘与网络活动明显高于你的近期基线，像是有较多同步、下载、构建或归档任务。",
            InsightSeverity::Positive,
            Some(format_delta(baseline.io_delta_ratio)),
        ));
    }

    if baseline.sample_days > 0 && baseline.thermal_delta_ratio <= -0.20 {
        highlights.push(insight(
            "机器运行更冷静",
            "今天的热压力低于近期平均，说明机器在当前任务下保持了更稳定的散热状态。",
            InsightSeverity::Positive,
            Some(format_delta(baseline.thermal_delta_ratio)),
        ));
    }

    if features.memory_over_70_ratio >= 0.35 {
        risks.push(insight(
            "内存仓库偏拥挤",
            "内存高占用持续时间较长，后续如果继续多任务，可能会影响切换和响应速度。",
            InsightSeverity::Warning,
            Some(format!("{:.0}%", features.memory_over_70_ratio * 100.0)),
        ));
        suggestions.push(insight(
            "明日留意后台任务",
            "如果明天继续保持类似工作强度，可以在开始前关闭长期闲置的大型应用，减少内存拥挤片段。",
            InsightSeverity::Neutral,
            None,
        ));
    }

    if features.thermal_warning_ratio() >= 0.03 || features.thermal_avg >= 48.0 {
        risks.push(insight(
            "热压力值得关注",
            "今天出现了较明显的温度压力。CoreCat 建议后续高负载任务时留意散热和风道状态。",
            InsightSeverity::Warning,
            Some(format!("{:.0}%", features.thermal_avg)),
        ));
        suggestions.push(insight(
            "高负载前留出散热余量",
            "如果明天需要构建、渲染或长时间运行高负载任务，建议提前检查散热环境。",
            InsightSeverity::Neutral,
            None,
        ));
    }

    if features.input_per_hour() >= 1200.0 && day_type != WorkDayType::PressureRepair {
        highlights.push(insight(
            "输入节奏清晰",
            "今天键鼠活动密度较高，CoreCat 判断你有一段比较明确的实际操作窗口。",
            InsightSeverity::Positive,
            Some(format!("{:.0}/h", features.input_per_hour())),
        ));
    }

    if suggestions.is_empty() {
        suggestions.push(match day_type {
            WorkDayType::BuildBurst => insight(
                "记录构建高峰",
                "如果明天继续做构建或批处理任务，可以把工况报告当作机器负载变化的复盘线索。",
                InsightSeverity::Neutral,
                None,
            ),
            WorkDayType::ArchiveFlow => insight(
                "关注传输完成度",
                "资料流动较多的工作日适合在收尾时检查同步、下载和归档是否完成。",
                InsightSeverity::Neutral,
                None,
            ),
            _ => insight(
                "保持当前节奏",
                "今天的工况记录已经形成可回看的工作画像，明天继续让 CoreCat 安静观察即可。",
                InsightSeverity::Neutral,
                None,
            ),
        });
    }

    if highlights.is_empty() {
        highlights.push(insight(
            "CoreCat 已记录今日节奏",
            "今天的数据虽然不极端，但已经足够成为后续个人基线的一部分。",
            InsightSeverity::Positive,
            None,
        ));
    }

    if risks.is_empty() {
        risks.push(insight(
            "暂无明显隐患",
            "今天没有观察到突出的高温或内存拥挤风险，整体工况比较平顺。",
            InsightSeverity::Neutral,
            None,
        ));
    }

    highlights.truncate(3);
    risks.truncate(3);
    suggestions.truncate(3);

    (highlights, risks, suggestions)
}

fn build_process_insights(entry: &WorkLogEntry) -> Vec<ProcessUsageInsight> {
    let duration_cap = process_duration_cap(entry);
    let sample_count_cap = entry.sample_count.max(1);
    let mut insights = entry
        .process_usage
        .values()
        .filter(|usage| usage.sample_count > 0 && usage.observed_seconds > 0)
        .map(|usage| {
            let observed_seconds = usage.observed_seconds.min(duration_cap).max(1);
            let active_seconds = usage.active_seconds.min(observed_seconds);
            let sample_count = usage.sample_count.min(sample_count_cap).max(1);
            let active_sample_count = usage.active_sample_count.min(sample_count);
            let cpu_pressure_percent =
                usage.cpu_pressure_points / usage.observed_seconds.max(1) as f64;
            let average_memory_bytes = usage.memory_bytes_points / usage.observed_seconds.max(1);
            let disk_total = usage
                .disk_read_bytes_total
                .saturating_add(usage.disk_write_bytes_total);
            let severity = if cpu_pressure_percent >= 55.0
                || usage.memory_bytes_peak >= 2 * 1024 * 1024 * 1024
                || disk_total >= 2 * 1024 * 1024 * 1024
            {
                InsightSeverity::Warning
            } else if active_seconds >= 30 * 60 || observed_seconds >= 2 * 3600 {
                InsightSeverity::Positive
            } else {
                InsightSeverity::Neutral
            };
            let rank_label = process_rank_label(
                usage,
                observed_seconds,
                sample_count,
                active_sample_count,
            );
            let summary = format!(
                "{}：驻留 {}，活跃 {}，CPU 压力约 {:.0}%，内存峰值 {}。",
                rank_label,
                format_duration(observed_seconds),
                format_duration(active_seconds),
                cpu_pressure_percent,
                format_bytes(usage.memory_bytes_peak)
            );

            ProcessUsageInsight {
                name: usage.name.clone(),
                observed_seconds,
                active_seconds,
                sample_count,
                active_sample_count,
                cpu_pressure_percent,
                average_memory_bytes,
                memory_bytes_peak: usage.memory_bytes_peak,
                disk_read_bytes_total: usage.disk_read_bytes_total,
                disk_write_bytes_total: usage.disk_write_bytes_total,
                rank_label,
                summary,
                severity,
            }
        })
        .collect::<Vec<_>>();

    insights.sort_by(|left, right| {
        process_insight_score(right)
            .total_cmp(&process_insight_score(left))
            .then_with(|| right.observed_seconds.cmp(&left.observed_seconds))
    });
    insights.truncate(8);
    insights
}

fn process_duration_cap(entry: &WorkLogEntry) -> u64 {
    let active_day_seconds = entry.active_seconds.min(WORK_LOG_DAY_SECONDS);
    if active_day_seconds > 0 {
        active_day_seconds
    } else {
        WORK_LOG_DAY_SECONDS
    }
}

fn process_insight_score(insight: &ProcessUsageInsight) -> f64 {
    let disk_total = insight
        .disk_read_bytes_total
        .saturating_add(insight.disk_write_bytes_total) as f64;

    insight.observed_seconds as f64 / 60.0
        + insight.active_seconds as f64 / 30.0
        + insight.active_sample_count as f64 * 0.8
        + insight.cpu_pressure_percent * 2.0
        + insight.memory_bytes_peak as f64 / (256.0 * 1024.0 * 1024.0)
        + disk_total / (128.0 * 1024.0 * 1024.0)
}

fn process_rank_label(
    usage: &WorkLogProcessUsage,
    observed_seconds: u64,
    sample_count: u64,
    active_sample_count: u64,
) -> String {
    let disk_total = usage
        .disk_read_bytes_total
        .saturating_add(usage.disk_write_bytes_total);
    let cpu_pressure = usage.cpu_pressure_points / usage.observed_seconds.max(1) as f64;

    if observed_seconds >= 2 * 3600 {
        "长驻后台".to_string()
    } else if active_sample_count >= sample_count.saturating_div(3).max(3) {
        "高频活跃".to_string()
    } else if cpu_pressure >= 35.0 {
        "CPU 压力源".to_string()
    } else if usage.memory_bytes_peak >= 1024 * 1024 * 1024 {
        "内存常驻".to_string()
    } else if disk_total >= 512 * 1024 * 1024 {
        "磁盘流动".to_string()
    } else {
        "后台观察".to_string()
    }
}

fn assessment_dimensions(mut dimensions: Vec<WorkLogScoreDimension>) -> Vec<WorkLogScoreDimension> {
    for dimension in &mut dimensions {
        dimension.title = match dimension.key.as_str() {
            "duration" => "陪伴时长".to_string(),
            "load" => "工坊火力".to_string(),
            "complexity" => "蓝图复杂度".to_string(),
            "stability" => "机器健康".to_string(),
            "continuity" => "专注节奏".to_string(),
            _ => dimension.title.clone(),
        };
    }

    dimensions
}

fn corecat_daily_summary(day_type: WorkDayType, features: &WorkLogFeatures) -> String {
    match day_type {
        WorkDayType::DeepFocus => {
            "CoreCat 观察到你今天有一段稳定连续的工作节奏，负载和输入都比较平顺，像是一次扎实推进的深度工作日。".to_string()
        }
        WorkDayType::BuildBurst => {
            "今天工坊火力集中，CPU 与磁盘活动出现明显峰值。CoreCat 判断这更像一个编译、打包或批处理较多的工作日。".to_string()
        }
        WorkDayType::ArchiveFlow => {
            "今天的数据传输和磁盘归档更活跃，CoreCat 在硬盘柜旁忙了一阵，整体更像资料整理或同步归档的一天。".to_string()
        }
        WorkDayType::PressureRepair => {
            "今天机器承压明显，高负载、温度或内存拥挤都有出现。CoreCat 已把它记录为需要留意的高压工况日。".to_string()
        }
        WorkDayType::StableMaintenance => format!(
            "今天 CoreCat 陪你稳定运行了 {}，系统状态整体平顺，是一次轻量但可回看的维护型工作日。",
            format_duration(features.active_seconds)
        ),
        WorkDayType::FragmentedSwitching => {
            "今天的活动呈现多段切换特征，CoreCat 看到不少短促峰值，像是任务来回切换比较频繁的一天。".to_string()
        }
        WorkDayType::LowLoadCompanion => {
            "今天 CoreCat 更多是在安静陪伴，系统负载和操作节奏都偏轻。这样的日子也适合整理环境、阅读资料或让机器休息。".to_string()
        }
        WorkDayType::Unknown => {
            "今天的数据还不够完整，CoreCat 会继续观察你的工作节奏，稍后生成更可靠的每日画像。".to_string()
        }
    }
}

fn work_day_type_title(day_type: WorkDayType) -> &'static str {
    match day_type {
        WorkDayType::DeepFocus => "深度专注日",
        WorkDayType::BuildBurst => "编译构建日",
        WorkDayType::ArchiveFlow => "资料归档日",
        WorkDayType::PressureRepair => "高压抢修日",
        WorkDayType::StableMaintenance => "平稳维护日",
        WorkDayType::FragmentedSwitching => "碎片切换日",
        WorkDayType::LowLoadCompanion => "低负载陪伴日",
        WorkDayType::Unknown => "数据积累中",
    }
}

fn dominant_work_day_type(summaries: &[DailyWorkAssessmentSummary]) -> WorkDayType {
    let mut counts: HashMap<WorkDayType, u32> = HashMap::new();
    for summary in summaries {
        *counts.entry(summary.day_type).or_insert(0) += 1;
    }

    let mut best_type = WorkDayType::Unknown;
    let mut best_count = 0;
    for summary in summaries {
        let count = counts.get(&summary.day_type).copied().unwrap_or(0);
        if count > best_count {
            best_type = summary.day_type;
            best_count = count;
        }
    }

    best_type
}

fn work_day_type_count(
    summaries: &[DailyWorkAssessmentSummary],
    day_type: WorkDayType,
) -> u32 {
    summaries
        .iter()
        .filter(|summary| summary.day_type == day_type)
        .count() as u32
}

fn trend_summary(
    sample_days: u32,
    average_score: u32,
    score_delta: i32,
    dominant_day_type: WorkDayType,
    timeline_days: u32,
) -> String {
    if sample_days == 1 {
        return format!(
            "CoreCat 已保存 1 天工作画像：{}，画像分 {average_score}。再积累几天后会形成更稳定的趋势判断。",
            work_day_type_title(dominant_day_type)
        );
    }

    let delta_text = if score_delta.abs() >= 10 {
        format!(
            "最近一次画像分较最早记录{}，节奏变化比较明显。",
            format_score_delta(score_delta)
        )
    } else {
        "最近几次画像分变化不大，整体节奏比较平稳。".to_string()
    };
    let timeline_text = if timeline_days > 0 {
        format!("已有 {timeline_days} 天带有 15 分钟节奏线，可用于回看具体片段。")
    } else {
        "这些历史记录还没有节奏线，后续新日志会逐步补齐。".to_string()
    };

    format!(
        "近 {sample_days} 个有记录日里，最常见的是{}，平均画像分 {average_score}。{delta_text}{timeline_text}",
        work_day_type_title(dominant_day_type)
    )
}

fn trend_insights(
    summaries: &[DailyWorkAssessmentSummary],
    average_score: u32,
    score_delta: i32,
    dominant_day_type: WorkDayType,
    best_summary: &DailyWorkAssessmentSummary,
    timeline_days: u32,
) -> Vec<AssessmentInsight> {
    let sample_days = summaries.len() as u32;
    let dominant_count = work_day_type_count(summaries, dominant_day_type);
    let mut insights = vec![
        insight(
            "近期主导形态",
            format!(
                "近 {sample_days} 个有记录日中，{}出现 {dominant_count} 次，是这段时间最常见的工作画像。",
                work_day_type_title(dominant_day_type)
            ),
            InsightSeverity::Positive,
            Some(work_day_type_title(dominant_day_type).to_string()),
        ),
        insight(
            "最高画像日",
            format!(
                "{} 的画像分最高，为 {} 分，类型是{}。",
                best_summary.date, best_summary.score, best_summary.day_type_title
            ),
            InsightSeverity::Positive,
            Some(format!("{} 分", best_summary.score)),
        ),
    ];

    if sample_days > 1 {
        if score_delta.abs() >= 10 {
            insights.push(insight(
                "节奏变化明显",
                format!(
                    "最近一次画像分较最早记录{}。CoreCat 会把这种变化视为近期工作节奏迁移，而不是好坏评价。",
                    format_score_delta(score_delta)
                ),
                InsightSeverity::Neutral,
                Some(format_score_delta(score_delta)),
            ));
        } else {
            insights.push(insight(
                "节奏比较稳定",
                format!(
                    "近 {sample_days} 个有记录日的平均画像分为 {average_score}，首尾变化不大，说明近期工况形态较稳定。"
                ),
                InsightSeverity::Neutral,
                Some(format!("{average_score} 分")),
            ));
        }
    }

    insights.push(insight(
        "节奏线覆盖",
        if timeline_days == sample_days {
            "这些历史画像都带有 15 分钟节奏线，适合回看具体工作片段。".to_string()
        } else if timeline_days > 0 {
            format!(
                "已有 {timeline_days}/{sample_days} 天带有 15 分钟节奏线，旧日志缺少切片时会保持摘要模式。"
            )
        } else {
            "当前历史画像还没有时间片数据，新生成的日报会逐步补齐节奏线。".to_string()
        },
        InsightSeverity::Neutral,
        Some(format!("{timeline_days}/{sample_days} 天")),
    ));

    insights.truncate(3);
    insights
}

fn format_score_delta(delta: i32) -> String {
    if delta == 0 {
        return "±0 分".to_string();
    }

    format!("{}{} 分", if delta > 0 { "+" } else { "-" }, delta.abs())
}

fn work_day_badges(day_type: WorkDayType, features: &WorkLogFeatures) -> Vec<String> {
    let mut badges = vec![match day_type {
        WorkDayType::DeepFocus => "FOCUS",
        WorkDayType::BuildBurst => "BUILD",
        WorkDayType::ArchiveFlow => "ARCHIVE",
        WorkDayType::PressureRepair => "REPAIR",
        WorkDayType::StableMaintenance => "STEADY",
        WorkDayType::FragmentedSwitching => "SWITCH",
        WorkDayType::LowLoadCompanion => "QUIET",
        WorkDayType::Unknown => "OBSERVE",
    }
    .to_string()];

    if features.thermal_warning_ratio() < 0.01 && features.active_seconds >= 1800 {
        badges.push("COOL".to_string());
    }
    if features.input_per_hour() >= 1200.0 {
        badges.push("RHYTHM".to_string());
    }

    badges.truncate(3);
    badges
}

fn build_workprint_grid(metrics: [f64; 5]) -> Vec<u8> {
    let mut grid = Vec::with_capacity(64);

    for y in 0..8 {
        for x in 0..8 {
            let metric = if y < 5 {
                metrics[y]
            } else {
                let a = metrics[(x + y) % metrics.len()];
                let b = metrics[(x * 2 + y) % metrics.len()];
                (a * 0.62 + b * 0.38).clamp(0.0, 1.0)
            };
            let threshold = (x + 1) as f64 / 8.0;
            let shimmer = ((x * 3 + y * 5) % 7) as f64 / 120.0;

            if metric + shimmer >= threshold {
                grid.push(intensity_grade(metric));
            } else {
                grid.push(0);
            }
        }
    }

    grid
}

fn intensity_grade(value: f64) -> u8 {
    if value >= 0.78 {
        4
    } else if value >= 0.52 {
        3
    } else if value >= 0.28 {
        2
    } else if value >= 0.08 {
        1
    } else {
        0
    }
}

fn baseline_summary(
    sample_days: u32,
    active_delta: f64,
    load_delta: f64,
    io_delta: f64,
    thermal_delta: f64,
    input_delta: f64,
) -> String {
    let comparisons = [
        ("陪伴时长", active_delta),
        ("工坊火力", load_delta),
        ("资料流动", io_delta),
        ("热压力", thermal_delta),
        ("输入节奏", input_delta),
    ];
    let Some((label, delta)) = comparisons
        .iter()
        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return "CoreCat 已经开始建立你的个人工作基线。".to_string();
    };

    if delta.abs() < 0.12 {
        return format!(
            "与近 {sample_days} 个有记录的工作日相比，今天整体节奏接近你的常规状态。"
        );
    }

    format!(
        "与近 {sample_days} 个有记录的工作日相比，今天的{label}{}。",
        if *delta > 0.0 {
            format!("更高，约 {}", format_delta(*delta))
        } else {
            format!("更低，约 {}", format_delta(*delta))
        }
    )
}

fn ratio_delta(current: f64, baseline: f64) -> f64 {
    if baseline.abs() < f64::EPSILON {
        return 0.0;
    }

    ((current - baseline) / baseline).clamp(-9.99, 9.99)
}

fn format_delta(value: f64) -> String {
    let percent = (value.abs() * 100.0).round() as i64;
    if value >= 0.0 {
        format!("+{percent}%")
    } else {
        format!("-{percent}%")
    }
}

fn insight(
    title: impl Into<String>,
    body: impl Into<String>,
    severity: InsightSeverity,
    metric_value: Option<String>,
) -> AssessmentInsight {
    AssessmentInsight {
        title: title.into(),
        body: body.into(),
        severity,
        metric_value,
    }
}

fn align_work_log_time_slice(timestamp: i64) -> i64 {
    timestamp - timestamp.rem_euclid(WORK_LOG_TIME_SLICE_MS)
}

fn format_time_slice_label(timestamp: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Local::now)
        .format("%H:%M")
        .to_string()
}

/// Aggregate a user's work-log time slices into a 24-hour × 7-day rhythm
/// profile. Each 15-min slice contributes its `active_seconds` to the matching
/// local hour and weekday (derived via chrono Local). When `history` entries
/// carry a score for a slice's date, that score is averaged into the bucket so
/// the heatmap can color "high quality" hours differently from merely "busy" ones.
pub fn build_rhythm_profile(
    book: &WorkLogBook,
    history: &[DailyWorkAssessmentSummary],
) -> RhythmProfile {
    // Build a date -> score lookup once for cheap per-slice joins.
    let score_by_date: std::collections::HashMap<&str, u32> = history
        .iter()
        .filter(|summary| summary.has_data)
        .map(|summary| (summary.date.as_str(), summary.score))
        .collect();

    let mut hours: [RhythmBucket; 24] = std::array::from_fn(|i| RhythmBucket {
        index: i as u8,
        ..Default::default()
    });
    let mut weekdays: [RhythmBucket; 7] = std::array::from_fn(|i| RhythmBucket {
        index: i as u8,
        ..Default::default()
    });

    // Track (sum_score, scored_day_count, distinct_day_set) per bucket to compute
    // a correct average later.
    let mut hour_score_sum = [0u64; 24];
    let mut hour_scored_days = [0u32; 24];
    let mut weekday_score_sum = [0u64; 7];
    let mut weekday_scored_days = [0u32; 7];
    // Distinct days per bucket — approximated by counting a day once we see it.
    use std::collections::HashSet;
    let mut hour_days: [HashSet<String>; 24] = std::array::from_fn(|_| HashSet::new());
    let mut weekday_days: [HashSet<String>; 7] = std::array::from_fn(|_| HashSet::new());

    for (date, entry) in &book.entries {
        for slice in &entry.time_slices {
            let Some(local) = Local.timestamp_millis_opt(slice.start_timestamp).single() else {
                continue;
            };
            let hour = local.hour() as usize;
            let weekday = (local.weekday().num_days_from_monday()) as usize;

            hours[hour].active_seconds = hours[hour]
                .active_seconds
                .saturating_add(slice.active_seconds);
            weekdays[weekday].active_seconds = weekdays[weekday]
                .active_seconds
                .saturating_add(slice.active_seconds);

            hour_days[hour].insert(date.clone());
            weekday_days[weekday].insert(date.clone());

            if let Some(score) = score_by_date.get(date.as_str()) {
                hour_score_sum[hour] += *score as u64;
                hour_scored_days[hour] += 1;
                weekday_score_sum[weekday] += *score as u64;
                weekday_scored_days[weekday] += 1;
            }
        }
    }

    for i in 0..24 {
        hours[i].sample_days = hour_days[i].len() as u32;
        hours[i].avg_focus_score = if hour_scored_days[i] > 0 {
            hour_score_sum[i] as f64 / hour_scored_days[i] as f64
        } else {
            0.0
        };
    }
    for i in 0..7 {
        weekdays[i].sample_days = weekday_days[i].len() as u32;
        weekdays[i].avg_focus_score = if weekday_scored_days[i] > 0 {
            weekday_score_sum[i] as f64 / weekday_scored_days[i] as f64
        } else {
            0.0
        };
    }

    // Peak hours: rank by a blend of activity volume and focus score.
    let mut ranked: Vec<(usize, f64)> = (0..24)
        .map(|i| {
            let activity = hours[i].active_seconds as f64;
            let focus = hours[i].avg_focus_score;
            (i, activity.max(1.0).ln() + focus * 0.05)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let peak_hours: Vec<u8> = ranked.iter().take(3).map(|(i, _)| *i as u8).collect();

    let summary = build_rhythm_summary(&hours, &peak_hours);

    RhythmProfile {
        hour_buckets: hours.to_vec(),
        weekday_buckets: weekdays.to_vec(),
        peak_hours,
        summary,
    }
}

fn build_rhythm_summary(hours: &[RhythmBucket], peak_hours: &[u8]) -> String {
    if peak_hours.is_empty() || hours.iter().all(|h| h.active_seconds == 0) {
        return "还没有足够的数据来识别你的节律，多用几天就能看到规律啦。".to_string();
    }
    let labels: Vec<String> = peak_hours
        .iter()
        .map(|h| format!("{:02}:00", h))
        .collect();
    format!(
        "你的高效时段集中在 {}。在这些时段安排重要任务，CoreCat 会陪你一起发力。",
        labels.join("、")
    )
}

pub fn today_key() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn date_key_from_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Local::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn summarize_work_score(score: u32) -> String {
    match score {
        91..=100 => "极高投入工作日：长时间运行、硬件负载、IO 吞吐和键鼠操作都很密集。".to_string(),
        76..=90 => "高投入工作日：CoreCat 观察到持续运行、明确任务压力和较高操作密度。".to_string(),
        51..=75 => "稳定推进日：运行时长、硬件参与度与操作记录较均衡，工作投入清晰。".to_string(),
        31..=50 => "轻中度工作日：有一定运行与操作记录，但高强度负载或复杂 IO 较少。".to_string(),
        1..=30 => "轻量维护日：工作窗口较短或操作密度较低，偏向整理与低强度任务。".to_string(),
        _ => "暂无足够数据：保持 CoreCat 运行后，将自动生成当天工作投入度。".to_string(),
    }
}

fn bounded_metric(value: Option<f32>) -> f64 {
    value.unwrap_or(0.0).clamp(0.0, 100.0) as f64
}

fn normalize_process_name(name: &str) -> Option<String> {
    let trimmed = name.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return None;
    }

    let file_name = trimmed
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(trimmed)
        .trim();

    (!file_name.is_empty()).then(|| file_name.to_string())
}

fn should_ignore_process_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized.contains("coreworkpal")
        || normalized.contains("core-work-pal")
        || normalized == "system idle process"
        || normalized == "idle"
}

fn thermal_pressure(snapshot: &HardwareSnapshot) -> f64 {
    let cpu_pressure = snapshot
        .cpu_temperature_celsius
        .map(|value| ((value - 45.0) / 50.0 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let gpu_pressure = snapshot
        .gpu_temperature_celsius
        .map(|value| ((value - 45.0) / 50.0 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    cpu_pressure.max(gpu_pressure) as f64
}

fn io_activity(snapshot: &HardwareSnapshot) -> f64 {
    let bytes_per_second = snapshot.disk_read_bytes_per_second.unwrap_or(0.0)
        + snapshot.disk_write_bytes_per_second.unwrap_or(0.0)
        + snapshot.network_download_bytes_per_second.unwrap_or(0.0)
        + snapshot.network_upload_bytes_per_second.unwrap_or(0.0);

    (bytes_per_second as f64 / (5.0 * 1024.0 * 1024.0) * 100.0).clamp(0.0, 100.0)
}

fn duration_ratio(seconds: u64, active_seconds: u64) -> f64 {
    if active_seconds == 0 {
        return 0.0;
    }

    seconds as f64 / active_seconds as f64
}

fn rate_total_bytes(rate_bytes_per_second: Option<f32>, delta_seconds: u64) -> u64 {
    if delta_seconds == 0 {
        return 0;
    }

    (rate_bytes_per_second.unwrap_or(0.0).max(0.0) as f64 * delta_seconds as f64).round() as u64
}

fn volume_score(value: u64, full_value: f64) -> f64 {
    if full_value <= 0.0 {
        return 0.0;
    }

    (value as f64 / full_value).clamp(0.0, 1.0)
}

fn metric_fact(label: impl Into<String>, value: impl Into<String>) -> WorkLogMetricFact {
    WorkLogMetricFact {
        label: label.into(),
        value: value.into(),
    }
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("{hours}h {minutes}m")
}

fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        return format!("{} {}", bytes, units[unit_index]);
    }

    format!("{value:.1} {}", units[unit_index])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutState {
    pub schema_version: u32,
    pub cat_window_x: f64,
    pub cat_window_y: f64,
    pub monitor_bar_x: f64,
    pub monitor_bar_y: f64,
}

impl Default for LayoutState {
    fn default() -> Self {
        let settings = AppSettings::default();
        Self {
            schema_version: 1,
            cat_window_x: settings.cat_window_x,
            cat_window_y: settings.cat_window_y,
            monitor_bar_x: settings.monitor_bar_x,
            monitor_bar_y: settings.monitor_bar_y,
        }
    }
}

pub fn current_timestamp_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod assessment_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn rhythm_profile_aggregates_active_seconds_by_local_hour() {
        // Two slices on the same day at 10:00 and 10:15 local → both land in hour 10.
        let base = Local
            .timestamp_millis_opt(0)
            .single()
            .unwrap()
            .date()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let day = base.format("%Y-%m-%d").to_string();
        let t1 = Local
            .from_local_datetime(&base.naive_local())
            .single()
            .unwrap()
            .timestamp_millis();
        let t2 = t1 + WORK_LOG_TIME_SLICE_MS as i64;

        let mut book = WorkLogBook::default();
        let entry = WorkLogEntry {
            date: day.clone(),
            ..Default::default()
        };
        book.entries.insert(day, entry);
        let entry = book.entries.values_mut().next().unwrap();
        for t in [t1, t2] {
            let slice = WorkLogTimeSlice {
                start_timestamp: t,
                end_timestamp: t + WORK_LOG_TIME_SLICE_MS as i64,
                active_seconds: 600,
                ..Default::default()
            };
            entry.time_slices.push(slice);
        }

        let profile = build_rhythm_profile(&book, &[]);
        let hour10 = &profile.hour_buckets[10];
        assert_eq!(hour10.active_seconds, 1200);
        assert_eq!(hour10.sample_days, 1);
        assert!(profile.peak_hours.contains(&10));
    }

    #[test]
    fn rhythm_profile_handles_empty_work_log() {
        let book = WorkLogBook::default();
        let profile = build_rhythm_profile(&book, &[]);
        assert!(profile.hour_buckets.iter().all(|h| h.active_seconds == 0));
        assert!(profile.peak_hours.is_empty() || profile.hour_buckets.iter().all(|h| h.active_seconds == 0));
    }

    fn test_rarity(tier: &str) -> WorkCardRarity {
        WorkCardRarity {
            tier: tier.to_string(),
            label: format!("{tier} 级工况卡"),
            score: 60,
            reason: "test".to_string(),
        }
    }

    fn test_title(title: &str) -> WorkDayTitle {
        WorkDayTitle {
            family: "test".to_string(),
            title: title.to_string(),
            level: 1,
            progress: 1,
            next_level_at: Some(3),
        }
    }

    fn test_workprint() -> WorkprintSummary {
        WorkprintSummary {
            label: "测试指纹".to_string(),
            description: "test".to_string(),
            pixel_grid: vec![0; 64],
            width: 8,
            height: 8,
            load_shape: 0.0,
            input_rhythm: 0.0,
            io_intensity: 0.0,
            thermal_pressure: 0.0,
            continuity: 0.0,
        }
    }

    #[test]
    fn daily_assessment_handles_empty_day() {
        let entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            ..Default::default()
        };

        let assessment = DailyWorkAssessment::from_entry(entry, &[]);

        assert_eq!(assessment.day_type, WorkDayType::Unknown);
        assert_eq!(assessment.baseline.sample_days, 0);
        assert_eq!(assessment.baseline.metrics.len(), 5);
        assert!(!assessment.highlights.is_empty());
        assert!(!assessment.suggestions.is_empty());
    }

    #[test]
    fn daily_assessment_detects_build_burst() {
        let entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            active_seconds: 3600,
            sample_count: 60,
            cpu_load_points: 60.0 * 68.0,
            memory_load_points: 60.0 * 54.0,
            io_activity_points: 60.0 * 48.0,
            high_load_seconds: 1800,
            cpu_over_50_seconds: 1800,
            disk_read_bytes_total: 1024 * 1024 * 1024,
            keyboard_press_count: 2400,
            mouse_click_count: 300,
            ..Default::default()
        };

        let assessment = DailyWorkAssessment::from_entry(entry, &[]);

        assert_eq!(assessment.day_type, WorkDayType::BuildBurst);
        assert_eq!(assessment.day_type_title, "编译构建日");
        assert!(assessment.badge_ids.iter().any(|badge| badge == "BUILD"));
    }

    #[test]
    fn daily_assessment_compares_against_history() {
        let entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            active_seconds: 7200,
            sample_count: 80,
            cpu_load_points: 80.0 * 36.0,
            memory_load_points: 80.0 * 42.0,
            io_activity_points: 80.0 * 58.0,
            disk_write_bytes_total: 2 * 1024 * 1024 * 1024,
            network_download_bytes_total: 1024 * 1024 * 1024,
            keyboard_press_count: 3200,
            ..Default::default()
        };
        let history = vec![WorkLogEntry {
            date: "2026-06-22".to_string(),
            active_seconds: 3600,
            sample_count: 60,
            cpu_load_points: 60.0 * 24.0,
            memory_load_points: 60.0 * 35.0,
            io_activity_points: 60.0 * 12.0,
            keyboard_press_count: 900,
            ..Default::default()
        }];

        let assessment = DailyWorkAssessment::from_entry(entry, &history);

        assert_eq!(assessment.baseline.sample_days, 1);
        assert_eq!(assessment.baseline.metrics.len(), 5);
        assert!(assessment
            .baseline
            .metrics
            .iter()
            .any(|metric| metric.key == "io" && metric.baseline_value != "--"));
        assert!(assessment.baseline.io_delta_ratio > 1.0);
        assert!(assessment.corecat_commentary.body.contains("工况解释"));
        assert!(assessment
            .baseline
            .summary
            .contains("近 1 个有记录的工作日"));
    }

    #[test]
    fn work_log_entry_records_snapshot_time_slices() {
        let start = 1_780_000_000_000;
        let mut entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            started_at: start,
            updated_at: start,
            ..Default::default()
        };
        let snapshot = HardwareSnapshot {
            timestamp: start + 60_000,
            cpu_usage_percent: Some(65.0),
            memory_usage_percent: Some(52.0),
            disk_read_bytes_per_second: Some(8.0 * 1024.0 * 1024.0),
            ..Default::default()
        };

        entry.record_snapshot(&snapshot, start + 60_000);

        assert_eq!(entry.time_slices.len(), 1);
        assert_eq!(entry.time_slices[0].sample_count, 1);
        assert!(entry.time_slices[0].disk_read_bytes_total > 0);
    }

    #[test]
    fn process_usage_counts_same_process_name_once_per_sample() {
        let start = 1_780_000_000_000;
        let mut entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            started_at: start,
            updated_at: start,
            ..Default::default()
        };
        let snapshot = HardwareSnapshot {
            timestamp: start + 60_000,
            processes: vec![
                ProcessUsageSnapshot {
                    pid: 10,
                    name: "Code.exe".to_string(),
                    cpu_usage_percent: 0.4,
                    memory_bytes: 120 * 1024 * 1024,
                    disk_read_bytes_per_second: Some(16.0 * 1024.0),
                    disk_write_bytes_per_second: Some(0.0),
                },
                ProcessUsageSnapshot {
                    pid: 11,
                    name: "code.exe".to_string(),
                    cpu_usage_percent: 0.7,
                    memory_bytes: 180 * 1024 * 1024,
                    disk_read_bytes_per_second: Some(20.0 * 1024.0),
                    disk_write_bytes_per_second: Some(0.0),
                },
            ],
            ..Default::default()
        };

        entry.record_snapshot(&snapshot, start + 60_000);

        let usage = entry.process_usage.get("code.exe").expect("code usage");
        assert_eq!(usage.sample_count, 1);
        assert_eq!(usage.active_sample_count, 1);
        assert_eq!(usage.observed_seconds, 60);
        assert_eq!(usage.active_seconds, 60);
        assert_eq!(usage.memory_bytes_peak, 300 * 1024 * 1024);
        assert!(usage.disk_read_bytes_total > 2 * 1024 * 1024);
    }

    #[test]
    fn process_insights_cap_legacy_durations_to_day_runtime() {
        let mut process_usage = BTreeMap::new();
        process_usage.insert(
            "browser.exe".to_string(),
            WorkLogProcessUsage {
                name: "browser.exe".to_string(),
                sample_count: 240,
                active_sample_count: 180,
                observed_seconds: 12 * 3600,
                active_seconds: 9 * 3600,
                cpu_pressure_points: 12.0 * 12.0 * 3600.0,
                memory_bytes_peak: 1024 * 1024 * 1024,
                ..Default::default()
            },
        );
        let entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            active_seconds: 3600,
            sample_count: 60,
            process_usage,
            ..Default::default()
        };

        let insights = build_process_insights(&entry);

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].observed_seconds, 3600);
        assert_eq!(insights[0].active_seconds, 3600);
        assert_eq!(insights[0].sample_count, 60);
        assert_eq!(insights[0].active_sample_count, 60);
        assert!(insights[0].summary.contains("驻留 1h 0m"));
        assert!(insights[0].summary.contains("活跃 1h 0m"));
    }

    #[test]
    fn input_activity_is_added_to_time_slice() {
        let timestamp = 1_780_000_000_000;
        let mut entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            ..Default::default()
        };

        entry.record_input_activity_at(12, 48, timestamp);

        assert_eq!(entry.mouse_click_count, 12);
        assert_eq!(entry.keyboard_press_count, 48);
        assert_eq!(entry.time_slices.len(), 1);
        assert_eq!(entry.time_slices[0].keyboard_press_count, 48);
    }

    #[test]
    fn daily_assessment_returns_timeline_segments() {
        let start = 1_780_000_000_000;
        let mut entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            started_at: start,
            updated_at: start,
            ..Default::default()
        };
        let snapshot = HardwareSnapshot {
            timestamp: start + 60_000,
            cpu_usage_percent: Some(72.0),
            memory_usage_percent: Some(58.0),
            disk_read_bytes_per_second: Some(10.0 * 1024.0 * 1024.0),
            ..Default::default()
        };

        entry.record_snapshot(&snapshot, start + 60_000);
        let assessment = DailyWorkAssessment::from_entry(entry, &[]);

        assert_eq!(assessment.timeline.len(), 1);
        assert_eq!(assessment.timeline[0].kind, WorkTimelineSegmentKind::BuildPeak);
    }

    #[test]
    fn daily_assessment_summary_marks_timeline_presence() {
        let start = 1_780_000_000_000;
        let entry = WorkLogEntry {
            date: "2026-06-23".to_string(),
            active_seconds: 900,
            sample_count: 6,
            time_slices: vec![WorkLogTimeSlice {
                start_timestamp: start,
                end_timestamp: start + WORK_LOG_TIME_SLICE_MS,
                active_seconds: 900,
                sample_count: 6,
                cpu_load_points: 6.0 * 36.0,
                memory_load_points: 6.0 * 42.0,
                ..Default::default()
            }],
            ..Default::default()
        };

        let summary = DailyWorkAssessment::from_entry(entry, &[]).summary(true);

        assert_eq!(summary.date, "2026-06-23");
        assert!(summary.has_timeline);
        assert!(summary.has_data);
        assert!(!summary.badge_ids.is_empty());
    }

    #[test]
    fn daily_assessment_trend_summarizes_recent_history() {
        let summaries = vec![
            DailyWorkAssessmentSummary {
                date: "2026-06-23".to_string(),
                day_type: WorkDayType::BuildBurst,
                day_type_title: "编译构建日".to_string(),
                rarity: test_rarity("S"),
                title: test_title("风暴构筑师"),
                workprint: test_workprint(),
                score: 78,
                corecat_summary: String::new(),
                badge_ids: vec!["BUILD".to_string()],
                has_timeline: true,
                has_data: true,
            },
            DailyWorkAssessmentSummary {
                date: "2026-06-22".to_string(),
                day_type: WorkDayType::DeepFocus,
                day_type_title: "深度专注日".to_string(),
                rarity: test_rarity("S"),
                title: test_title("深潜构筑师"),
                workprint: test_workprint(),
                score: 86,
                corecat_summary: String::new(),
                badge_ids: vec!["FOCUS".to_string()],
                has_timeline: true,
                has_data: true,
            },
            DailyWorkAssessmentSummary {
                date: "2026-06-21".to_string(),
                day_type: WorkDayType::BuildBurst,
                day_type_title: "编译构建日".to_string(),
                rarity: test_rarity("A"),
                title: test_title("火力装配员"),
                workprint: test_workprint(),
                score: 60,
                corecat_summary: String::new(),
                badge_ids: vec!["BUILD".to_string()],
                has_timeline: false,
                has_data: true,
            },
        ];

        let trend = DailyWorkAssessmentTrend::from_summaries(&summaries);

        assert_eq!(trend.sample_days, 3);
        assert_eq!(trend.dominant_day_type, WorkDayType::BuildBurst);
        assert_eq!(trend.best_date.as_deref(), Some("2026-06-22"));
        assert_eq!(trend.score_delta, 18);
        assert_eq!(trend.timeline_days, 2);
        assert!(!trend.insights.is_empty());
    }

    #[test]
    fn daily_assessment_trend_handles_empty_history() {
        let trend = DailyWorkAssessmentTrend::from_summaries(&[]);

        assert_eq!(trend.sample_days, 0);
        assert_eq!(trend.dominant_day_type, WorkDayType::Unknown);
        assert_eq!(trend.best_score, None);
        assert_eq!(trend.insights.len(), 1);
    }
}
