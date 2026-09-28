use chrono::{Datelike, Local, TimeZone};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    achievements::runtime::{
        ensure_weekly_goals_with_progress, local_iso_week, PendingReward, WeeklyGoals, WeeklyProgress,
    },
    achievements::seed_definitions,
    app_state::AppState,
    commands::{compute_focus_quality, mutate_and_save_workshop, record_internal_achievement_event_buffered},
    events::{ACHIEVEMENT_PROGRESS_UPDATED, FOCUS_SESSION_UPDATED, REWARD_GRANTED, WORKSHOP_UPDATED},
    models::{date_key_from_timestamp, CatState, FocusSession, FocusSessionBook, FocusSessionStatus,
        RewardAmount, RewardReceipt, WorkLogBook, WorkshopState},
    pet::FOCUS_NUDGE_HOLD_MS,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardNotice {
    reward_id: String,
    title: String,
    reward: RewardAmount,
}

pub fn focus_reward(session: &FocusSession) -> RewardAmount {
    let seconds = (session.credited_duration_ms / 1000).min(session.planned_duration_seconds);
    if seconds < 5 * 60 {
        return RewardAmount::default();
    }
    let minutes = seconds as f64 / 60.0;
    let quality = compute_focus_quality(session.distraction_count);
    RewardAmount { parts: minutes * 8.0 * quality, insight: minutes * 0.5 * quality,
        affinity_experience: 0 }
}

pub fn apply_reward_once(
    workshop: &mut WorkshopState,
    key: &str,
    intent: &PendingReward,
    qualified_focus: bool,
    now: i64,
) -> Result<Option<RewardReceipt>, String> {
    if workshop.reward_receipts.contains_key(key) {
        return Ok(None);
    }
    let mut reward = intent.reward.clone();
    if key.is_empty() || !reward.parts.is_finite() || !reward.insight.is_finite()
        || !(0.0..=1440.0).contains(&reward.parts)
        || !(0.0..=90.0).contains(&reward.insight) || reward.affinity_experience > 10
    {
        return Err("奖励数据无效".to_string());
    }
    if qualified_focus {
        let earned_date = date_key_from_timestamp(intent.earned_at);
        let paid_today = workshop.reward_receipts.iter().filter(|(id, receipt)| {
            id.starts_with("focus:") && receipt.reward.affinity_experience > 0
                && date_key_from_timestamp(receipt.earned_at) == earned_date
        }).count();
        reward.affinity_experience = if paid_today < 2 { 5 } else { 0 };
    }
    let mut next = workshop.clone();
    let today = date_key_from_timestamp(now);
    if next.last_daily_reset_date != today {
        next.today_parts = 0.0;
        next.today_insight = 0.0;
        next.last_daily_reset_date = today;
    }
    next.parts += reward.parts;
    next.insight += reward.insight;
    next.today_parts += reward.parts;
    next.today_insight += reward.insight;
    if [next.parts, next.insight, next.today_parts, next.today_insight]
        .iter().any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("工坊余额无效，奖励尚未发放".to_string());
    }
    next.affinity_experience = next.affinity_experience.saturating_add(reward.affinity_experience);
    next.cat_affinity_level = next.cat_affinity_level.saturating_add(next.affinity_experience / 100);
    next.affinity_experience %= 100;
    let receipt = RewardReceipt { title: intent.title.clone(), reward,
        earned_at: intent.earned_at, paid_at: now };
    next.reward_receipts.insert(key.to_string(), receipt.clone());
    *workshop = next;
    Ok(Some(receipt))
}

