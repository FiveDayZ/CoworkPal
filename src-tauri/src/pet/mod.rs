use tauri::{AppHandle, Emitter, Manager};

use crate::{
    app_state::AppState,
    models::{
        current_timestamp_ms, AppSettings, CatState, CatStateChangedEvent, HardwareSnapshot,
        ProcessUsageSnapshot,
    },
};

mod process_classify;

const MIN_STATE_HOLD_MS: i64 = 3_000;
const REPAIR_LIGHT_LOAD_THRESHOLD: f32 = 76.0;
const REPAIR_HEAVY_LOAD_THRESHOLD: f32 = 92.0;
const TEMPERATURE_CHECK_ENTER_MIN_CELSIUS: f32 = 75.0;
const TEMPERATURE_CHECK_EXIT_CELSIUS: f32 = 70.0;
const TEMPERATURE_CHECK_EXIT_STABLE_MS: i64 = 5_000;
/// After this much continuous work (with input present), CoCat looks tired.
const FATIGUED_AFTER_MS: i64 = 90 * 60 * 1000;
/// Once input has been silent this long (while the machine stayed on), suggest
/// a break — the user has likely been staring at the screen without interacting.
const NEEDS_BREAK_AFTER_MS: i64 = 50 * 60 * 1000;
/// Idle gap long enough that we no longer count the session as "continuous work".
const CONTINUOUS_WORK_BREAK_MS: i64 = 5 * 60 * 1000;
/// During a focus session, this much input silence counts as a distraction.
pub const FOCUS_DISTRACTION_SILENCE_MS: i64 = 90 * 1000;
pub const FOCUS_NUDGE_HOLD_MS: i64 = 5 * 60 * 1000;
const FOCUS_FATIGUE_PROGRESS_NUMERATOR: i64 = 4;
const FOCUS_FATIGUE_PROGRESS_DENOMINATOR: i64 = 5;

pub struct PetStateService;

impl PetStateService {
    pub async fn update_for_snapshot(app: &AppHandle, snapshot: &HardwareSnapshot) {
        let now_ms = current_timestamp_ms();
        let state = app.state::<AppState>();
        let settings = state.settings.read().await.clone();

        // Read, decide, and write the entire cat runtime state under a single
        // lock guard. Previously these were five separate RwLocks read in one
        // phase and written in another, so a concurrent tick or settings update
        // could interleave between the read and the write, producing a decision
        // from stale state or a lost update.
        let payload = {
            let mut runtime = state.cat_runtime.write().await;

            // Maintain the continuous-work marker from the input timestamp. We
            // count a stretch as continuous while input is recent; a long enough
            // gap ends the stretch (so the fatigue nudge resets after a real break).
            let recent_input = runtime
                .last_input_at
                .is_some_and(|t| now_ms.saturating_sub(t) < CONTINUOUS_WORK_BREAK_MS);
            if recent_input {
                runtime.continuous_work_since.get_or_insert(now_ms);
            } else {
                runtime.continuous_work_since = None;
            }

            let temperature_safe_since = next_temperature_safe_since(
                &settings,
                snapshot,
                &runtime.cat_state,
                runtime.temperature_safe_since,
                now_ms,
            );
            let candidate = resolve_candidate_state(
                &settings,
                snapshot,
                &runtime.cat_state,
                temperature_safe_since,
                now_ms,
                runtime.last_input_at,
                runtime.continuous_work_since,
                runtime.active_focus_session_id.as_deref(),
                runtime.active_focus_started_at,
                runtime.active_focus_planned_duration_ms,
                runtime.focus_nudge_state.as_ref(),
                runtime.focus_nudge_until,
            );

            // Always advance the temperature-safe marker; it is derived from the
            // snapshot, not from whether the visible state transitions.
            runtime.temperature_safe_since = temperature_safe_since;

            if runtime.has_emitted_cat_state
                && !should_transition(
                    &runtime.cat_state,
                    &candidate,
                    now_ms.saturating_sub(runtime.last_cat_state_changed_at),
                )
            {
                return;
            }

            // Note: the bubble text is (re)computed only when the CatState
            // actually transitions — `should_transition` above returns early
            // when the state is unchanged, so the lead-process story below is
            // refreshed on state changes, not on every tick. In practice the
            // story lead (the top story-worthy process) is stable while a given
            // workload persists, so this matches user expectations. Re-evaluating
            // every tick would also re-fire `pet:state-changed`, which the
            // frontend maps to OS notifications for the alert states.
            let lead = process_classify::pick_story_lead(&snapshot.processes);
            let message = message_for_state(&candidate, lead);
            runtime.cat_state = candidate.clone();
            runtime.cat_message = message.clone();
            runtime.last_cat_state_changed_at = now_ms;
            runtime.has_emitted_cat_state = true;

            CatStateChangedEvent {
                timestamp: now_ms,
                cat_state: candidate,
                cat_message: message,
            }
        };

        if let Err(error) = app.emit(crate::events::PET_STATE_CHANGED, payload) {
            tracing::warn!("failed to emit pet:state-changed: {error}");
        }
    }
}

