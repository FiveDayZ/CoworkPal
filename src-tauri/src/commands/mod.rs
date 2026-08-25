use std::collections::BTreeMap;

use chrono::{Duration, Local};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub mod updater;

use crate::{
    achievements::{
        get_achievement_card, list_achievement_cards, load_seed_definitions,
        mark_achievement_notifications_seen as mark_notifications_seen_in_book,
        record_achievement_event, summarize_achievements, AchievementCard, AchievementSummary,
        TrackAchievementEventRequest, TrackAchievementEventResponse,
    },
    app_state::AppState,
    events::{
        ACHIEVEMENT_UNLOCKED, COCAT_INTERACTION_STATE, FOCUS_SESSION_UPDATED, NOTES_UPDATED,
        SETTINGS_UPDATED, UI_NAVIGATE_MAIN, WORKSHOP_UPDATED,
    },
    memory_release::{self, ReleaseKind, ReleaseResult},
    models::{
        build_rhythm_profile, current_timestamp_ms, generate_unique_id, today_key, AppSettings,
        AppSettingsPatch, CatState, DailyWorkAssessment, DailyWorkAssessmentSummary,
        DailyWorkAssessmentTrend, FocusSession, FocusSessionBook, FocusSessionStatus,
        HardwareSnapshot, HealthTrendReport, LastMemoryRelease, NoteBook, NoteColor, NoteKind,
        RhythmProfile, TrendRange, TodaySuggestions, WorkLogEntry, WorkLogReport, WorkshopState,
    },
    pet::FOCUS_NUDGE_HOLD_MS,
    taskbar_embed,
    window_manager,
};