async fn grant(
    state: &AppState, app: &AppHandle, key: &str, intent: &PendingReward, qualified: bool, now: i64,
) -> Result<RewardReceipt, String> {
    if let Some(receipt) = state.workshop.read().await.reward_receipts.get(key) {
        return Ok(receipt.clone());
    }
    let mut notice = None;
    let (_, next) = mutate_and_save_workshop(state, |workshop| {
        notice = apply_reward_once(workshop, key, intent, qualified, now)?;
        Ok(())
    }).await?;
    if let Some(receipt) = notice {
        let _ = app.emit(WORKSHOP_UPDATED, &next);
        if receipt.reward != RewardAmount::default() {
            let _ = app.emit(REWARD_GRANTED, RewardNotice {
                reward_id: key.to_string(), title: receipt.title.clone(), reward: receipt.reward,
            });
            let _ = crate::commands::emit_cocat_interaction_state(app, "celebrate");
        }
    }
    next.reward_receipts.get(key).cloned().ok_or_else(|| "奖励凭证缺失".to_string())
}

pub fn weekly_progress(
    now: i64, logs: &WorkLogBook, focus: &FocusSessionBook, workshop: &WorkshopState,
) -> WeeklyProgress {
    let date = Local.timestamp_millis_opt(now).single().unwrap_or_else(Local::now).date_naive();
    let start = date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64);
    let end = start + chrono::Duration::days(7);
    let in_week = |value: &str| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .is_ok_and(|day| day >= start && day < end);
    let entries = logs.entries.iter().filter(|(day, _)| in_week(day)).map(|(_, entry)| entry);
    let mut progress = WeeklyProgress::default();
    for entry in entries {
        progress.online_seconds += entry.active_seconds as f64;
        if entry.sample_count > 0 {
            progress.report_days += 1;
        }
    }
    progress.qualified_focus_count = focus.sessions.iter().filter(|session| {
        session.is_qualified_completion()
            && session.ended_at.is_some_and(|ended| in_week(&date_key_from_timestamp(ended)))
    }).count() as u32;
    progress.order_count = workshop.completed_order_ids.iter().filter(|id| {
        id.split_once(':').is_some_and(|(day, _)| in_week(day))
    }).count() as u32;
    progress
}

pub fn queue_weekly_rewards(
    book: &mut crate::achievements::AchievementBook, goals: &WeeklyGoals,
    workshop: &WorkshopState, now: i64,
) -> bool {
    let mut changed = false;
    let mut queue = |key: String, title: String, reward: RewardAmount| {
        if !workshop.reward_receipts.contains_key(&key) && !book.pending_rewards.contains_key(&key) {
            book.pending_rewards.insert(key, PendingReward { title, reward, earned_at: now });
            changed = true;
        }
    };
    for goal in goals.goals.iter().filter(|goal| goal.is_complete) {
        queue(format!("week:{}:{}", goals.week_key, goal.goal_id), goal.title.clone(), goal.reward.clone());
    }
    if goals.goals.len() == 3 && goals.goals.iter().all(|goal| goal.is_complete) {
        queue(format!("week:{}:all", goals.week_key), "本周目标全部完成".to_string(), goals.bonus_reward.clone());
    }
    changed
}

pub async fn weekly_goals(state: &AppState, app: &AppHandle, now: i64) -> Result<WeeklyGoals, String> {
    let _guard = state.reward_lock.lock().await;
    settle_weekly_locked(state, app, now).await
}

fn queue_previous_week_rewards(
    book: &mut crate::achievements::AchievementBook, workshop: &WorkshopState,
    logs: &WorkLogBook, focus: &FocusSessionBook, now: i64,
) -> Result<bool, String> {
    let Some(plan) = book.weekly_goal_plan.clone().filter(|plan| {
        plan.week_key != local_iso_week(now).0 && plan.goal_keys.iter().all(|key| {
            matches!(key.as_str(), "companion-120m" | "qualified-focus-2" | "weekly-action")
        })
    }) else { return Ok(false); };
    // 仅重算有日期证据的新目标；旧累计计数无法证明增长属于上周，沿用已保存的待发奖记录。
    let progress = weekly_progress(plan.generated_at, logs, focus, workshop);
    let (goals, changed) = ensure_weekly_goals_with_progress(
        book, seed_definitions().map_err(|error| error.to_string())?, plan.generated_at, &progress);
    Ok(queue_weekly_rewards(book, &goals, workshop, plan.generated_at) || changed)
}