fn resolve_candidate_state(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    current: &CatState,
    temperature_safe_since: Option<i64>,
    now_ms: i64,
    last_input_at: Option<i64>,
    continuous_work_since: Option<i64>,
    active_focus_session_id: Option<&str>,
    active_focus_started_at: Option<i64>,
    active_focus_planned_duration_ms: Option<i64>,
    focus_nudge_state: Option<&CatState>,
    focus_nudge_until: Option<i64>,
) -> CatState {
    if !settings.is_cat_visible {
        return CatState::Hidden;
    }

    if settings.is_production_paused {
        return CatState::Sleep;
    }

    if is_temperature_warning(settings, snapshot, current, temperature_safe_since, now_ms) {
        return CatState::TemperatureCheck;
    }

    if snapshot
        .memory_usage_percent
        .is_some_and(|value| value > settings.memory_crowded_threshold)
    {
        return CatState::MemoryCrowded;
    }

    if is_heavy_load(snapshot) {
        return CatState::RepairHeavy;
    }

    if snapshot
        .cpu_usage_percent
        .is_some_and(|value| value >= settings.data_sorting_cpu_threshold)
    {
        return CatState::RepairLight;
    }

    if is_sustained_busy_load(snapshot) {
        return CatState::RepairLight;
    }

    // Focus ritual takes priority over generic wellness nudges but yields to
    // the system alerts above.
    if active_focus_session_id.is_some() {
        let distracted = last_input_at
            .is_some_and(|t| now_ms.saturating_sub(t) >= FOCUS_DISTRACTION_SILENCE_MS);
        if distracted {
            return CatState::Distracted;
        }

        if is_focus_fatigued(
            now_ms,
            active_focus_started_at,
            active_focus_planned_duration_ms,
        ) {
            return CatState::Fatigued;
        }

        return CatState::DeepWork;
    }

    if let (Some(state), Some(until)) = (focus_nudge_state, focus_nudge_until) {
        if now_ms < until {
            return state.clone();
        }
    }

    // Health nudges: only when the machine is otherwise calm (no load/thermal
    // condition above) so we never mask a real alert with a wellness message.
    // No input for a long stretch while the app kept sampling → suggest a break.
    if let Some(last) = last_input_at {
        let silent_for = now_ms.saturating_sub(last);
        if silent_for >= NEEDS_BREAK_AFTER_MS {
            return CatState::NeedsBreak;
        }
    }

    // Long continuous-work stretch with input present → fatigue nudge.
    if let Some(since) = continuous_work_since {
        if now_ms.saturating_sub(since) >= FATIGUED_AFTER_MS {
            return CatState::Fatigued;
        }
    }

    CatState::Idle
}

fn is_focus_fatigued(
    now_ms: i64,
    active_focus_started_at: Option<i64>,
    active_focus_planned_duration_ms: Option<i64>,
) -> bool {
    let (Some(started_at), Some(planned_ms)) =
        (active_focus_started_at, active_focus_planned_duration_ms)
    else {
        return false;
    };
    let threshold_ms =
        planned_ms * FOCUS_FATIGUE_PROGRESS_NUMERATOR / FOCUS_FATIGUE_PROGRESS_DENOMINATOR;
    now_ms.saturating_sub(started_at) >= threshold_ms
}