#[tauri::command]
pub async fn track_achievement_event(
    request: TrackAchievementEventRequest,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TrackAchievementEventResponse, String> {
    record_achievement_event_with_state(state.inner(), &app, request).await
}

#[tauri::command]
pub async fn get_achievement_summary(
    state: State<'_, AppState>,
) -> Result<AchievementSummary, String> {
    let definitions = load_seed_definitions().map_err(|error| error.to_string())?;
    let achievements = state.achievements.read().await;
    Ok(summarize_achievements(&achievements, &definitions))
}

#[tauri::command]
pub async fn list_achievements(
    include_unlocked_hidden: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<AchievementCard>, String> {
    let definitions = load_seed_definitions().map_err(|error| error.to_string())?;
    let achievements = state.achievements.read().await;
    Ok(list_achievement_cards(
        &achievements,
        &definitions,
        include_unlocked_hidden.unwrap_or(true),
    ))
}

#[tauri::command]
pub async fn get_achievement_detail(
    achievement_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AchievementCard>, String> {
    let definitions = load_seed_definitions().map_err(|error| error.to_string())?;
    let achievements = state.achievements.read().await;
    Ok(get_achievement_card(
        &achievements,
        &definitions,
        &achievement_id,
        true,
    ))
}

#[tauri::command]
pub async fn mark_achievement_notifications_seen(
    unlock_ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<AchievementSummary, String> {
    let definitions = load_seed_definitions().map_err(|error| error.to_string())?;
    let summary = {
        let mut achievements = state.achievements.write().await;
        let changed = mark_notifications_seen_in_book(
            &mut achievements,
            unlock_ids,
            current_timestamp_ms(),
        );
        if changed > 0 {
            state.storage.save_achievements(&achievements)?;
        }
        summarize_achievements(&achievements, &definitions)
    };

    Ok(summary)
}

pub async fn record_internal_achievement_event(
    app: &AppHandle,
    event_name: &str,
    idempotency_key: String,
    payload: Value,
) -> Result<TrackAchievementEventResponse, String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Err("app state is not available".to_string());
    };

    let request = TrackAchievementEventRequest {
        event_name: event_name.to_string(),
        occurred_at: current_timestamp_ms(),
        idempotency_key,
        payload,
        source: "backend".to_string(),
    };

    record_achievement_event_with_state(state.inner(), app, request).await
}

async fn record_achievement_event_with_state(
    state: &AppState,
    app: &AppHandle,
    request: TrackAchievementEventRequest,
) -> Result<TrackAchievementEventResponse, String> {
    let definitions = load_seed_definitions().map_err(|error| error.to_string())?;
    let response = {
        let mut achievements = state.achievements.write().await;
        let response = record_achievement_event(
            &mut achievements,
            &definitions,
            request,
            current_timestamp_ms(),
            env!("CARGO_PKG_VERSION"),
        );

        if response.accepted {
            state.storage.save_achievements(&achievements)?;
        }

        response
    };

    emit_achievement_unlocks(app, &response)?;
    Ok(response)
}

fn emit_achievement_unlocks(
    app: &AppHandle,
    response: &TrackAchievementEventResponse,
) -> Result<(), String> {
    if response.unlocked.is_empty() {
        return Ok(());
    }

    for unlocked in &response.unlocked {
        app.emit(ACHIEVEMENT_UNLOCKED, unlocked)
            .map_err(|error| format!("failed to emit {ACHIEVEMENT_UNLOCKED}: {error}"))?;
    }

    emit_cocat_interaction_state(app, "achievementPop")
}

#[tauri::command]
pub async fn get_hardware_snapshot(state: State<'_, AppState>) -> Result<HardwareSnapshot, String> {
    if let Some(snapshot) = state.last_snapshot.read().await.clone() {
        return Ok(snapshot);
    }

    let snapshot = {
        // `sample()` may spawn subprocesses; run it off the async runtime.
        let adapter = state.hardware_adapter.clone();
        tokio::task::spawn_blocking(move || {
            let mut adapter = adapter
                .lock()
                .map_err(|error| format!("hardware adapter lock failed: {error}"))?;
            Ok::<HardwareSnapshot, String>(adapter.sample())
        })
        .await
        .map_err(|join_error| format!("hardware sample task failed: {join_error}"))??
    };

    *state.last_snapshot.write().await = Some(snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn update_app_settings(
    patch: AppSettingsPatch,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<AppSettings, String> {
    let launch_at_startup = patch.launch_at_startup;
    let achievement_payloads = settings_patch_achievement_payloads(&patch);

    let settings = {
        let mut settings = state.settings.write().await;
        // Snapshot before applying so we can roll back on a failed save.
        let previous = settings.clone();
        settings.apply_patch(patch);
        if let Err(error) = state.storage.save_settings(&settings) {
            *settings = previous;
            return Err(format!("failed to save settings: {error}"));
        }
        settings.clone()
    };

    // Sync the launch-at-startup registry ONLY after settings persisted
    // successfully. Previously this ran before save_settings, so a failed save
    // left the registry changed while memory/disk rolled back — a split state
    // where the app would auto-start despite showing "disabled".
    if let Some(enabled) = launch_at_startup {
        sync_launch_at_startup(enabled)?;
    }

    if let Err(error) = app.emit(SETTINGS_UPDATED, settings.clone()) {
        tracing::warn!("failed to emit {SETTINGS_UPDATED}: {error}");
    }

    taskbar_embed::sync_taskbar_monitor(&app).await;

    for payload in achievement_payloads {
        let changed_key = payload
            .get("changedKey")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let idempotency_key = format!("settings.update:{changed_key}:{}", current_timestamp_ms());
        if let Err(error) = record_internal_achievement_event(
            &app,
            "settings.update",
            idempotency_key,
            payload,
        )
        .await
        {
            tracing::warn!("failed to record settings achievement event: {error}");
        }
    }

    Ok(settings)
}

#[cfg(windows)]
pub(crate) fn sync_launch_at_startup(enabled: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "CoworkPal";

    let mut command = std::process::Command::new("reg.exe");
    command.creation_flags(CREATE_NO_WINDOW);

    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|error| format!("failed to resolve current executable: {error}"))?;
        let exe_command = format!("\"{}\"", exe_path.display());
        command.args([
            "add",
            RUN_KEY,
            "/v",
            VALUE_NAME,
            "/t",
            "REG_SZ",
            "/d",
            &exe_command,
            "/f",
        ]);
    } else {
        command.args(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"]);
    }

    let output = crate::process_util::run_command_with_timeout(
        command,
        crate::process_util::DEFAULT_SUBPROCESS_TIMEOUT,
    )
    .ok_or_else(|| "failed to update startup registry: subprocess timed out or failed".to_string())?;

    if output.status.success() || (!enabled && output.status.code() == Some(1)) {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("failed to update startup registry: {stderr}"))
}

#[cfg(not(windows))]
pub(crate) fn sync_launch_at_startup(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_workshop_state(state: State<'_, AppState>) -> Result<WorkshopState, String> {
    Ok(state.workshop.read().await.clone())
}

#[tauri::command]
pub async fn reward_cocat_interaction(
    action: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<WorkshopState, String> {
    let animation_state = match action.as_str() {
        "pet" => "pettingHearts",
        "sortParts" => "dataSorting",
        _ => return Err(format!("unknown CoCat interaction action: {action}")),
    };
    let next_workshop = state.workshop.read().await.clone();

    emit_cocat_interaction_state(&app, animation_state)?;
    if action == "pet" {
        if let Err(error) = record_internal_achievement_event(
            &app,
            "pet.click",
            format!("pet.click:{}", current_timestamp_ms()),
            serde_json::json!({ "action": action }),
        )
        .await
        {
            tracing::warn!("failed to record pet click achievement event: {error}");
        }
    }
    Ok(next_workshop)
}

/// How strongly distraction erodes the session's quality and reward.
const FOCUS_DISTRACTION_PENALTY: f64 = 0.15;
const FOCUS_MIN_QUALITY: f64 = 0.4;
/// Base parts/insight awarded per planned minute, scaled by focus_quality.
const FOCUS_PARTS_PER_MINUTE: f64 = 8.0;
const FOCUS_INSIGHT_PER_MINUTE: f64 = 0.5;

/// Pure quality score for a completed focus session, clamped to [MIN, 1.0].
/// Extracted so it can be unit-tested independently of the command path.
pub(crate) fn compute_focus_quality(distraction_count: u32) -> f64 {
    (1.0 - distraction_count as f64 * FOCUS_DISTRACTION_PENALTY)
        .max(FOCUS_MIN_QUALITY)
        .min(1.0)
}

#[tauri::command]
pub async fn start_focus_session(
    task_label: String,
    duration_minutes: u64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<FocusSessionBook, String> {
    let trimmed = task_label.trim();
    if trimmed.is_empty() {
        return Err("任务名称不能为空".to_string());
    }
    let duration_minutes = duration_minutes.clamp(5, 180);
    let duration_seconds = duration_minutes * 60;
    let now = current_timestamp_ms();
    let id = generate_unique_id("focus");

    let next_book = {
        let mut sessions = state.focus_sessions.write().await;
        // Only one active session at a time: abandon any prior active one.
        for session in sessions.sessions.iter_mut() {
            if session.status == FocusSessionStatus::Active {
                session.status = FocusSessionStatus::Abandoned;
                session.ended_at = Some(now);
            }
        }
        sessions.sessions.push(FocusSession {
            id: id.clone(),
            task_label: trimmed.to_string(),
            planned_duration_seconds: duration_seconds,
            started_at: now,
            ended_at: None,
            status: FocusSessionStatus::Active,
            distraction_count: 0,
            focus_quality: 0.0,
        });
        sessions.clone()
        // Lock released before the blocking fs::write below (S5: never hold the
        // RwLock across disk IO).
    };

    // Persist outside the lock. If the write fails, roll the in-memory state
    // back so memory and disk stay consistent, then surface the error (C1).
    if let Err(error) = state.storage.save_focus_sessions(&next_book) {
        let mut sessions = state.focus_sessions.write().await;
        sessions.sessions.retain(|s| s.id != id);
        return Err(format!("failed to save focus session: {error}"));
    }

    // Mark the active session on the cat runtime so the pet state machine can
    // enter DeepWork / Distracted. Lock acquired after focus_sessions released.
    {
        let mut runtime = state.cat_runtime.write().await;
        runtime.active_focus_session_id = Some(id);
        runtime.active_focus_started_at = Some(now);
        runtime.active_focus_planned_duration_ms = Some(duration_seconds as i64 * 1000);
        runtime.focus_nudge_state = None;
        runtime.focus_nudge_until = None;
        runtime.last_distraction_at = None;
    }

    let _ = emit_cocat_interaction_state(&app, "dataSorting");
    if let Err(error) = app.emit(FOCUS_SESSION_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {FOCUS_SESSION_UPDATED}: {error}");
    }
    Ok(next_book)
}

#[tauri::command]
pub async fn complete_focus_session(
    session_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(FocusSessionBook, WorkshopState), String> {
    let now = current_timestamp_ms();
    let (next_book, completed) = {
        let mut sessions = state.focus_sessions.write().await;
        let session = sessions
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id && s.status == FocusSessionStatus::Active)
            .ok_or_else(|| format!("no active focus session with id {session_id}"))?;

        let quality = compute_focus_quality(session.distraction_count);
        let previous_status = session.status;
        let previous_ended_at = session.ended_at;
        let previous_quality = session.focus_quality;
        session.focus_quality = quality;
        session.status = FocusSessionStatus::Completed;
        session.ended_at = Some(now);
        let snapshot = session.clone();
        (sessions.clone(), (snapshot, previous_status, previous_ended_at, previous_quality))
        // Lock released before the blocking fs::write below (S5).
    };
    let (completed, previous_status, previous_ended_at, previous_quality) = completed;

    // Persist outside the lock; roll back on failure so a failed write cannot
    // leave the session marked Completed in memory while unsaved on disk (C1).
    if let Err(error) = state.storage.save_focus_sessions(&next_book) {
        let mut sessions = state.focus_sessions.write().await;
        if let Some(session) = sessions
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
        {
            session.status = previous_status;
            session.ended_at = previous_ended_at;
            session.focus_quality = previous_quality;
        }
        return Err(format!("failed to save focus session: {error}"));
    }

    // Clear the active marker on the cat runtime.
    {
        let mut runtime = state.cat_runtime.write().await;
        if runtime.active_focus_session_id.as_deref() == Some(session_id.as_str()) {
            runtime.active_focus_session_id = None;
            runtime.active_focus_started_at = None;
            runtime.active_focus_planned_duration_ms = None;
            runtime.focus_nudge_state = Some(CatState::NeedsBreak);
            runtime.focus_nudge_until = Some(now + FOCUS_NUDGE_HOLD_MS);
        }
    }

    // Land the workshop reward, scaled by focus_quality. focus_quality is
    // clamped to [0.4, 1.0] by compute_focus_quality so it can't be NaN here,
    // but guard defensively — a non-finite award would permanently corrupt the
    // workshop economy (NaN propagates through all arithmetic and can't be
    // undone except by reset). Apply the same guard to both parts and insight.
    let planned_minutes = completed.planned_duration_seconds / 60;
    let raw_parts = planned_minutes as f64 * FOCUS_PARTS_PER_MINUTE * completed.focus_quality;
    let parts_award = if raw_parts.is_finite() && raw_parts >= 0.0 {
        raw_parts.round() as u32
    } else {
        0
    };
    let raw_insight = planned_minutes as f64 * FOCUS_INSIGHT_PER_MINUTE * completed.focus_quality;
    let insight_award = if raw_insight.is_finite() && raw_insight >= 0.0 {
        raw_insight
    } else {
        0.0
    };
    // Land the workshop reward. Unlike most write commands, this saves INSIDE
    // the workshop write lock (deliberately deviating from the usual "lock-free
    // IO" S5 pattern). complete_focus_session is a rare, user-initiated action,
    // and the monitoring pump writes workshop every ~2s — saving the reward
    // outside the lock let the pump's tick land between our mutate and our save,
    // so our stale snapshot would overwrite the pump's production (or vice
    // versa). Holding the lock across the save makes the reward atomic.
    let next_workshop = {
        let mut workshop = state.workshop.write().await;
        workshop.parts += parts_award as f64;
        workshop.insight += insight_award;
        workshop.today_parts += parts_award as f64;
        workshop.today_insight += insight_award;
        if let Err(error) = state.storage.save_workshop(&workshop) {
            // Roll back the in-memory reward so it matches the unchanged disk.
            workshop.parts -= parts_award as f64;
            workshop.insight -= insight_award;
            workshop.today_parts -= parts_award as f64;
            workshop.today_insight -= insight_award;
            // The session was already persisted as Completed (above) and
            // cat_runtime was already cleared, so the backend is in a consistent
            // "session ended" state — but without this emit the frontend never
            // learns the session ended and keeps showing it as active. Drop the
            // workshop lock before emitting/awaiting (lock-order safety).
            drop(workshop);
            let _ = app.emit(FOCUS_SESSION_UPDATED, next_book.clone());
            // The session IS completed (persisted), so record its completion
            // achievement even though the reward failed to land — otherwise the
            // user finishes a focus session but never gets credit toward focus.
            if let Err(ach_error) = record_internal_achievement_event(
                &app,
                "focus.session.completed",
                format!("focus.session:{}:{}", completed.id, now),
                serde_json::json!({
                    "taskLabel": completed.task_label,
                    "plannedDurationSeconds": completed.planned_duration_seconds,
                    "distractionCount": completed.distraction_count,
                    "focusQuality": completed.focus_quality,
                }),
            )
            .await
            {
                tracing::warn!("failed to record focus completion achievement event: {ach_error}");
            }
            return Err(format!("failed to save workshop after focus reward: {error}"));
        }
        workshop.clone()
    };

    if let Err(error) = record_internal_achievement_event(
        &app,
        "focus.session.completed",
        format!("focus.session:{}:{}", completed.id, now),
        serde_json::json!({
            "taskLabel": completed.task_label,
            "plannedDurationSeconds": completed.planned_duration_seconds,
            "distractionCount": completed.distraction_count,
            "focusQuality": completed.focus_quality,
        }),
    )
    .await
    {
        tracing::warn!("failed to record focus completion achievement event: {error}");
    }

    let _ = emit_cocat_interaction_state(&app, "celebrate");
    if let Err(error) = app.emit(FOCUS_SESSION_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {FOCUS_SESSION_UPDATED}: {error}");
    }
    if let Err(error) = app.emit(WORKSHOP_UPDATED, next_workshop.clone()) {
        tracing::warn!("failed to emit {WORKSHOP_UPDATED}: {error}");
    }
    Ok((next_book, next_workshop))
}

#[tauri::command]
pub async fn abandon_focus_session(
    session_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<FocusSessionBook, String> {
    let now = current_timestamp_ms();
    let next_book = {
        let mut sessions = state.focus_sessions.write().await;
        let session = sessions
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id && s.status == FocusSessionStatus::Active)
            .ok_or_else(|| format!("no active focus session with id {session_id}"))?;
        session.status = FocusSessionStatus::Abandoned;
        session.ended_at = Some(now);
        sessions.clone()
        // Lock released before the blocking fs::write below (S5).
    };

    if let Err(error) = state.storage.save_focus_sessions(&next_book) {
        let mut sessions = state.focus_sessions.write().await;
        if let Some(session) = sessions.sessions.iter_mut().find(|s| s.id == session_id) {
            session.status = FocusSessionStatus::Active;
            session.ended_at = None;
        }
        return Err(format!("failed to save focus session: {error}"));
    }

    {
        let mut runtime = state.cat_runtime.write().await;
        if runtime.active_focus_session_id.as_deref() == Some(session_id.as_str()) {
            runtime.active_focus_session_id = None;
            runtime.active_focus_started_at = None;
            runtime.active_focus_planned_duration_ms = None;
            runtime.focus_nudge_state = Some(CatState::Fatigued);
            runtime.focus_nudge_until = Some(now + FOCUS_NUDGE_HOLD_MS);
        }
    }

    if let Err(error) = app.emit(FOCUS_SESSION_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {FOCUS_SESSION_UPDATED}: {error}");
    }
    Ok(next_book)
}

// --- Notes & memos -------------------------------------------------------

#[tauri::command]
pub async fn get_notes(state: State<'_, AppState>) -> Result<NoteBook, String> {
    let notes = state.notes.read().await;
    Ok(notes.clone())
}

#[tauri::command]
pub async fn create_note(
    kind: NoteKind,
    title: String,
    body: String,
    memo_due_at: Option<i64>,
    color: NoteColor,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(NoteBook, String), String> {
    let now = current_timestamp_ms();
    let id = generate_unique_id("note");
    let note = crate::models::Note {
        id: id.clone(),
        kind,
        title: title.trim().to_string(),
        body,
        created_at: now,
        updated_at: now,
        pinned: false,
        archived: false,
        memo_due_at,
        color,
    };

    let next_book = {
        let mut notes = state.notes.write().await;
        notes.notes.push(note);
        notes.clone()
        // Lock released before the blocking fs::write below (S5).
    };

    // Persist outside the lock; roll back the in-memory push on failure so the
    // user is not shown a note that was never written to disk (C1).
    if let Err(error) = state.storage.save_notes(&next_book) {
        let mut notes = state.notes.write().await;
        notes.notes.retain(|n| n.id != id);
        return Err(format!("failed to save note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    // Return the new note's id alongside the book so the frontend does not have
    // to guess it by matching kind+title (which collides on duplicate titles).
    Ok((next_book, id))
}

#[tauri::command]
pub async fn update_note(
    id: String,
    title: String,
    body: String,
    memo_due_at: Option<i64>,
    color: NoteColor,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<NoteBook, String> {
    let now = current_timestamp_ms();
    let next_book = {
        let mut notes = state.notes.write().await;
        let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) else {
            return Err(format!("note {id} not found"));
        };
        // Capture previous values so we can roll back on a failed write (C1).
        let previous = (
            note.title.clone(),
            note.body.clone(),
            note.memo_due_at,
            note.color,
            note.updated_at,
        );
        note.title = title.trim().to_string();
        note.body = body;
        note.memo_due_at = memo_due_at;
        note.color = color;
        note.updated_at = now;
        let snapshot = (notes.clone(), previous);
        snapshot
        // Lock released before the blocking fs::write below (S5).
    };
    let (next_book, previous) = next_book;

    if let Err(error) = state.storage.save_notes(&next_book) {
        let (prev_title, prev_body, prev_memo, prev_color, prev_updated) = previous;
        let mut notes = state.notes.write().await;
        if let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) {
            note.title = prev_title;
            note.body = prev_body;
            note.memo_due_at = prev_memo;
            note.color = prev_color;
            note.updated_at = prev_updated;
        }
        return Err(format!("failed to save note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    Ok(next_book)
}

#[tauri::command]
pub async fn toggle_note_pinned(
    id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<NoteBook, String> {
    let next_book = {
        let mut notes = state.notes.write().await;
        let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) else {
            return Err(format!("note {id} not found"));
        };
        note.pinned = !note.pinned;
        note.updated_at = current_timestamp_ms();
        notes.clone()
        // Lock released before the blocking fs::write below (S5).
    };

    if let Err(error) = state.storage.save_notes(&next_book) {
        // Roll back the toggle so memory matches the still-old disk file (C1).
        let mut notes = state.notes.write().await;
        if let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) {
            note.pinned = !note.pinned;
        }
        return Err(format!("failed to save note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    Ok(next_book)
}

#[tauri::command]
pub async fn toggle_note_archived(
    id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<NoteBook, String> {
    let next_book = {
        let mut notes = state.notes.write().await;
        let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) else {
            return Err(format!("note {id} not found"));
        };
        note.archived = !note.archived;
        note.updated_at = current_timestamp_ms();
        notes.clone()
        // Lock released before the blocking fs::write below (S5).
    };

    if let Err(error) = state.storage.save_notes(&next_book) {
        let mut notes = state.notes.write().await;
        if let Some(note) = notes.notes.iter_mut().find(|n| n.id == id) {
            note.archived = !note.archived;
        }
        return Err(format!("failed to save note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    Ok(next_book)
}

#[tauri::command]
pub async fn delete_note(
    id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<NoteBook, String> {
    let (next_book, removed) = {
        let mut notes = state.notes.write().await;
        // Partition instead of `retain`: keeps the removed note(s) so we can
        // restore them if the subsequent write fails (C1).
        let mut removed: Vec<crate::models::Note> = Vec::new();
        let mut kept = Vec::with_capacity(notes.notes.len());
        for note in notes.notes.drain(..) {
            if note.id == id {
                removed.push(note);
            } else {
                kept.push(note);
            }
        }
        notes.notes = kept;
        (notes.clone(), removed)
        // Lock released before the blocking fs::write below (S5).
    };

    if let Err(error) = state.storage.save_notes(&next_book) {
        // Restore the removed note(s) so the list reflects the unchanged disk.
        let mut notes = state.notes.write().await;
        notes.notes.extend(removed);
        return Err(format!("failed to save note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    Ok(next_book)
}

/// Export a single note as a `.md` file. Shows a native save dialog so the
/// user picks the destination; the note body is written verbatim (it already
/// is Markdown source). Returns the chosen path on success, or null if the
/// user cancelled the dialog.
#[tauri::command]
pub async fn export_note(id: String, state: State<'_, AppState>, app: AppHandle) -> Result<Option<String>, String> {
    // Read the note under a read lock, then drop it before the blocking dialog.
    let (title, body) = {
        let notes = state.notes.read().await;
        let Some(note) = notes.notes.iter().find(|n| n.id == id) else {
            return Err(format!("note {id} not found"));
        };
        (note.title.clone(), note.body.clone())
    };

    // Default file name: the note title (sanitized) + .md, fall back to the id.
    let safe_name: String = title
        .trim()
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string();
    let default_name = if safe_name.is_empty() {
        format!("{}.md", id)
    } else {
        format!("{safe_name}.md")
    };

    // Native save dialog (blocking — run on the dialog plugin's own thread).
    let file_path = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md"])
        .set_file_name(&default_name)
        .blocking_save_file();

    let Some(file_path) = file_path else {
        // User cancelled the save dialog.
        return Ok(None);
    };
    let path = file_path.as_path().ok_or_else(|| "invalid save path".to_string())?.to_path_buf();

    std::fs::write(&path, body.as_bytes())
        .map_err(|error| format!("failed to write note file: {error}"))?;

    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Import a `.md` file as a new note. Shows a native open dialog; the file's
/// content becomes the note body (verbatim Markdown). The title is derived from
/// the first H1 heading in the file (`# Title`), falling back to the file stem.
/// Returns the id of the created note on success, or null if the user cancelled.
#[tauri::command]
pub async fn import_note(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    // Native open dialog (blocking — run on the dialog plugin's own thread).
    let file_path = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md"])
        .set_title("选择要导入的 Markdown 文件")
        .blocking_pick_file();

    let Some(file_path) = file_path else {
        // User cancelled the open dialog.
        return Ok(None);
    };
    let path = file_path
        .as_path()
        .ok_or_else(|| "invalid file path".to_string())?
        .to_path_buf();

    // Read the file content. Markdown files are UTF-8 text.
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read file: {error}"))?;

    // Derive a title: prefer the first H1 heading (`# Title`), else the file
    // stem (filename without extension). Trim and cap to keep it readable.
    let title = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("# ")
                .map(|rest| rest.trim().to_string())
                .filter(|t| !t.is_empty())
        })
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "导入的笔记".to_string());

    let now = current_timestamp_ms();
    let id = generate_unique_id("note");
    let note = crate::models::Note {
        id: id.clone(),
        kind: NoteKind::Note,
        title,
        body: content,
        created_at: now,
        updated_at: now,
        pinned: false,
        archived: false,
        memo_due_at: None,
        color: NoteColor::Default,
    };

    let next_book = {
        let mut notes = state.notes.write().await;
        notes.notes.push(note);
        notes.clone()
        // Lock released before the blocking fs::write below (S5).
    };

    if let Err(error) = state.storage.save_notes(&next_book) {
        let mut notes = state.notes.write().await;
        notes.notes.retain(|n| n.id != id);
        return Err(format!("failed to save imported note: {error}"));
    }

    if let Err(error) = app.emit(NOTES_UPDATED, next_book.clone()) {
        tracing::warn!("failed to emit {NOTES_UPDATED}: {error}");
    }
    Ok(Some(id))
}

#[tauri::command]
pub async fn get_work_log_report(
    date: Option<String>,
    state: State<'_, AppState>,
) -> Result<WorkLogReport, String> {
    let date = date.unwrap_or_else(today_key);
    let entry = state
        .work_logs
        .read()
        .await
        .entries
        .get(&date)
        .cloned()
        .unwrap_or_else(|| WorkLogEntry {
            date,
            ..Default::default()
        });

    Ok(WorkLogReport::from_entry(entry))
}

#[tauri::command]
pub async fn get_daily_work_assessment(
    date: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<DailyWorkAssessment, String> {
    let date = date.unwrap_or_else(today_key);
    let (entry, history) = {
        let work_logs = state.work_logs.read().await;
        let entry = work_logs
            .entries
            .get(&date)
            .cloned()
            .unwrap_or_else(|| WorkLogEntry {
                date: date.clone(),
                ..Default::default()
            });
        let history = build_assessment_history(&work_logs.entries, &date);

        (entry, history)
    };

    let assessment = DailyWorkAssessment::from_entry(entry, &history);
    if let Err(error) = record_internal_achievement_event(
        &app,
        "worklog.daily_generated",
        format!("worklog.daily_generated:{date}"),
        serde_json::json!({
            "date": date,
            "score": assessment.score,
            "dayType": format!("{:?}", assessment.day_type),
            "rarityTier": assessment.rarity.tier.clone(),
            "rarityScore": assessment.rarity.score,
            "titleFamily": assessment.title.family.clone(),
            "titleName": assessment.title.title.clone(),
            "titleLevel": assessment.title.level,
            "titleProgress": assessment.title.progress,
        }),
    )
    .await
    {
        tracing::warn!("failed to record daily assessment achievement event: {error}");
    }

    Ok(assessment)
}

#[tauri::command]
pub async fn get_daily_work_assessment_history(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<DailyWorkAssessmentSummary>, String> {
    let limit = limit.unwrap_or(14).clamp(1, 31);
    let work_logs = state.work_logs.read().await;

    Ok(build_calendar_assessment_summaries(&work_logs.entries, limit))
}

#[tauri::command]
pub async fn get_daily_work_assessment_trend(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<DailyWorkAssessmentTrend, String> {
    let limit = limit.unwrap_or(14).clamp(1, 31);
    let work_logs = state.work_logs.read().await;
    let summaries = build_assessment_summaries(&work_logs.entries, limit);

    Ok(DailyWorkAssessmentTrend::from_summaries(&summaries))
}

#[tauri::command]
pub async fn get_rhythm_profile(
    state: State<'_, AppState>,
) -> Result<RhythmProfile, String> {
    // Aggregate over a generous window so the rhythm has enough signal.
    let work_logs = state.work_logs.read().await;
    let summaries = build_assessment_summaries(&work_logs.entries, 90);
    Ok(build_rhythm_profile(&work_logs, &summaries))
}

#[tauri::command]
pub async fn get_health_trend(
    range: Option<String>,
    state: State<'_, AppState>,
) -> Result<HealthTrendReport, String> {
    let trend_range = parse_trend_range(range.as_deref());
    let work_logs = state.work_logs.read().await;
    Ok(HealthTrendReport::from_book(&work_logs.entries, trend_range))
}

/// Map the frontend range string ("7" | "30" | "90") to the enum, defaulting
/// to 30 days. Unknown values fall back to the default rather than erroring,
/// so a stale client cannot break the endpoint.
fn parse_trend_range(range: Option<&str>) -> TrendRange {
    match range {
        Some("7") => TrendRange::Days7,
        Some("90") => TrendRange::Days90,
        _ => TrendRange::Days30,
    }
}

#[tauri::command]
pub async fn get_today_suggestions(
    state: State<'_, AppState>,
) -> Result<TodaySuggestions, String> {
    let work_logs = state.work_logs.read().await;
    let current = state.last_snapshot.read().await.clone();
    Ok(TodaySuggestions::from_local(
        &work_logs,
        current.as_ref(),
    ))
}

fn build_assessment_summaries(
    entries: &BTreeMap<String, WorkLogEntry>,
    limit: usize,
) -> Vec<DailyWorkAssessmentSummary> {
    entries
        .iter()
        .rev()
        .filter(|(_, entry)| entry_has_assessment_signal(entry))
        .take(limit)
        .map(|(date, entry)| {
            let history = build_assessment_history(entries, date);
            DailyWorkAssessment::from_entry(entry.clone(), &history).summary(true)
        })
        .collect()
}

fn build_calendar_assessment_summaries(
    entries: &BTreeMap<String, WorkLogEntry>,
    limit: usize,
) -> Vec<DailyWorkAssessmentSummary> {
    recent_calendar_dates(limit)
        .into_iter()
        .map(|date| {
            let entry = entries
                .get(&date)
                .cloned()
                .unwrap_or_else(|| WorkLogEntry {
                    date: date.clone(),
                    ..Default::default()
                });
            let has_data = entry_has_assessment_signal(&entry);
            let history = build_assessment_history(entries, &date);

            DailyWorkAssessment::from_entry(entry, &history).summary(has_data)
        })
        .collect()
}

fn recent_calendar_dates(limit: usize) -> Vec<String> {
    let today = Local::now().date_naive();

    (0..limit)
        .map(|offset| {
            (today - Duration::days(offset as i64))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

fn build_assessment_history(
    entries: &BTreeMap<String, WorkLogEntry>,
    date: &str,
) -> Vec<WorkLogEntry> {
    entries
        .iter()
        .filter(|(entry_date, entry)| {
            entry_date.as_str() < date && entry_has_assessment_signal(entry)
        })
        .rev()
        .take(90)
        .map(|(_, entry)| entry.clone())
        .collect()
}

fn entry_has_assessment_signal(entry: &WorkLogEntry) -> bool {
    entry.sample_count > 0
        || entry.active_seconds > 0
        || entry.mouse_click_count > 0
        || entry.keyboard_press_count > 0
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn focus_quality_is_perfect_with_no_distractions() {
        assert!((compute_focus_quality(0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn focus_quality_decays_with_each_distraction() {
        // 0.15 penalty per distraction.
        assert!((compute_focus_quality(1) - 0.85).abs() < 1e-9);
        assert!((compute_focus_quality(2) - 0.70).abs() < 1e-9);
    }

    #[test]
    fn focus_quality_floors_at_minimum_after_many_distractions() {
        // 5+ distractions would go negative without the floor.
        assert!((compute_focus_quality(5) - FOCUS_MIN_QUALITY).abs() < 1e-9);
        assert!((compute_focus_quality(99) - FOCUS_MIN_QUALITY).abs() < 1e-9);
    }

    #[test]
    fn assessment_history_is_newest_first_and_skips_empty_entries() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "2026-06-20".to_string(),
            WorkLogEntry {
                date: "2026-06-20".to_string(),
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-21".to_string(),
            WorkLogEntry {
                date: "2026-06-21".to_string(),
                sample_count: 12,
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-22".to_string(),
            WorkLogEntry {
                date: "2026-06-22".to_string(),
                keyboard_press_count: 180,
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-23".to_string(),
            WorkLogEntry {
                date: "2026-06-23".to_string(),
                active_seconds: 900,
                ..Default::default()
            },
        );

        let history = build_assessment_history(&entries, "2026-06-24");

        assert_eq!(
            history.iter().map(|entry| entry.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-06-23", "2026-06-22", "2026-06-21"]
        );
    }

    #[test]
    fn assessment_history_excludes_selected_date() {
        let mut entries = BTreeMap::new();
        for date in ["2026-06-21", "2026-06-22", "2026-06-23"] {
            entries.insert(
                date.to_string(),
                WorkLogEntry {
                    date: date.to_string(),
                    sample_count: 1,
                    ..Default::default()
                },
            );
        }

        let history = build_assessment_history(&entries, "2026-06-23");

        assert_eq!(
            history.iter().map(|entry| entry.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-06-22", "2026-06-21"]
        );
    }

    #[test]
    fn assessment_summaries_are_limited_to_recent_signal_days() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "2026-06-20".to_string(),
            WorkLogEntry {
                date: "2026-06-20".to_string(),
                sample_count: 4,
                active_seconds: 240,
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-21".to_string(),
            WorkLogEntry {
                date: "2026-06-21".to_string(),
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-22".to_string(),
            WorkLogEntry {
                date: "2026-06-22".to_string(),
                keyboard_press_count: 200,
                ..Default::default()
            },
        );
        entries.insert(
            "2026-06-23".to_string(),
            WorkLogEntry {
                date: "2026-06-23".to_string(),
                sample_count: 8,
                active_seconds: 600,
                ..Default::default()
            },
        );

        let summaries = build_assessment_summaries(&entries, 2);

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].date, "2026-06-23");
        assert_eq!(summaries[1].date, "2026-06-22");
    }

    #[test]
    fn calendar_assessment_summaries_include_empty_dates() {
        let dates = recent_calendar_dates(3);
        let mut entries = BTreeMap::new();
        entries.insert(
            dates[1].clone(),
            WorkLogEntry {
                date: dates[1].clone(),
                sample_count: 5,
                active_seconds: 300,
                ..Default::default()
            },
        );

        let summaries = build_calendar_assessment_summaries(&entries, 3);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0].date, dates[0]);
        assert!(!summaries[0].has_data);
        assert_eq!(summaries[1].date, dates[1]);
        assert!(summaries[1].has_data);
        assert_eq!(summaries[2].date, dates[2]);
        assert!(!summaries[2].has_data);
    }
}

#[tauri::command]
pub async fn toggle_production_paused(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<AppSettings, String> {
    // Read-modify-write under a SINGLE write lock. The previous version read the
    // current value under a read lock, released it, then called
    // update_app_settings (which re-acquires the write lock) — leaving a window
    // in which two rapid toggles could both read `false` and both write `true`,
    // so the user's two clicks net to "paused" instead of "toggled". Doing the
    // flip inside one critical section makes the toggle atomic.
    //
    // Note: save_settings is called INSIDE the lock here, which deviates from
    // the "lock-free IO" (S5) pattern used by other settings writes. This is
    // intentional: splitting the save out would reintroduce the TOCTOU above,
    // and toggle is a rare user action so blocking settings readers for the
    // duration of one small JSON write is acceptable.
    let settings = {
        let mut settings = state.settings.write().await;
        settings.is_production_paused = !settings.is_production_paused;
        state.storage.save_settings(&settings)?;
        settings.clone()
    };

    if let Err(error) = app.emit(SETTINGS_UPDATED, settings.clone()) {
        tracing::warn!("failed to emit {SETTINGS_UPDATED}: {error}");
    }

    taskbar_embed::sync_taskbar_monitor(&app).await;

    Ok(settings)
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> Result<(), String> {
    window_manager::show_window(&app, "main", true).await
}

#[tauri::command]
pub async fn show_main_route(route: String, app: AppHandle) -> Result<(), String> {
    window_manager::show_window(&app, "main", true).await?;
    app.emit(UI_NAVIGATE_MAIN, route)
        .map_err(|error| format!("failed to emit {UI_NAVIGATE_MAIN}: {error}"))
}

#[tauri::command]
pub async fn hide_main_window(app: AppHandle) -> Result<(), String> {
    window_manager::hide_window(&app, "main").await
}

#[tauri::command]
pub async fn show_pet_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    window_manager::show_window(&app, "pet", false).await?;
    update_app_settings(
        AppSettingsPatch {
            is_cat_visible: Some(true),
            ..Default::default()
        },
        state,
        app,
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn hide_pet_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    window_manager::hide_window(&app, "pet").await?;
    window_manager::hide_window(&app, "pet-panel").await?;
    update_app_settings(
        AppSettingsPatch {
            is_cat_visible: Some(false),
            ..Default::default()
        },
        state,
        app,
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn toggle_monitor_bar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let is_monitor_bar_visible = window_manager::toggle_window(&app, "monitor-bar").await?;

    update_app_settings(
        AppSettingsPatch {
            is_monitor_bar_visible: Some(is_monitor_bar_visible),
            ..Default::default()
        },
        state,
        app.clone(),
    )
    .await
    .map(|_| ())?;

    if is_monitor_bar_visible {
        record_monitor_bar_open(&app).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn show_monitor_bar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    window_manager::show_window(&app, "monitor-bar", false).await?;
    update_app_settings(
        AppSettingsPatch {
            is_monitor_bar_visible: Some(true),
            ..Default::default()
        },
        state,
        app.clone(),
    )
    .await
    .map(|_| ())?;

    record_monitor_bar_open(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn hide_monitor_bar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    window_manager::hide_window(&app, "monitor-bar").await?;
    update_app_settings(
        AppSettingsPatch {
            is_monitor_bar_visible: Some(false),
            ..Default::default()
        },
        state,
        app,
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn show_pet_panel(app: AppHandle) -> Result<(), String> {
    window_manager::show_window(&app, "pet-panel", true).await?;
    emit_cocat_interaction_state(&app, "panelOpen")?;
    record_pet_panel_open(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn hide_pet_panel(app: AppHandle) -> Result<(), String> {
    window_manager::hide_window(&app, "pet-panel").await
}

#[tauri::command]
pub async fn toggle_pet_panel(app: AppHandle) -> Result<(), String> {
    let is_visible = window_manager::toggle_window(&app, "pet-panel").await?;
    emit_cocat_interaction_state(
        &app,
        if is_visible {
            "panelOpen"
        } else {
            "panelClose"
        },
    )?;
    if is_visible {
        record_pet_panel_open(&app).await;
    }
    Ok(())
}

fn emit_cocat_interaction_state(app: &AppHandle, state: &'static str) -> Result<(), String> {
    app.emit(COCAT_INTERACTION_STATE, state)
        .map_err(|error| format!("failed to emit {COCAT_INTERACTION_STATE}: {error}"))
}

#[tauri::command]
pub async fn save_window_position(
    window_label: String,
    x: f64,
    y: f64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    window_manager::save_window_position(&app, &state, &window_label, x, y).await
}

#[tauri::command]
pub async fn exit_app(app: AppHandle) -> Result<(), String> {
    crate::IS_EXITING.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn update_workshop_state(
    workshop: WorkshopState,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<WorkshopState, String> {
    // Reject obviously invalid state from the frontend: a non-positive level or
    // negative/non-finite resources have no business meaning and would corrupt
    // the economy (NaN would bypass the monitoring NaN guard since it writes
    // directly, not via apply_tick).
    if workshop.workshop_level < 1 {
        return Err("invalid workshop state: level must be >= 1".to_string());
    }
    if !workshop.parts.is_finite()
        || !workshop.insight.is_finite()
        || !workshop.today_parts.is_finite()
        || !workshop.today_insight.is_finite()
    {
        return Err("invalid workshop state: resources must be finite numbers".to_string());
    }
    if workshop.parts < 0.0
        || workshop.insight < 0.0
        || workshop.today_parts < 0.0
        || workshop.today_insight < 0.0
    {
        return Err("invalid workshop state: resources must not be negative".to_string());
    }
    // Capture previous_workshop and apply the new value under the SAME write
    // lock. Reading it separately (read lock, release, write lock) left a
    // TOCTOU window where a concurrent writer could change workshop between
    // the read and the write, and a failed-save rollback would then restore a
    // stale `previous_workshop`, clobbering that concurrent write.
    let (previous_workshop, next_workshop) = {
        let mut w = state.workshop.write().await;
        let previous = w.clone();
        // MERGE instead of wholesale overwrite (*w = workshop). The frontend
        // sends a snapshot of the workshop it last saw, but the monitoring pump
        // updates several fields every ~2s (total_online_seconds, today_parts,
        // today_insight, last_production_time, last_daily_reset_date). A full
        // overwrite would clobber those with stale values from the snapshot,
        // causing resources/on-line time to jump backwards after each upgrade —
        // which in turn made parts hover right around a cost threshold, so the
        // user's upgrade attempts kept failing the resource check and needed
        // many retries. Preserve the pump-maintained fields; only accept the
        // economy decisions the frontend legitimately owns.
        w.parts = workshop.parts;
        w.insight = workshop.insight;
        w.workshop_level = workshop.workshop_level;
        w.cat_affinity_level = workshop.cat_affinity_level;
        w.module_levels = workshop.module_levels.clone();
        // total_online_seconds / today_parts / today_insight /
        // last_production_time / last_daily_reset_date stay as the pump left
        // them. (today_parts/insight SHOULD drop when resources are spent on an
        // upgrade, but they are cumulative "earned today" counters, not
        // spendable balances — so keeping them is correct.)
        (previous, w.clone())
        // Lock released before the blocking fs::write below (S5).
    };

    // Persist outside the lock; roll back to the prior state on failure so the
    // in-memory workshop never diverges from disk (C1/S6).
    if let Err(error) = state.storage.save_workshop(&next_workshop) {
        let mut w = state.workshop.write().await;
        *w = previous_workshop.clone();
        return Err(format!("failed to save workshop: {error}"));
    }

    if let Err(error) = app.emit(WORKSHOP_UPDATED, next_workshop.clone()) {
        tracing::warn!("failed to emit {WORKSHOP_UPDATED}: {error}");
    }
    record_workshop_upgrade_events(&app, &previous_workshop, &next_workshop).await;
    Ok(next_workshop)
}

fn settings_patch_achievement_payloads(patch: &AppSettingsPatch) -> Vec<Value> {
    let mut payloads = Vec::new();

    macro_rules! push_bool {
        ($field:ident, $key:literal) => {
            if let Some(value) = patch.$field {
                payloads.push(serde_json::json!({
                    "changedKey": $key,
                    "value": value,
                }));
            }
        };
    }

    push_bool!(show_monitor_data_in_taskbar, "showMonitorDataInTaskbar");
    push_bool!(enable_low_power_mode, "enableLowPowerMode");
    push_bool!(enable_static_cat_mode, "enableStaticCatMode");
    push_bool!(enable_pet_bubble, "enablePetBubble");
    push_bool!(enable_sound, "enableSound");

    if let Some(theme_name) = &patch.theme_name {
        payloads.push(serde_json::json!({
            "changedKey": "themeName",
            "themeName": theme_name,
        }));
    }
    if let Some(metrics) = &patch.visible_monitor_metrics {
        payloads.push(serde_json::json!({
            "changedKey": "visibleMonitorMetrics",
            "visibleMetricCount": metrics.len(),
        }));
    }

    payloads
}

async fn record_monitor_bar_open(app: &AppHandle) {
    if let Err(error) = record_internal_achievement_event(
        app,
        "monitor_bar.open",
        format!("monitor_bar.open:{}", current_timestamp_ms()),
        serde_json::json!({}),
    )
    .await
    {
        tracing::warn!("failed to record monitor bar achievement event: {error}");
    }
}

async fn record_pet_panel_open(app: &AppHandle) {
    if let Err(error) = record_internal_achievement_event(
        app,
        "pet.panel.open",
        format!("pet.panel.open:{}", current_timestamp_ms()),
        serde_json::json!({}),
    )
    .await
    {
        tracing::warn!("failed to record pet panel achievement event: {error}");
    }
}

async fn record_workshop_upgrade_events(
    app: &AppHandle,
    previous: &WorkshopState,
    next: &WorkshopState,
) {
    if next.workshop_level > previous.workshop_level {
        if let Err(error) = record_internal_achievement_event(
            app,
            "workshop.level_up",
            format!("workshop.level_up:{}", current_timestamp_ms()),
            serde_json::json!({
                "fromLevel": previous.workshop_level,
                "toLevel": next.workshop_level,
            }),
        )
        .await
        {
            tracing::warn!("failed to record workshop level achievement event: {error}");
        }
    }

    for (module_key, previous_parts, next_parts, previous_process, next_process) in [
        (
            "cpu",
            previous.module_levels.cpu.parts,
            next.module_levels.cpu.parts,
            previous.module_levels.cpu.process,
            next.module_levels.cpu.process,
        ),
        (
            "gpu",
            previous.module_levels.gpu.parts,
            next.module_levels.gpu.parts,
            previous.module_levels.gpu.process,
            next.module_levels.gpu.process,
        ),
        (
            "ram",
            previous.module_levels.ram.parts,
            next.module_levels.ram.parts,
            previous.module_levels.ram.process,
            next.module_levels.ram.process,
        ),
        (
            "network",
            previous.module_levels.network.parts,
            next.module_levels.network.parts,
            previous.module_levels.network.process,
            next.module_levels.network.process,
        ),
        (
            "temperature",
            previous.module_levels.temperature.parts,
            next.module_levels.temperature.parts,
            previous.module_levels.temperature.process,
            next.module_levels.temperature.process,
        ),
        (
            "disk",
            previous.module_levels.disk.parts,
            next.module_levels.disk.parts,
            previous.module_levels.disk.process,
            next.module_levels.disk.process,
        ),
    ] {
        if next_parts > previous_parts {
            record_module_upgrade_event(app, module_key, "parts", previous_parts, next_parts).await;
        }
        if next_process > previous_process {
            record_module_upgrade_event(app, module_key, "process", previous_process, next_process)
                .await;
        }
    }
}

async fn record_module_upgrade_event(
    app: &AppHandle,
    module_key: &str,
    track: &str,
    from_level: u32,
    to_level: u32,
) {
    if let Err(error) = record_internal_achievement_event(
        app,
        "workshop.module_upgrade",
        format!("workshop.module_upgrade:{module_key}:{track}:{}", current_timestamp_ms()),
        serde_json::json!({
            "moduleKey": module_key,
            "track": track,
            "fromLevel": from_level,
            "toLevel": to_level,
        }),
    )
    .await
    {
        tracing::warn!("failed to record module upgrade achievement event: {error}");
    }
}

// ---------------------------------------------------------------------------
// Memory release
// ---------------------------------------------------------------------------

/// Live system memory snapshot shown in the settings card so users can pick a
/// sensible auto-release threshold.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub used_gib: f64,
    pub load_percent: u32,
}

#[tauri::command]
pub async fn get_memory_status() -> Result<MemoryStatus, String> {
    let Some(pressure) = memory_release::sysinfo_query::sample_pressure() else {
        return Err("memory status unavailable on this platform".to_string());
    };
    Ok(MemoryStatus {
        total_bytes: pressure.total_bytes,
        used_bytes: pressure.used_bytes,
        used_gib: pressure.used_gib(),
        load_percent: pressure.load_percent,
    })
}

#[tauri::command]
pub async fn trigger_memory_release(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ReleaseResult, String> {
    perform_release(&state, &app, ReleaseKind::ManualFull).await
}

/// Shared entry point for manual and auto triggers. Runs the appropriate tier,
/// persists `memory_last_release`, and emits `MEMORY_RELEASE_COMPLETED`.
pub async fn perform_release(
    state: &State<'_, AppState>,
    app: &AppHandle,
    kind: ReleaseKind,
) -> Result<ReleaseResult, String> {
    let result = match kind {
        ReleaseKind::ManualFull => memory_release::run_full_clean_via_helper().await,
        ReleaseKind::AutoLight => {
            let released = memory_release::run_light_clean();
            ReleaseResult::light(released)
        }
    };

    // Persist the last-release record (only when something was actually freed;
    // a 0-byte release isn't worth surfacing as "last release").
    if result.released_bytes > 0 {
        let patch = AppSettingsPatch {
            memory_last_release: Some(LastMemoryRelease {
                timestamp_ms: current_timestamp_ms(),
                released_bytes: result.released_bytes,
                full_tier: result.full_tier,
            }),
            ..Default::default()
        };
        let settings = {
            let mut settings = state.settings.write().await;
            settings.apply_patch(patch);
            state.storage.save_settings(&settings)?;
            settings.clone()
        };
        app.emit(SETTINGS_UPDATED, settings)
            .map_err(|error| format!("failed to emit {SETTINGS_UPDATED}: {error}"))?;
    }

    memory_release::emit_result(app, &result);

    // Trigger the one-shot Free_Memory CoCat animation so the user sees the
    // pet react to every release (manual, tray, or auto-threshold).
    let _ = emit_cocat_interaction_state(app, "freeMemory");

    Ok(result)
}

/// Show the shared context menu as a native popup on the taskbar monitor window.
///
/// The taskbar window is only ~36px tall and embedded as a WS_CHILD of the
/// shell taskbar, so an HTML overlay menu would be clipped by the window's
/// rect. A native popup menu (Win32 `TrackPopupMenuEx` under the hood) is not
/// bound by the window rect, so it displays fully above the taskbar — matching
/// how the tray icon's own right-click menu behaves.
///
/// Clicks are dispatched by the same `on_menu_event` handler registered on the
/// tray icon (Tauri routes *all* menu events there), so item ids match the tray
/// menu exactly.
#[tauri::command]
pub async fn show_taskbar_context_menu(app: AppHandle) -> Result<(), String> {
    // Build the menu and resolve the window on the async runtime, then run the
    // blocking native popup off-thread so it doesn't stall the async runtime.
    let menu = crate::tray::build_shared_menu(&app)
        .await
        .map_err(|error| format!("failed to build taskbar context menu: {error}"))?;
    let window = app
        .get_webview_window("taskbar-monitor")
        .ok_or_else(|| "taskbar-monitor window not found".to_string())?;

    tokio::task::spawn_blocking(move || {
        // popup_menu blocks until the user clicks an item or dismisses; that's
        // why it runs in spawn_blocking. It pops up at the cursor position.
        window
            .popup_menu(&menu)
            .map_err(|error| format!("failed to popup taskbar context menu: {error}"))
    })
    .await
    .map_err(|join_error| format!("taskbar menu task failed: {join_error}"))??;

    Ok(())
}