async fn settle_weekly_locked(state: &AppState, app: &AppHandle, now: i64) -> Result<WeeklyGoals, String> {
    let logs = state.work_logs.read().await;
    let focus = state.focus_sessions.read().await;
    let workshop = state.workshop.read().await;
    let definitions = seed_definitions().map_err(|error| error.to_string())?;
    let progress = weekly_progress(now, &logs, &focus, &workshop);
    let (mut goals, pending) = {
        let mut book = state.achievements.write().await;
        let previous_plan = book.weekly_goal_plan.clone();
        let previous_pending = book.pending_rewards.clone();
        let mut changed = queue_previous_week_rewards(&mut book, &workshop, &logs, &focus, now)?;
        let (goals, plan_changed) = ensure_weekly_goals_with_progress(&mut book, definitions, now, &progress);
        changed |= plan_changed;
        changed |= queue_weekly_rewards(&mut book, &goals, &workshop, now);
        if changed {
            if let Err(error) = state.storage.save_achievements(&book) {
                book.weekly_goal_plan = previous_plan;
                book.pending_rewards = previous_pending;
                return Err(format!("周奖励尚未保存: {error}"));
            }
        }
        (goals, book.pending_rewards.clone())
    };
    drop(logs);
    drop(focus);
    drop(workshop);
    for (key, intent) in pending {
        if let Err(error) = grant(state, app, &key, &intent, false, now).await {
            tracing::warn!("weekly reward remains pending: {error}");
            continue;
        }
        let mut book = state.achievements.write().await;
        let previous = book.clone();
        book.pending_rewards.remove(&key);
        if let Err(error) = state.storage.save_achievements(&book) {
            *book = previous;
            tracing::warn!("reward receipt saved; pending queue acknowledgement will retry: {error}");
        }
    }
    let workshop = state.workshop.read().await;
    for goal in &mut goals.goals {
        goal.reward_paid = workshop.reward_receipts.contains_key(&format!("week:{}:{}", goals.week_key, goal.goal_id));
    }
    goals.bonus_paid = workshop.reward_receipts.contains_key(&format!("week:{}:all", goals.week_key));
    Ok(goals)
}

pub async fn complete_focus(
    state: &AppState, app: &AppHandle, id: &str, now: i64,
) -> Result<(FocusSessionBook, WorkshopState), String> {
    let _guard = state.reward_lock.lock().await;
    complete_focus_locked(state, app, id, now).await?;
    if let Err(error) = settle_weekly_locked(state, app, now).await {
        tracing::warn!("focus saved; weekly settlement will retry: {error}");
    }
    Ok((state.focus_sessions.read().await.clone(), state.workshop.read().await.clone()))
}