fn is_temperature_warning(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    current: &CatState,
    temperature_safe_since: Option<i64>,
    now_ms: i64,
) -> bool {
    if is_temperature_above_enter_threshold(settings, snapshot) {
        return true;
    }

    if current != &CatState::TemperatureCheck {
        return false;
    }

    if !is_temperature_below_exit_threshold(snapshot) {
        return true;
    }

    temperature_safe_since
        .is_some_and(|safe_since| now_ms.saturating_sub(safe_since) < TEMPERATURE_CHECK_EXIT_STABLE_MS)
}

fn is_temperature_above_enter_threshold(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
) -> bool {
    let cpu_warning = settings
        .cpu_temperature_warning
        .max(TEMPERATURE_CHECK_ENTER_MIN_CELSIUS);
    let gpu_warning = settings
        .gpu_temperature_warning
        .max(TEMPERATURE_CHECK_ENTER_MIN_CELSIUS);

    snapshot
        .cpu_temperature_celsius
        .is_some_and(|value| value > cpu_warning)
        || snapshot
            .gpu_temperature_celsius
            .is_some_and(|value| value > gpu_warning)
}

fn is_temperature_below_exit_threshold(snapshot: &HardwareSnapshot) -> bool {
    let mut has_temperature = false;
    let mut all_safe = true;

    for value in [
        snapshot.cpu_temperature_celsius,
        snapshot.gpu_temperature_celsius,
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_finite() {
            continue;
        }

        has_temperature = true;
        all_safe &= value < TEMPERATURE_CHECK_EXIT_CELSIUS;
    }

    // Only declare "safe" when we actually have a reading that confirms it.
    // Previously a missing reading (!has_temperature) was treated as safe, so a
    // momentary sensor dropout (e.g. LibreHardwareMonitor restarting) would
    // immediately dismiss an active high-temperature alert even though the CPU
    // was still hot. Now a missing reading keeps the alert active until real
    // cool-down data arrives.
    has_temperature && all_safe
}

fn next_temperature_safe_since(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    current: &CatState,
    current_safe_since: Option<i64>,
    now_ms: i64,
) -> Option<i64> {
    if is_temperature_above_enter_threshold(settings, snapshot) {
        return None;
    }

    if current == &CatState::TemperatureCheck && is_temperature_below_exit_threshold(snapshot) {
        return Some(current_safe_since.unwrap_or(now_ms));
    }

    None
}

fn is_heavy_load(snapshot: &HardwareSnapshot) -> bool {
    snapshot
        .cpu_usage_percent
        .is_some_and(|value| value >= REPAIR_HEAVY_LOAD_THRESHOLD)
        || snapshot
            .gpu_usage_percent
            .is_some_and(|value| value >= REPAIR_HEAVY_LOAD_THRESHOLD)
}

fn is_sustained_busy_load(snapshot: &HardwareSnapshot) -> bool {
    snapshot
        .gpu_usage_percent
        .is_some_and(|value| value >= REPAIR_LIGHT_LOAD_THRESHOLD)
}

fn should_transition(current: &CatState, candidate: &CatState, elapsed_ms: i64) -> bool {
    if current == candidate {
        return false;
    }

    if severity(candidate) > severity(current) {
        return true;
    }

    elapsed_ms >= MIN_STATE_HOLD_MS
}

fn severity(state: &CatState) -> u8 {
    match state {
        CatState::Hidden => 100,
        CatState::TemperatureCheck => 90,
        CatState::RepairHeavy => 80,
        CatState::MemoryCrowded => 70,
        CatState::DeepWork => 60,
        CatState::RepairLight => 50,
        CatState::NeedsBreak => 46,
        CatState::Sleep => 40,
        CatState::Fatigued => 36,
        CatState::Distracted => 34,
        CatState::DataSorting => 30,
        CatState::Celebrate => 15,
        CatState::Interactive => 8,
        CatState::Idle => 0,
    }
}