async fn complete_focus_locked(state: &AppState, app: &AppHandle, id: &str, now: i64) -> Result<(), String> {
    let completed = {
        let mut book = state.focus_sessions.write().await;
        let previous = book.clone();
        let session = book.sessions.iter_mut().find(|session| session.id == id)
            .ok_or_else(|| "专注会话不存在".to_string())?;
        if session.status == FocusSessionStatus::Abandoned || session.reward_version == 0 {
            return Err("该会话不支持奖励结算".to_string());
        }
        if session.status == FocusSessionStatus::Active {
            session.credit_tick(now);
            session.status = FocusSessionStatus::Completed;
            session.ended_at = Some(now);
            session.focus_quality = compute_focus_quality(session.distraction_count);
            session.production_multiplier = 1.0;
            session.reward = Some(focus_reward(session));
            if let Err(error) = state.storage.save_focus_sessions(&book) {
                *book = previous;
                return Err(format!("专注完成记录尚未保存: {error}"));
            }
        }
        book.sessions.iter().find(|session| session.id == id).unwrap().clone()
    };
    {
        let mut runtime = state.cat_runtime.write().await;
        if runtime.active_focus_session_id.as_deref() == Some(id) {
            runtime.active_focus_session_id = None;
            runtime.active_focus_started_at = None;
            runtime.active_focus_planned_duration_ms = None;
            runtime.focus_nudge_state = Some(CatState::NeedsBreak);
            runtime.focus_nudge_until = Some(now + FOCUS_NUDGE_HOLD_MS);
        }
    }
    let _ = app.emit(FOCUS_SESSION_UPDATED, state.focus_sessions.read().await.clone());
    let receipt = if completed.reward_paid {
        None
    } else {
        Some(grant(state, app, &format!("focus:{id}"), &PendingReward {
            title: "专注完成奖励".to_string(), reward: focus_reward(&completed),
            earned_at: completed.ended_at.unwrap_or(now),
        }, completed.is_qualified_completion(), now).await?)
    };
    if !completed.achievement_recorded && completed.is_qualified_completion() {
        record_internal_achievement_event_buffered(
            app, "focus.session.completed", format!("focus.session:{id}"),
            completed.ended_at.unwrap_or(now), serde_json::json!({
                "taskLabel": completed.task_label,
                "plannedDurationSeconds": completed.planned_duration_seconds,
                "actualDurationSeconds": completed.credited_duration_ms / 1000,
                "distractionCount": completed.distraction_count,
                "focusQuality": completed.focus_quality,
            }),
        ).await?;
        crate::persistence::flush_achievements(state).await?;
    }
    let mut book = state.focus_sessions.write().await;
    let session = book.sessions.iter_mut().find(|session| session.id == id).unwrap();
    if let Some(receipt) = receipt {
        session.reward = Some(receipt.reward);
    }
    session.reward_paid = true;
    session.achievement_recorded = true;
    state.persistence.mark_focus_sessions_dirty();
    if let Err(error) = state.storage.save_focus_sessions(&book) {
        tracing::warn!("focus acknowledgement remains dirty; receipt prevents duplicate payment: {error}");
    }
    let _ = app.emit(FOCUS_SESSION_UPDATED, book.clone());
    let _ = app.emit(ACHIEVEMENT_PROGRESS_UPDATED, ());
    Ok(())
}