fn message_for_state(state: &CatState, lead: Option<&ProcessUsageSnapshot>) -> String {
    // Base text per state — the static fallback when no story-worthy process
    // is recognized (or the state is not "work-like").
    let base = match state {
        CatState::Idle => "CoCat 正在待命。",
        CatState::RepairLight => "检测到轻量维护负载。",
        CatState::RepairHeavy => "系统负载偏高，CoCat 正在检修。",
        CatState::TemperatureCheck => "温度偏高，正在关注散热状态。",
        CatState::MemoryCrowded => "内存较拥挤，建议留意后台任务。",
        CatState::DataSorting => "系统空闲，CoCat 正在整理数据。",
        CatState::Sleep => "长时间未操作，CoCat 进入休眠。",
        CatState::Interactive => "CoCat 正在响应你的操作。",
        CatState::Celebrate => "清理完成，工坊状态良好。",
        CatState::Fatigued => "连续工作很久啦，CoCat 也想歇一会儿。",
        CatState::NeedsBreak => "久坐提醒：起来活动一下，喝口水吧。",
        CatState::DeepWork => "专注仪式进行中，CoCat 陪你一起埋头干活。",
        CatState::Distracted => "好像走神了？深呼吸，回到任务上来吧。",
        CatState::Hidden => return String::new(),
    };

    // Only enrich "work-like" states with a process story. Other states
    // (temperature, sleep, celebrate, …) keep their domain-specific text so the
    // nudge stays focused.
    let category = lead.map(|p| process_classify::classify_process(&p.name));
    let Some(category) = category else {
        return base.to_string();
    };

    let enriched = match (state, category) {
        (CatState::RepairHeavy, process_classify::ProcessCategory::Compiler) => {
            Some("编译负载很高，CoCat 满头大汗地陪你一起敲键盘。")
        }
        (CatState::RepairLight, process_classify::ProcessCategory::Compiler) => {
            Some("检测到编译任务，CoCat 在旁边帮你一起敲键盘。")
        }
        (CatState::RepairHeavy, process_classify::ProcessCategory::Browser) => {
            Some("浏览器吃掉不少资源，CoCat 挤在角落里帮它扇风。")
        }
        (CatState::RepairHeavy | CatState::RepairLight, process_classify::ProcessCategory::Game) => {
            Some("检测到游戏在跑，CoCat 戴上耳机在旁边围观。")
        }
        (CatState::RepairHeavy | CatState::RepairLight, process_classify::ProcessCategory::Ide) => {
            Some("你的编辑器正忙，CoCat 趴在键盘边盯着代码。")
        }
        (CatState::RepairHeavy | CatState::RepairLight, process_classify::ProcessCategory::VideoCall) => {
            Some("视频会议进行中，CoCat 安静地躲到屏幕后面。")
        }
        (CatState::MemoryCrowded, process_classify::ProcessCategory::Browser) => {
            Some("浏览器吃掉不少内存，CoCat 被挤到了角落里蹲着。")
        }
        (CatState::DeepWork, process_classify::ProcessCategory::Ide) => {
            Some("你在编辑器里专注，CoCat 趴在键盘边静静陪着。")
        }
        (CatState::DeepWork, process_classify::ProcessCategory::Compiler) => {
            Some("专注仪式 + 编译任务，CoCat 和你一起埋头干活。")
        }
        _ => None,
    };

    enriched.unwrap_or(base).to_string()
}

#[cfg(test)]
mod tests {
    use crate::models::{AppSettings, CatState, HardwareSnapshot, ProcessUsageSnapshot};

    use super::{message_for_state, resolve_candidate_state, should_transition};

    fn candidate(settings: &AppSettings, snapshot: &HardwareSnapshot) -> CatState {
        resolve_candidate_state(
            settings,
            snapshot,
            &CatState::Idle,
            None,
            1_000,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    fn hidden_setting_overrides_snapshot() {
        let mut settings = AppSettings::default();
        settings.is_cat_visible = false;
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(95.0),
            ..Default::default()
        };

        assert_eq!(
            candidate(&settings, &snapshot),
            CatState::Hidden
        );
    }

    #[test]
    fn working_cpu_threshold_maps_to_repair_light() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(40.0),
            ..Default::default()
        };

        assert_eq!(
            candidate(&settings, &snapshot),
            CatState::RepairLight
        );
    }

    #[test]
    fn ram_above_threshold_maps_to_memory_crowded() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot {
            memory_usage_percent: Some(85.1),
            ..Default::default()
        };

        assert_eq!(
            candidate(&settings, &snapshot),
            CatState::MemoryCrowded
        );
    }

    #[test]
    fn high_load_maps_to_repair_states() {
        let settings = AppSettings::default();

        assert_eq!(
            candidate(
                &settings,
                &HardwareSnapshot {
                    cpu_usage_percent: Some(92.0),
                    ..Default::default()
                }
            ),
            CatState::RepairHeavy
        );
        assert_eq!(
            candidate(
                &settings,
                &HardwareSnapshot {
                    gpu_usage_percent: Some(76.0),
                    ..Default::default()
                }
            ),
            CatState::RepairLight
        );
    }

    #[test]
    fn temperature_above_threshold_maps_to_temperature_check() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot {
            cpu_temperature_celsius: Some(80.1),
            ..Default::default()
        };

        assert_eq!(
            candidate(&settings, &snapshot),
            CatState::TemperatureCheck
        );
    }

    #[test]
    fn moderate_temperature_does_not_enter_temperature_check() {
        let mut settings = AppSettings::default();
        settings.cpu_temperature_warning = 50.0;
        let snapshot = HardwareSnapshot {
            cpu_temperature_celsius: Some(60.0),
            ..Default::default()
        };

        assert_eq!(candidate(&settings, &snapshot), CatState::Idle);
    }

    #[test]
    fn temperature_check_exits_after_safe_temperature_is_stable() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot {
            cpu_temperature_celsius: Some(60.0),
            ..Default::default()
        };

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::TemperatureCheck,
                Some(1_000),
                5_999,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            CatState::TemperatureCheck
        );
        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::TemperatureCheck,
                Some(1_000),
                6_000,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            CatState::Idle
        );
    }

    #[test]
    fn configured_thresholds_drive_candidate_state() {
        let mut settings = AppSettings::default();
        settings.data_sorting_cpu_threshold = 15.0;
        settings.cpu_temperature_warning = 78.0;

        assert_eq!(
            candidate(
                &settings,
                &HardwareSnapshot {
                    cpu_usage_percent: Some(16.0),
                    ..Default::default()
                }
            ),
            CatState::RepairLight
        );
        assert_eq!(
            candidate(
                &settings,
                &HardwareSnapshot {
                    cpu_temperature_celsius: Some(79.0),
                    cpu_usage_percent: Some(16.0),
                    ..Default::default()
                }
            ),
            CatState::TemperatureCheck
        );
        assert_eq!(
            candidate(
                &settings,
                &HardwareSnapshot {
                    cpu_temperature_celsius: Some(79.0),
                    ..Default::default()
                }
            ),
            CatState::TemperatureCheck
        );
    }

    #[test]
    fn ordinary_state_waits_for_hold_window() {
        assert!(!should_transition(
            &CatState::MemoryCrowded,
            &CatState::Idle,
            1_000
        ));
        assert!(should_transition(
            &CatState::MemoryCrowded,
            &CatState::Idle,
            3_000
        ));
    }

    #[test]
    fn severe_state_can_override_immediately() {
        assert!(should_transition(
            &CatState::Idle,
            &CatState::TemperatureCheck,
            0
        ));
    }

    #[test]
    fn long_silent_stretch_suggests_a_break() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        // Last input 55 minutes ago → beyond the 50-min NeedsBreak threshold.
        let now = 60 * 60 * 1000;
        let last_input = Some(5 * 60 * 1000);

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                last_input,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            CatState::NeedsBreak
        );
    }

    #[test]
    fn continuous_work_past_threshold_becomes_fatigued() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        // Input is recent (continuous), and the stretch began 100 minutes ago.
        let now = 100 * 60 * 1000;
        let last_input = Some(now - 10_000);
        let continuous_since = Some(0);

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                last_input,
                continuous_since,
                None,
                None,
                None,
                None,
                None,
            ),
            CatState::Fatigued
        );
    }

    #[test]
    fn recent_input_with_short_stretch_stays_idle() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 10 * 60 * 1000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 5_000),
                Some(0),
                None,
                None,
                None,
                None,
                None,
            ),
            CatState::Idle
        );
    }

    #[test]
    fn active_focus_session_with_recent_input_is_deep_work() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 5 * 60 * 1000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 5_000),
                None,
                Some("focus-1"),
                Some(0),
                Some(25 * 60 * 1000),
                None,
                None,
            ),
            CatState::DeepWork
        );
    }

    #[test]
    fn active_focus_session_with_silent_input_is_distracted() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 5 * 60 * 1000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 120_000), // 2 min silence > 90s threshold
                None,
                Some("focus-1"),
                Some(0),
                Some(25 * 60 * 1000),
                None,
                None,
            ),
            CatState::Distracted
        );
    }

    #[test]
    fn active_focus_late_stage_becomes_fatigued() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 20 * 60 * 1000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 5_000),
                None,
                Some("focus-1"),
                Some(0),
                Some(25 * 60 * 1000),
                None,
                None,
            ),
            CatState::Fatigued
        );
    }

    #[test]
    fn completed_focus_nudge_suggests_break() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 10_000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 5_000),
                None,
                None,
                None,
                None,
                Some(&CatState::NeedsBreak),
                Some(now + 1_000),
            ),
            CatState::NeedsBreak
        );
    }

    #[test]
    fn expired_focus_nudge_is_ignored() {
        let settings = AppSettings::default();
        let snapshot = HardwareSnapshot::default();
        let now = 10_000;

        assert_eq!(
            resolve_candidate_state(
                &settings,
                &snapshot,
                &CatState::Idle,
                None,
                now,
                Some(now - 5_000),
                None,
                None,
                None,
                None,
                Some(&CatState::NeedsBreak),
                Some(now),
            ),
            CatState::Idle
        );
    }

    // --- message_for_state process-story enrichment ---

    fn proc(name: &str) -> ProcessUsageSnapshot {
        ProcessUsageSnapshot {
            pid: 1,
            name: name.to_string(),
            cpu_usage_percent: 30.0,
            memory_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_write_bytes_per_second: None,
        }
    }

    #[test]
    fn message_compiler_repair_light_is_enriched() {
        let msg = message_for_state(&CatState::RepairLight, Some(&proc("node.exe")));
        assert!(
            msg.contains("编译"),
            "expected compiler story, got: {msg}"
        );
    }

    #[test]
    fn message_compiler_repair_heavy_is_enriched() {
        let msg = message_for_state(&CatState::RepairHeavy, Some(&proc("cargo.exe")));
        assert!(msg.contains("满头大汗"));
    }

    #[test]
    fn message_browser_memory_crowded_is_enriched() {
        let msg = message_for_state(&CatState::MemoryCrowded, Some(&proc("chrome.exe")));
        assert!(msg.contains("浏览器") && msg.contains("内存"));
    }

    #[test]
    fn message_unknown_lead_falls_back_to_static() {
        let msg = message_for_state(&CatState::RepairLight, Some(&proc("explorer.exe")));
        assert_eq!(msg, "检测到轻量维护负载。");
    }

    #[test]
    fn message_no_lead_falls_back_to_static() {
        let msg = message_for_state(&CatState::RepairLight, None);
        assert_eq!(msg, "检测到轻量维护负载。");
    }

    #[test]
    fn message_temperature_state_keeps_domain_text_even_with_lead() {
        // Temperature nudge must not be masked by a process story.
        let msg =
            message_for_state(&CatState::TemperatureCheck, Some(&proc("chrome.exe")));
        assert_eq!(msg, "温度偏高，正在关注散热状态。");
    }

    #[test]
    fn message_hidden_is_empty() {
        assert_eq!(message_for_state(&CatState::Hidden, None), "");
    }
}