pub async fn tick(app: &AppHandle, now: i64) {
    let state = app.state::<AppState>();
    let _guard = state.reward_lock.lock().await;
    let (done, pending, book) = {
        let mut book = state.focus_sessions.write().await;
        let mut done = None;
        if let Some(session) = book.sessions.iter_mut().find(|s| s.status == FocusSessionStatus::Active) {
            session.credit_tick(now);
            if session.credited_duration_ms >= session.planned_duration_seconds.saturating_mul(1000) {
                done = Some(session.id.clone());
            }
            state.persistence.mark_focus_sessions_dirty();
        }
        let pending = book.sessions.iter().filter(|s| s.reward_version > 0
            && s.status == FocusSessionStatus::Completed && (!s.reward_paid || !s.achievement_recorded))
            .map(|s| s.id.clone()).collect::<Vec<_>>();
        (done, pending, book.clone())
    };
    if book.active_session().is_some() {
        let _ = app.emit(FOCUS_SESSION_UPDATED, book);
    }
    for id in done.into_iter().chain(pending) {
        if let Err(error) = complete_focus_locked(&state, app, &id, now).await {
            tracing::warn!("focus settlement will retry: {error}");
        }
    }
    if let Err(error) = settle_weekly_locked(&state, app, now).await {
        tracing::warn!("weekly settlement will retry: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::achievements::{AchievementBook, seed_definitions};

    fn session(minutes: u64) -> FocusSession {
        FocusSession { id: "session".into(), planned_duration_seconds: minutes * 60,
            reward_version: 1, credited_duration_ms: minutes * 60_000,
            status: FocusSessionStatus::Completed, ..Default::default() }
    }

    #[test]
    fn focus_amount_uses_only_credited_time_and_quality() {
        let mut focus = session(25);
        assert_eq!(focus_reward(&focus).parts, 200.0);
        assert_eq!(focus_reward(&focus).insight, 12.5);
        focus.distraction_count = 2;
        assert!((focus_reward(&focus).parts - 140.0).abs() < 1e-9);
        focus.credited_duration_ms = 299_999;
        assert_eq!(focus_reward(&focus), RewardAmount::default());
        focus.credited_duration_ms = 600_000;
        assert!(!focus.is_qualified_completion());
    }

    #[test]
    fn reward_receipt_deduplicates_and_daily_affinity_caps_at_two() {
        let mut workshop = WorkshopState::default();
        let initial = workshop.parts;
        let intent = PendingReward { title: "专注".into(), reward: focus_reward(&session(25)), earned_at: 1000 };
        for id in ["focus:a", "focus:b", "focus:c"] {
            apply_reward_once(&mut workshop, id, &intent, true, 1000).unwrap();
        }
        assert_eq!(workshop.parts, initial + 600.0);
        assert_eq!(workshop.affinity_experience, 10);
        assert!(apply_reward_once(&mut workshop, "focus:a", &intent, true, 1000).unwrap().is_none());
        assert_eq!(workshop.parts, initial + 600.0);
        let restored: WorkshopState = serde_json::from_str(&serde_json::to_string(&workshop).unwrap()).unwrap();
        assert_eq!(restored.reward_receipts.len(), 3);
    }

    #[test]
    fn invalid_amount_does_not_mutate_balance_or_receipts() {
        let mut workshop = WorkshopState::default();
        let initial = serde_json::to_value(&workshop).unwrap();
        let intent = PendingReward { title: "invalid".into(), reward: RewardAmount {
            parts: f64::NAN, ..Default::default() }, earned_at: 0 };
        assert!(apply_reward_once(&mut workshop, "focus:bad", &intent, false, 0).is_err());
        assert_eq!(serde_json::to_value(workshop).unwrap(), initial);
    }

    #[test]
    fn all_weekly_rewards_total_480_30_20_and_queue_survives_restart() {
        let mut book = AchievementBook::default();
        let progress = WeeklyProgress { online_seconds: 7200.0, qualified_focus_count: 2,
            report_days: 2, order_count: 0 };
        let (goals, _) = ensure_weekly_goals_with_progress(
            &mut book, seed_definitions().unwrap(), 1000, &progress);
        let mut workshop = WorkshopState::default();
        assert!(queue_weekly_rewards(&mut book, &goals, &workshop, 1000));
        assert!(!queue_weekly_rewards(&mut book, &goals, &workshop, 1000));
        let restored: AchievementBook = serde_json::from_str(&serde_json::to_string(&book).unwrap()).unwrap();
        let initial_parts = workshop.parts;
        let initial_insight = workshop.insight;
        for (key, intent) in &restored.pending_rewards {
            apply_reward_once(&mut workshop, key, intent, false, 1000).unwrap();
        }
        assert_eq!(workshop.parts - initial_parts, 480.0);
        assert_eq!(workshop.insight - initial_insight, 30.0);
        assert_eq!(workshop.affinity_experience, 20);
        assert_eq!(workshop.reward_receipts.len(), 4);
    }

    #[test]
    fn credited_clock_rejects_sleep_restart_and_clock_reversal() {
        let mut focus = FocusSession { reward_version: 1, ..Default::default() };
        assert_eq!(focus.credit_tick(1000), 0);
        assert_eq!(focus.credit_tick(3000), 2000);
        assert_eq!(focus.credit_tick(900_000), 0);
        assert_eq!(focus.credit_tick(902_000), 2000);
        assert_eq!(focus.credit_tick(100), 0);
        focus.last_tick_at = None;
        assert_eq!(focus.credit_tick(950_000), 0);
        focus.record_distraction();
        assert_eq!(focus.normalized_production_multiplier(), 1.5);
    }

    #[test]
    fn production_bonus_stops_at_remaining_budget_and_excludes_long_gaps() {
        let mut focus = session(25);
        focus.status = FocusSessionStatus::Active;
        focus.credited_duration_ms -= 1000;
        focus.last_tick_at = Some(1000);
        assert_eq!(focus.production_multiplier_for_tick(3000, 1000), 1.25);
        assert_eq!(focus.production_multiplier_for_tick(91_000, 1000), 1.0);
        focus.credit_tick(3000);
        assert_eq!(focus.credited_duration_ms, 1_500_000);
        assert_eq!(focus.normalized_production_multiplier(), 1.0);
    }

    #[test]
    fn legacy_current_week_keeps_its_targets_baselines_and_pays_once() {
        use crate::achievements::WeeklyGoalPlan;
        let now = Local::now().timestamp_millis();
        let mut book = AchievementBook {
            weekly_goal_plan: Some(WeeklyGoalPlan {
                week_key: local_iso_week(now).0,
                goal_keys: vec!["active-30m".into(), "report-1".into(), "pet-5".into()],
                baselines: [("active-30m".into(), 1000.0), ("report-1".into(), 2.0), ("pet-5".into(), 10.0)].into(),
                generated_at: now,
                ..Default::default()
            }),
            counters: [("lifetime.total_online_seconds".into(), 2800.0),
                ("worklog.daily_generated.count".into(), 3.0), ("pet.click.count".into(), 15.0)].into(),
            ..Default::default()
        };
        let (goals, changed) = ensure_weekly_goals_with_progress(&mut book, seed_definitions().unwrap(), now, &WeeklyProgress::default());
        assert!(!changed);
        assert_eq!(goals.goals[0].goal_id, "active-30m");
        assert!(goals.goals.iter().all(|goal| goal.is_complete));
        let mut workshop = WorkshopState::default();
        assert!(queue_weekly_rewards(&mut book, &goals, &workshop, now));
        for (key, intent) in &book.pending_rewards {
            apply_reward_once(&mut workshop, key, intent, false, now).unwrap();
        }
        book.pending_rewards.clear();
        assert!(!queue_weekly_rewards(&mut book, &goals, &workshop, now));
        let (next, changed) = ensure_weekly_goals_with_progress(&mut book, seed_definitions().unwrap(),
            now + 7 * 86_400_000, &WeeklyProgress::default());
        assert!(changed);
        assert_eq!(next.goals[0].goal_id, "companion-120m");
    }

    #[test]
    fn pending_weekly_intents_survive_rollover_and_paid_but_unacknowledged_retry() {
        let now = Local::now().timestamp_millis();
        let mut book = AchievementBook::default();
        let (goals, _) = ensure_weekly_goals_with_progress(&mut book, seed_definitions().unwrap(), now,
            &WeeklyProgress { online_seconds: 7200.0, ..Default::default() });
        let mut workshop = WorkshopState::default();
        queue_weekly_rewards(&mut book, &goals, &workshop, now);
        ensure_weekly_goals_with_progress(&mut book, seed_definitions().unwrap(), now + 7 * 86_400_000,
            &WeeklyProgress::default());
        assert_eq!(book.pending_rewards.len(), 1);
        let (key, intent) = book.pending_rewards.first_key_value().unwrap();
        apply_reward_once(&mut workshop, key, intent, false, now).unwrap();
        let paid = workshop.parts;
        let mut restored: WorkshopState = serde_json::from_value(serde_json::to_value(workshop).unwrap()).unwrap();
        assert!(apply_reward_once(&mut restored, key, intent, false, now + 1000).unwrap().is_none());
        assert_eq!(restored.parts, paid);
    }

    #[test]
    fn rollover_does_not_credit_new_week_counters_to_a_legacy_plan() {
        use crate::achievements::WeeklyGoalPlan;
        let now = Local::now().timestamp_millis();
        let previous = now - 7 * 86_400_000;
        let mut book = AchievementBook {
            weekly_goal_plan: Some(WeeklyGoalPlan {
                week_key: local_iso_week(previous).0, generated_at: previous,
                goal_keys: vec!["active-30m".into(), "report-1".into(), "pet-5".into()],
                baselines: [("active-30m".into(), 0.0), ("report-1".into(), 0.0), ("pet-5".into(), 0.0)].into(),
                ..Default::default()
            }),
            counters: [("lifetime.total_online_seconds".into(), 1800.0),
                ("worklog.daily_generated.count".into(), 1.0), ("pet.click.count".into(), 5.0)].into(),
            ..Default::default()
        };
        let workshop = WorkshopState::default();
        assert!(!queue_previous_week_rewards(&mut book, &workshop, &WorkLogBook::default(),
            &FocusSessionBook::default(), now).unwrap());
        assert!(book.pending_rewards.is_empty());
        let (goals, _) = ensure_weekly_goals_with_progress(&mut book, seed_definitions().unwrap(), now,
            &WeeklyProgress::default());
        assert!(goals.goals.iter().all(|goal| !goal.is_complete));
    }

    #[test]
    fn weekly_focus_and_report_days_require_native_evidence_in_the_same_week() {
        let now = Local::now().timestamp_millis();
        let day = date_key_from_timestamp(now);
        let mut logs = WorkLogBook::default();
        let mut entry = crate::models::WorkLogEntry::new(day.clone(), now);
        entry.active_seconds = 7200;
        entry.sample_count = 1;
        logs.entries.insert(day.clone(), entry);
        let mut complete = session(25);
        complete.ended_at = Some(now);
        let mut early = complete.clone();
        early.credited_duration_ms -= 1000;
        let mut legacy = complete.clone();
        legacy.reward_version = 0;
        let mut old = complete.clone();
        old.ended_at = Some(now - 7 * 86_400_000);
        let focus = FocusSessionBook { sessions: vec![complete, early, legacy, old], ..Default::default() };
        let mut workshop = WorkshopState::default();
        workshop.completed_order_ids.insert(format!("{day}:standard"));
        workshop.completed_order_ids.insert(format!("{}:standard", date_key_from_timestamp(now - 7 * 86_400_000)));
        let progress = weekly_progress(now, &logs, &focus, &workshop);
        assert_eq!(progress.online_seconds, 7200.0);
        assert_eq!(progress.qualified_focus_count, 1);
        assert_eq!(progress.report_days, 1);
        assert_eq!(progress.order_count, 1);
    }

    #[test]
    fn failed_balance_save_keeps_intent_and_restart_can_pay_it_once() {
        use crate::storage::StorageService;
        let root = std::env::temp_dir().join(crate::models::generate_unique_id("coworkpal-reward-test"));
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let workshop = storage.load_or_create_workshop().unwrap();
        let initial = workshop.parts;
        let mut book = AchievementBook::default();
        let key = "week:test:goal";
        let intent = PendingReward { title: "test".into(),
            reward: RewardAmount { parts: 80.0, insight: 5.0, affinity_experience: 0 }, earned_at: 1000 };
        book.pending_rewards.insert(key.into(), intent.clone());
        storage.save_achievements(&book).unwrap();
        std::fs::rename(root.join("save.json"), root.join("saved-before-test.json")).unwrap();
        std::fs::create_dir(root.join("save.json")).unwrap();
        let mut candidate = workshop.clone();
        apply_reward_once(&mut candidate, key, &intent, false, 1000).unwrap();
        assert!(storage.save_workshop(&candidate).is_err());
        assert_eq!(storage.load_or_create_achievements().unwrap().pending_rewards.len(), 1);
        std::fs::remove_dir(root.join("save.json")).unwrap();
        std::fs::rename(root.join("saved-before-test.json"), root.join("save.json")).unwrap();
        let mut retry = storage.load_or_create_workshop().unwrap();
        assert_eq!(retry.parts, initial);
        assert!(retry.reward_receipts.is_empty());
        apply_reward_once(&mut retry, key, &intent, false, 1000).unwrap();
        storage.save_workshop(&retry).unwrap();
        let mut restarted = storage.load_or_create_workshop().unwrap();
        assert_eq!(restarted.parts, initial + 80.0);
        assert!(apply_reward_once(&mut restarted, key, &intent, false, 2000).unwrap().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
