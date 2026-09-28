use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::models::RewardAmount;

use super::definitions::{
    AchievementCondition, AchievementDailyPredicate, AchievementDefinition,
    AchievementDistinctFilter, AchievementOperator,
};

const MAX_STORED_EVENTS: usize = 2_000;
pub const ACHIEVEMENT_BOOK_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Default)]
pub struct WeeklyProgress {
    pub online_seconds: f64,
    pub qualified_focus_count: u32,
    pub report_days: u32,
    pub order_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingReward {
    pub title: String,
    pub reward: RewardAmount,
    pub earned_at: i64,
}

const REWARDED_WEEKLY_GOALS: [WeeklyGoalDefinition; 3] = [
    WeeklyGoalDefinition { key: "companion-120m", title: "本周陪伴 120 分钟",
        counter_key: "", target: 7200.0, unit: "分钟", badge_achievement_id: "A002", route_key: "dashboard" },
    WeeklyGoalDefinition { key: "qualified-focus-2", title: "完整完成 2 次专注",
        counter_key: "", target: 2.0, unit: "次", badge_achievement_id: "A066", route_key: "focus" },
    WeeklyGoalDefinition { key: "weekly-action", title: "2 天报告或 1 张工单",
        counter_key: "", target: 1.0, unit: "项", badge_achievement_id: "A003", route_key: "workLog" },
];

#[derive(Debug, Clone, Copy)]
struct WeeklyGoalDefinition {
    key: &'static str,
    title: &'static str,
    counter_key: &'static str,
    target: f64,
    unit: &'static str,
    badge_achievement_id: &'static str,
    route_key: &'static str,
}

const WEEKLY_GOAL_GROUPS: [[WeeklyGoalDefinition; 2]; 3] = [
    [
        WeeklyGoalDefinition {
            key: "active-30m",
            title: "本周陪伴 30 分钟",
            counter_key: "lifetime.total_online_seconds",
            target: 1_800.0,
            unit: "分钟",
            badge_achievement_id: "A002",
            route_key: "dashboard",
        },
        WeeklyGoalDefinition {
            key: "active-90m",
            title: "本周陪伴 90 分钟",
            counter_key: "lifetime.total_online_seconds",
            target: 5_400.0,
            unit: "分钟",
            badge_achievement_id: "A021",
            route_key: "dashboard",
        },
    ],
    [
        WeeklyGoalDefinition {
            key: "report-1",
            title: "生成 1 份工况报告",
            counter_key: "worklog.daily_generated.count",
            target: 1.0,
            unit: "份",
            badge_achievement_id: "A003",
            route_key: "workLog",
        },
        WeeklyGoalDefinition {
            key: "focus-2",
            title: "完成 2 次专注",
            counter_key: "focus.session.completed.count",
            target: 2.0,
            unit: "次",
            badge_achievement_id: "A066",
            route_key: "focus",
        },
    ],
    [
        WeeklyGoalDefinition {
            key: "workshop-2",
            title: "查看工坊 2 次",
            counter_key: "page.view.count(pageKey='workshop')",
            target: 2.0,
            unit: "次",
            badge_achievement_id: "A005",
            route_key: "workshop",
        },
        WeeklyGoalDefinition {
            key: "pet-5",
            title: "和 CoCat 互动 5 次",
            counter_key: "pet.click.count",
            target: 5.0,
            unit: "次",
            badge_achievement_id: "A011",
            route_key: "dashboard",
        },
    ],
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AchievementBook {
    pub schema_version: u32,
    pub events: Vec<AchievementEventRecord>,
    pub counters: BTreeMap<String, f64>,
    pub daily_rollups: BTreeMap<String, AchievementDailyRollup>,
    pub distinct_values: BTreeMap<String, BTreeSet<String>>,
    pub unlocks: BTreeMap<String, AchievementUnlockRecord>,
    pub notifications: BTreeMap<String, AchievementNotificationRecord>,
    pub notification_queue: Vec<String>,
    pub idempotency_keys: BTreeSet<String>,
    pub weekly_goal_plan: Option<WeeklyGoalPlan>,
    pub pending_rewards: BTreeMap<String, PendingReward>,
}

pub fn compact_achievement_book(book: &mut AchievementBook) -> bool {
    let retained_keys = book
        .events
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect::<BTreeSet<_>>();
    if book.idempotency_keys == retained_keys {
        return false;
    }
    book.idempotency_keys = retained_keys;
    true
}

impl Default for AchievementBook {
    fn default() -> Self {
        Self {
            schema_version: ACHIEVEMENT_BOOK_SCHEMA_VERSION,
            events: Vec::new(),
            counters: BTreeMap::new(),
            daily_rollups: BTreeMap::new(),
            distinct_values: BTreeMap::new(),
            unlocks: BTreeMap::new(),
            notifications: BTreeMap::new(),
            notification_queue: Vec::new(),
            idempotency_keys: BTreeSet::new(),
            weekly_goal_plan: None,
            pending_rewards: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WeeklyGoalPlan {
    pub week_key: String,
    pub goal_keys: Vec<String>,
    pub baselines: BTreeMap<String, f64>,
    // Retained only so schema-v2 plans deserialize and can be regenerated.
    pub achievement_ids: Vec<String>,
    pub generated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyGoals {
    pub week_key: String,
    pub goals: Vec<WeeklyGoal>,
    pub bonus_reward: RewardAmount,
    pub bonus_paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyGoal {
    pub goal_id: String,
    pub title: String,
    pub badge_key: String,
    pub route_key: String,
    pub current: f64,
    pub target: f64,
    pub percent: f64,
    pub progress_label: String,
    pub is_complete: bool,
    pub reward: RewardAmount,
    pub reward_paid: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AchievementDailyRollup {
    pub active_seconds: f64,
    pub high_load_seconds: f64,
    pub thermal_warning_seconds: f64,
    pub active_00_05_seconds: f64,
    pub low_power_mode_enabled_seconds: f64,
    pub report_generated: bool,
    pub report_score: Option<f64>,
    pub report_day_type: Option<String>,
    pub rarity_tier: Option<String>,
    pub rarity_rank: Option<f64>,
    pub rarity_score: Option<f64>,
    pub title_family: Option<String>,
    pub title_level: Option<f64>,
    pub title_progress: Option<f64>,
    pub storage_corruption_rebuilt_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementEventRecord {
    pub event_id: String,
    pub event_name: String,
    pub occurred_at: i64,
    pub received_at: i64,
    pub source: String,
    pub idempotency_key: String,
    pub payload: Value,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackAchievementEventRequest {
    pub event_name: String,
    pub occurred_at: i64,
    pub idempotency_key: String,
    pub payload: Value,
    pub source: String,
}

impl Default for TrackAchievementEventRequest {
    fn default() -> Self {
        Self {
            event_name: String::new(),
            occurred_at: 0,
            idempotency_key: String::new(),
            payload: Value::Object(Default::default()),
            source: "frontend".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackAchievementEventResponse {
    pub accepted: bool,
    pub unlocked: Vec<AchievementUnlockedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementUnlockRecord {
    pub unlock_id: String,
    pub achievement_id: String,
    pub unlocked_at: i64,
    pub points_awarded: u32,
    pub source_event_id: String,
    pub progress_snapshot: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AchievementNotificationRecord {
    pub notification_id: String,
    pub unlock_id: String,
    pub achievement_id: String,
    pub created_at: i64,
    pub seen_at: Option<i64>,
    pub state: AchievementNotificationState,
}

impl Default for AchievementNotificationRecord {
    fn default() -> Self {
        Self {
            notification_id: String::new(),
            unlock_id: String::new(),
            achievement_id: String::new(),
            created_at: 0,
            seen_at: None,
            state: AchievementNotificationState::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementNotificationState {
    #[default]
    Pending,
    Seen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementUnlockedEvent {
    pub unlock_id: String,
    pub achievement_id: String,
    pub title: String,
    pub difficulty_key: String,
    pub category_key: String,
    pub points: u32,
    pub badge_key: String,
    pub unlocked_at: i64,
    pub is_hidden: bool,
    pub unlock_snapshot: Option<BTreeMap<String, f64>>,
    pub cocat_animation_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementCard {
    pub achievement_id: String,
    pub title: String,
    pub category_key: String,
    pub difficulty_key: String,
    pub points: u32,
    pub badge_key: String,
    pub is_hidden: bool,
    pub is_unlocked: bool,
    pub unlocked_at: Option<i64>,
    pub unlock_snapshot: Option<BTreeMap<String, f64>>,
    pub condition_summary: Option<String>,
    pub progress: Option<AchievementProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementProgress {
    pub current: f64,
    pub target: f64,
    pub percent: f64,
    pub label: String,
    pub is_complete: bool,
}

#[derive(Debug, Clone)]
struct ConditionProgress {
    current: f64,
    target: f64,
    label: String,
    is_complete: bool,
}

impl ConditionProgress {
    fn new(current: f64, target: f64, label: impl Into<String>, is_complete: bool) -> Self {
        let target = target.max(0.0);
        Self {
            current: current.max(0.0),
            target,
            label: label.into(),
            is_complete,
        }
    }

    fn ratio(&self) -> f64 {
        if self.target <= 0.0 {
            if self.is_complete {
                1.0
            } else {
                0.0
            }
        } else {
            (self.current / self.target).clamp(0.0, 1.0)
        }
    }
}

impl From<ConditionProgress> for AchievementProgress {
    fn from(progress: ConditionProgress) -> Self {
        let percent = (progress.ratio() * 100.0).round();
        Self {
            current: progress.current,
            target: progress.target,
            percent,
            label: progress.label,
            is_complete: progress.is_complete,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementSummary {
    pub total_points: u32,
    pub unlocked_count: usize,
    pub visible_total_count: usize,
    pub hidden_unlocked_count: usize,
    pub hidden_total_count: usize,
    pub by_difficulty: BTreeMap<String, AchievementBucketSummary>,
    pub by_category: BTreeMap<String, AchievementBucketSummary>,
    pub latest_unlocks: Vec<AchievementCard>,
    pub highest_rank_unlocks: Vec<AchievementCard>,
    pub pending_notification_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementBucketSummary {
    pub unlocked: usize,
    pub total: usize,
}

pub fn ensure_weekly_goals(
    book: &mut AchievementBook,
    definitions: &[AchievementDefinition],
    now: i64,
) -> (WeeklyGoals, bool) {
    ensure_weekly_goals_with_progress(book, definitions, now, &WeeklyProgress::default())
}

pub fn ensure_weekly_goals_with_progress(
    book: &mut AchievementBook,
    definitions: &[AchievementDefinition],
    now: i64,
    progress: &WeeklyProgress,
) -> (WeeklyGoals, bool) {
    let (week_key, _) = local_iso_week(now);
    let plan_is_current = book.weekly_goal_plan.as_ref().is_some_and(|plan| {
        plan.week_key == week_key
            && plan.goal_keys.len() == WEEKLY_GOAL_GROUPS.len()
            && plan.goal_keys.iter().all(|goal_key| {
                weekly_goal_definition(goal_key).is_some() && plan.baselines.contains_key(goal_key)
            })
    });

    let changed = !plan_is_current;
    if changed {
        let selected = REWARDED_WEEKLY_GOALS.iter().collect::<Vec<_>>();
        let baselines = selected
            .iter()
            .map(|goal| {
                (
                    goal.key.to_string(),
                    counter_value_with_fallback(book, goal.counter_key),
                )
            })
            .collect();
        book.weekly_goal_plan = Some(WeeklyGoalPlan {
            week_key: week_key.clone(),
            goal_keys: selected.iter().map(|goal| goal.key.to_string()).collect(),
            baselines,
            achievement_ids: Vec::new(),
            generated_at: now,
        });
    }

    let goals = book
        .weekly_goal_plan
        .as_ref()
        .into_iter()
        .flat_map(|plan| {
            plan.goal_keys.iter().enumerate().filter_map(|(index, goal_key)| {
                let goal = weekly_goal_definition(goal_key)?;
                let baseline = plan.baselines.get(goal_key).copied().unwrap_or_default();
                let mut card = build_weekly_goal(book, definitions, goal, baseline, progress);
                card.reward = weekly_goal_reward(index);
                Some(card)
            })
        })
        .collect();

    (WeeklyGoals { week_key, goals, bonus_reward: RewardAmount {
        parts: 160.0, insight: 10.0, affinity_experience: 10,
    }, bonus_paid: false }, changed)
}

fn weekly_goal_definition(goal_key: &str) -> Option<&'static WeeklyGoalDefinition> {
    WEEKLY_GOAL_GROUPS
        .iter()
        .flatten()
        .chain(REWARDED_WEEKLY_GOALS.iter())
        .find(|goal| goal.key == goal_key)
}

fn build_weekly_goal(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    goal: &WeeklyGoalDefinition,
    baseline: f64,
    progress: &WeeklyProgress,
) -> WeeklyGoal {
    let current_raw = match goal.key {
        "companion-120m" => progress.online_seconds.max(0.0),
        "qualified-focus-2" => progress.qualified_focus_count as f64,
        "weekly-action" => (progress.report_days as f64 / 2.0).max(progress.order_count as f64),
        _ => (counter_value_with_fallback(book, goal.counter_key) - baseline).max(0.0),
    };
    let current = current_raw.min(goal.target);
    let is_complete = current_raw >= goal.target;
    let percent = ((current / goal.target) * 100.0).round();
    let display_current = if goal.unit == "分钟" {
        (current / 60.0).floor()
    } else {
        current.floor()
    };
    let display_target = if goal.unit == "分钟" {
        goal.target / 60.0
    } else {
        goal.target
    };
    let badge_key = definitions
        .iter()
        .find(|definition| definition.id == goal.badge_achievement_id)
        .map(|definition| definition.badge_key.clone())
        .unwrap_or_else(|| "cwp_badge_daily_first_launch_entry".to_string());

    WeeklyGoal {
        goal_id: goal.key.to_string(),
        title: goal.title.to_string(),
        badge_key,
        route_key: goal.route_key.to_string(),
        current,
        target: goal.target,
        percent,
        progress_label: if goal.key == "weekly-action" {
            format!("报告 {}/2 天 · 工单 {}/1 张", progress.report_days.min(2), progress.order_count.min(1))
        } else { format!(
            "{}/{} {}",
            display_current as u64, display_target as u64, goal.unit
        ) },
        is_complete,
        reward: RewardAmount::default(),
        reward_paid: false,
    }
}

pub fn weekly_goal_reward(index: usize) -> RewardAmount {
    match index {
        1 => RewardAmount { parts: 160.0, insight: 10.0, affinity_experience: 5 },
        2 => RewardAmount { parts: 80.0, insight: 5.0, affinity_experience: 5 },
        _ => RewardAmount { parts: 80.0, insight: 5.0, affinity_experience: 0 },
    }
}

pub fn local_iso_week(timestamp_ms: i64) -> (String, u32) {
    let date = Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .unwrap_or_else(Local::now)
        .date_naive();
    let week = date.iso_week();
    (
        format!("{}-W{:02}", week.year(), week.week()),
        week.year() as u32 * 53 + week.week(),
    )
}

pub fn record_achievement_event(
    book: &mut AchievementBook,
    definitions: &[AchievementDefinition],
    request: TrackAchievementEventRequest,
    received_at: i64,
    app_version: &str,
) -> TrackAchievementEventResponse {
    let event_name = request.event_name.trim().to_string();
    if event_name.is_empty() || request.idempotency_key.trim().is_empty() {
        return TrackAchievementEventResponse {
            accepted: false,
            unlocked: Vec::new(),
        };
    }

    if !book
        .idempotency_keys
        .insert(request.idempotency_key.clone())
    {
        return TrackAchievementEventResponse {
            accepted: false,
            unlocked: Vec::new(),
        };
    }

    let event_id = format!("evt-{received_at}-{}", book.events.len() + 1);
    let record = AchievementEventRecord {
        event_id: event_id.clone(),
        event_name,
        occurred_at: if request.occurred_at > 0 {
            request.occurred_at
        } else {
            received_at
        },
        received_at,
        source: request.source,
        idempotency_key: request.idempotency_key,
        payload: request.payload,
        app_version: app_version.to_string(),
    };

    update_counters(book, &record);
    book.events.push(record.clone());
    trim_events(book);

    let unlocked = evaluate_unlocks(book, definitions, &record);

    TrackAchievementEventResponse {
        accepted: true,
        unlocked,
    }
}

pub fn summarize_achievements(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
) -> AchievementSummary {
    let mut total_points = 0_u32;
    let mut hidden_unlocked_count = 0_usize;
    let hidden_total_count = definitions
        .iter()
        .filter(|definition| definition.is_hidden)
        .count();
    let visible_total_count = definitions
        .iter()
        .filter(|definition| !definition.is_hidden)
        .count();
    let mut by_difficulty = BTreeMap::new();
    let mut by_category = BTreeMap::new();

    for definition in definitions {
        let is_unlocked = book.unlocks.contains_key(&definition.id);
        if is_unlocked {
            total_points = total_points.saturating_add(definition.points);
            if definition.is_hidden {
                hidden_unlocked_count += 1;
            }
        }

        increment_bucket(&mut by_difficulty, definition.difficulty.key(), is_unlocked);
        increment_bucket(&mut by_category, definition.category.key(), is_unlocked);
    }

    let unlocked_cards: Vec<AchievementCard> = list_achievement_cards(book, definitions, true)
        .into_iter()
        .filter(|card| card.is_unlocked)
        .collect();
    let mut latest_unlocks = unlocked_cards.clone();
    latest_unlocks.sort_by(|left, right| right.unlocked_at.cmp(&left.unlocked_at));
    latest_unlocks.truncate(3);
    let mut highest_rank_unlocks = unlocked_cards.clone();
    highest_rank_unlocks.sort_by(|left, right| {
        difficulty_weight(&right.difficulty_key)
            .cmp(&difficulty_weight(&left.difficulty_key))
            .then_with(|| right.unlocked_at.cmp(&left.unlocked_at))
    });
    highest_rank_unlocks.truncate(3);

    AchievementSummary {
        total_points,
        unlocked_count: book.unlocks.len(),
        visible_total_count,
        hidden_unlocked_count,
        hidden_total_count,
        by_difficulty,
        by_category,
        latest_unlocks,
        highest_rank_unlocks,
        pending_notification_count: pending_notification_count(book),
    }
}

pub fn list_achievement_cards(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    include_unlocked_hidden: bool,
) -> Vec<AchievementCard> {
    definitions
        .iter()
        .filter_map(|definition| {
            build_achievement_card(book, definitions, definition, include_unlocked_hidden)
        })
        .collect()
}

pub fn get_achievement_card(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    achievement_id: &str,
    include_unlocked_hidden: bool,
) -> Option<AchievementCard> {
    let target = achievement_id.trim();
    definitions
        .iter()
        .find(|definition| {
            definition.id.eq_ignore_ascii_case(target)
                || definition.code.eq_ignore_ascii_case(target)
        })
        .and_then(|definition| {
            build_achievement_card(book, definitions, definition, include_unlocked_hidden)
        })
}

fn build_achievement_card(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
    include_unlocked_hidden: bool,
) -> Option<AchievementCard> {
    let unlock = book.unlocks.get(&definition.id);
    if definition.is_hidden && (unlock.is_none() || !include_unlocked_hidden) {
        return None;
    }
    let progress = evaluate_condition(book, definitions, definition).map(AchievementProgress::from);

    Some(AchievementCard {
        achievement_id: definition.id.clone(),
        title: definition.title.clone(),
        category_key: definition.category.key().to_string(),
        difficulty_key: definition.difficulty.key().to_string(),
        points: definition.points,
        badge_key: definition.badge_key.clone(),
        is_hidden: definition.is_hidden,
        is_unlocked: unlock.is_some(),
        unlocked_at: unlock.map(|record| record.unlocked_at),
        unlock_snapshot: unlock.map(|record| record.progress_snapshot.clone()),
        condition_summary: if unlock.is_some() || !definition.is_hidden {
            Some(definition.condition_summary.clone())
        } else {
            None
        },
        progress,
    })
}

fn difficulty_weight(difficulty_key: &str) -> u32 {
    match difficulty_key {
        "entry" => 1,
        "normal" => 2,
        "skilled" => 4,
        "elite" => 7,
        "epic" => 12,
        "legendary" => 20,
        _ => 0,
    }
}

fn update_counters(book: &mut AchievementBook, record: &AchievementEventRecord) {
    increment_counter(book, &format!("{}.count", record.event_name), 1.0);

    match record.event_name.as_str() {
        "app.active_minute" => {
            increment_payload_counter(book, record, "seconds", "lifetime.total_online_seconds");
            let seconds = payload_f64(record, "seconds").unwrap_or(0.0);
            if seconds > 0.0 {
                let rollup = daily_rollup_mut(book, record);
                rollup.active_seconds += seconds;
                if is_night_watch_hour(record.occurred_at) {
                    rollup.active_00_05_seconds += seconds;
                }
            }
        }
        "page.view" => {
            if let Some(page_key) = payload_string(record, "pageKey") {
                increment_counter(book, &format!("page.view.count(pageKey='{page_key}')"), 1.0);
                insert_distinct_value(book, "page.view.pageKey", &page_key);
            }
        }
        "settings.update" => update_settings_counters(book, record),
        "workshop.production_tick" => {
            increment_payload_counter(book, record, "partsDelta", "lifetime.parts_earned");
            increment_payload_counter(book, record, "insightDelta", "lifetime.insight_earned");
        }
        "workshop.level_up" => {
            if let Some(level) = payload_f64(record, "toLevel") {
                set_counter_max(book, "workshop.level", level);
            }
        }
        "workshop.module_upgrade" => {
            if let Some(module_key) = payload_string(record, "moduleKey") {
                insert_distinct_value(book, "workshop.module_upgrade.moduleKey", &module_key);
            }
            if let Some(level) = payload_f64(record, "toLevel") {
                set_counter_max(book, "workshop.module.max_level", level);
                if let Some(track) = payload_string(record, "track") {
                    set_counter_max(book, &format!("workshop.module_level.{track}"), level);
                    if let Some(module_key) = payload_string(record, "moduleKey") {
                        set_counter_max(
                            book,
                            &format!("workshop.module_level.{module_key}.{track}"),
                            level,
                        );
                    }
                }
            }
        }
        "hardware.segment_rollup" => update_hardware_segment_counters(book, record),
        "worklog.daily_generated" => {
            let score = payload_f64(record, "score");
            let rarity_tier =
                payload_string(record, "rarityTier").map(|tier| normalize_rarity_tier(&tier));
            let rarity_rank_value = rarity_tier.as_deref().map(rarity_rank);
            let rarity_score = payload_f64(record, "rarityScore");
            let title_family =
                payload_string(record, "titleFamily").map(|family| normalize_title_family(&family));
            let title_level = payload_f64(record, "titleLevel");
            let title_progress = payload_f64(record, "titleProgress");
            let day_type =
                payload_string(record, "dayType").map(|day_type| normalize_day_type(&day_type));

            {
                let rollup = daily_rollup_mut(book, record);
                rollup.report_generated = true;
                rollup.report_score = score;
                rollup.rarity_tier = rarity_tier.clone();
                rollup.rarity_rank = rarity_rank_value;
                rollup.rarity_score = rarity_score;
                rollup.title_family = title_family.clone();
                rollup.title_level = title_level;
                rollup.title_progress = title_progress;
                rollup.report_day_type = day_type.clone();
            }

            if let Some(rarity_tier) = rarity_tier {
                insert_distinct_value(book, "worklog.daily_generated.rarityTier", &rarity_tier);
                if let Some(rank) = rarity_rank_value {
                    set_counter_max(book, "worklog.rarity.max_rank", rank);
                }
                increment_counter(
                    book,
                    &format!("worklog.rarity_tier.count(tier='{rarity_tier}')"),
                    1.0,
                );
            }
            if let Some(rarity_score) = rarity_score {
                set_counter_max(book, "worklog.rarity.max_score", rarity_score);
            }
            if let Some(title_family) = title_family {
                insert_distinct_value(book, "worklog.daily_generated.titleFamily", &title_family);
                if let Some(title_level) = title_level {
                    set_counter_max(book, "worklog.title_level.max", title_level);
                    set_counter_max(
                        book,
                        &format!("worklog.title_level.{title_family}"),
                        title_level,
                    );
                }
            }
            if let Some(day_type) = day_type {
                if day_type != "unknown" {
                    insert_distinct_value(book, "worklog.daily_generated.dayType", &day_type);
                }
            }
        }
        "cocat.animation_seen" => {
            if let Some(animation_state) = payload_string(record, "animationState") {
                increment_counter(
                    book,
                    &format!("cocat.animation_seen.count(animationState='{animation_state}')"),
                    1.0,
                );
            }
        }
        "pet.click_burst" => {
            let clicks = payload_f64(record, "clicks").unwrap_or(0.0);
            let window_ms = payload_f64(record, "windowMs").unwrap_or(f64::INFINITY);
            if clicks >= 3.0 && window_ms <= 2_000.0 {
                increment_counter(
                    book,
                    "pet.click_burst.count(clicks >= 3, windowMs <= 2000)",
                    1.0,
                );
            }
        }
        "storage.corruption_rebuilt" => {
            daily_rollup_mut(book, record).storage_corruption_rebuilt_count += 1.0;
        }
        _ => {}
    }
}

fn update_settings_counters(book: &mut AchievementBook, record: &AchievementEventRecord) {
    if let Some(changed_key) = payload_string(record, "changedKey") {
        insert_distinct_value(book, "settings.update.changedKey", &changed_key);
        increment_counter(
            book,
            &format!("settings.update.count(changedKey='{changed_key}')"),
            1.0,
        );
        if let Some(value) = payload_bool(record, "value") {
            increment_counter(
                book,
                &format!("settings.update.count(changedKey='{changed_key}', value={value})"),
                1.0,
            );
        }
    }

    if let Some(theme_name) = payload_string(record, "themeName") {
        insert_distinct_value(book, "settings.update.themeName", &theme_name);
    }

    if let Some(count) = payload_f64(record, "visibleMetricCount") {
        set_counter(book, "settings.visible_monitor_metrics.count", count);
    }
}

fn update_hardware_segment_counters(book: &mut AchievementBook, record: &AchievementEventRecord) {
    let high_load_seconds = payload_f64(record, "highLoadSeconds").unwrap_or(0.0);
    let thermal_warning_seconds = payload_f64(record, "thermalWarningSeconds").unwrap_or(0.0);
    let active_00_05_seconds = payload_f64(record, "active0005Seconds")
        .or_else(|| payload_f64(record, "active_00_05_seconds"))
        .unwrap_or(0.0);
    let low_power_mode_enabled_seconds = payload_f64(record, "lowPowerModeEnabledSeconds")
        .or_else(|| payload_f64(record, "low_power_mode_enabled_seconds"))
        .unwrap_or(0.0);

    increment_payload_counter(
        book,
        record,
        "highLoadSeconds",
        "lifetime.high_load_seconds",
    );
    increment_payload_counter(
        book,
        record,
        "cpuOver50Seconds",
        "lifetime.cpu_over_50_seconds",
    );
    increment_payload_counter(
        book,
        record,
        "memoryOver70Seconds",
        "lifetime.memory_over_70_seconds",
    );
    increment_payload_counter(
        book,
        record,
        "gpuOver70Seconds",
        "lifetime.gpu_over_70_seconds",
    );
    increment_payload_counter(
        book,
        record,
        "thermalWarningSeconds",
        "lifetime.thermal_warning_seconds",
    );
    increment_payload_counter(book, record, "diskBytesTotal", "lifetime.disk_bytes_total");
    increment_payload_counter(
        book,
        record,
        "networkBytesTotal",
        "lifetime.network_bytes_total",
    );
    increment_payload_counter(
        book,
        record,
        "mouseClickCount",
        "lifetime.mouse_click_count",
    );
    increment_payload_counter(
        book,
        record,
        "keyboardPressCount",
        "lifetime.keyboard_press_count",
    );
    increment_payload_counter(
        book,
        record,
        "lowPowerModeEnabledSeconds",
        "lifetime.low_power_mode_enabled_seconds",
    );

    let rollup = daily_rollup_mut(book, record);
    rollup.high_load_seconds += high_load_seconds;
    rollup.thermal_warning_seconds += thermal_warning_seconds;
    rollup.active_00_05_seconds += active_00_05_seconds;
    rollup.low_power_mode_enabled_seconds += low_power_mode_enabled_seconds;
}

fn daily_rollup_mut<'a>(
    book: &'a mut AchievementBook,
    record: &AchievementEventRecord,
) -> &'a mut AchievementDailyRollup {
    let date_key = event_date_key(record);
    book.daily_rollups.entry(date_key).or_default()
}

fn event_date_key(record: &AchievementEventRecord) -> String {
    payload_string(record, "date").unwrap_or_else(|| date_key_from_timestamp(record.occurred_at))
}

fn date_key_from_timestamp(timestamp_ms: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .unwrap_or_else(Local::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn is_night_watch_hour(timestamp_ms: i64) -> bool {
    let Some(timestamp) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return false;
    };
    timestamp.hour() < 5
}

fn evaluate_unlocks(
    book: &mut AchievementBook,
    definitions: &[AchievementDefinition],
    record: &AchievementEventRecord,
) -> Vec<AchievementUnlockedEvent> {
    let mut unlocked = Vec::new();

    for definition in definitions {
        if book.unlocks.contains_key(&definition.id) {
            continue;
        }
        if !is_condition_satisfied(book, definitions, definition) {
            continue;
        }

        let unlock_id = format!(
            "unlock-{}-{}",
            definition.id.to_ascii_lowercase(),
            record.received_at
        );
        let progress_snapshot = book.counters.clone();
        let unlock = AchievementUnlockRecord {
            unlock_id: unlock_id.clone(),
            achievement_id: definition.id.clone(),
            unlocked_at: record.received_at,
            points_awarded: definition.points,
            source_event_id: record.event_id.clone(),
            progress_snapshot: progress_snapshot.clone(),
        };
        book.unlocks.insert(definition.id.clone(), unlock);
        record_unlock_counters(book, definition);
        record_unlock_notification(book, &unlock_id, definition, record.received_at);
        book.notification_queue.push(unlock_id.clone());
        unlocked.push(AchievementUnlockedEvent {
            unlock_id,
            achievement_id: definition.id.clone(),
            title: definition.title.clone(),
            difficulty_key: definition.difficulty.key().to_string(),
            category_key: definition.category.key().to_string(),
            points: definition.points,
            badge_key: definition.badge_key.clone(),
            unlocked_at: record.received_at,
            is_hidden: definition.is_hidden,
            unlock_snapshot: Some(progress_snapshot),
            cocat_animation_state: "achievementPop".to_string(),
        });
    }

    unlocked
}

fn record_unlock_notification(
    book: &mut AchievementBook,
    unlock_id: &str,
    definition: &AchievementDefinition,
    created_at: i64,
) {
    let notification_id = format!("notification-{unlock_id}");
    book.notifications.insert(
        notification_id.clone(),
        AchievementNotificationRecord {
            notification_id,
            unlock_id: unlock_id.to_string(),
            achievement_id: definition.id.clone(),
            created_at,
            seen_at: None,
            state: AchievementNotificationState::Pending,
        },
    );
}

pub fn mark_achievement_notifications_seen(
    book: &mut AchievementBook,
    unlock_ids: Option<Vec<String>>,
    seen_at: i64,
) -> usize {
    let target_unlock_ids: BTreeSet<String> = unlock_ids
        .filter(|ids| !ids.is_empty())
        .map(|ids| ids.into_iter().collect())
        .unwrap_or_else(|| {
            book.notifications
                .values()
                .filter(|notification| notification.state == AchievementNotificationState::Pending)
                .map(|notification| notification.unlock_id.clone())
                .chain(book.notification_queue.iter().cloned())
                .collect()
        });

    if target_unlock_ids.is_empty() {
        return 0;
    }

    let mut changed = 0_usize;
    for notification in book.notifications.values_mut() {
        if target_unlock_ids.contains(&notification.unlock_id)
            && notification.state == AchievementNotificationState::Pending
        {
            notification.state = AchievementNotificationState::Seen;
            notification.seen_at = Some(seen_at);
            changed += 1;
        }
    }

    let previous_len = book.notification_queue.len();
    book.notification_queue
        .retain(|unlock_id| !target_unlock_ids.contains(unlock_id));
    changed + previous_len.saturating_sub(book.notification_queue.len())
}

fn pending_notification_count(book: &AchievementBook) -> usize {
    let notification_unlock_ids: BTreeSet<&str> = book
        .notifications
        .values()
        .map(|notification| notification.unlock_id.as_str())
        .collect();
    let structured_pending = book
        .notifications
        .values()
        .filter(|notification| notification.state == AchievementNotificationState::Pending)
        .count();
    let legacy_pending = book
        .notification_queue
        .iter()
        .filter(|unlock_id| !notification_unlock_ids.contains(unlock_id.as_str()))
        .count();

    structured_pending + legacy_pending
}

fn record_unlock_counters(book: &mut AchievementBook, definition: &AchievementDefinition) {
    increment_counter(book, "achievement.unlocked.count", 1.0);
    if definition.is_hidden {
        increment_counter(book, "hidden_achievement.unlocked.count", 1.0);
    } else {
        increment_counter(book, "non_hidden_achievement.unlocked.count", 1.0);
    }
    increment_counter(
        book,
        &format!(
            "achievement.difficulty.{}.unlocked.count",
            definition.difficulty.key()
        ),
        1.0,
    );
    increment_counter(
        book,
        &format!(
            "achievement.category.{}.unlocked.count",
            definition.category.key()
        ),
        1.0,
    );
}

fn is_condition_satisfied(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
) -> bool {
    evaluate_condition(book, definitions, definition)
        .map(|progress| progress.is_complete)
        .unwrap_or(false)
}

fn evaluate_condition(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
) -> Option<ConditionProgress> {
    let compiled =
        evaluate_compiled_condition(book, definitions, definition, &definition.condition);

    #[cfg(debug_assertions)]
    {
        let legacy = evaluate_legacy_condition(book, definitions, definition);
        if !condition_progress_matches(compiled.as_ref(), legacy.as_ref()) {
            tracing::error!(
                achievement_id = definition.id,
                compiled = ?compiled,
                legacy = ?legacy,
                "compiled achievement condition differs from the legacy evaluator"
            );
        }
    }

    compiled
}

fn evaluate_compiled_condition(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
    condition: &AchievementCondition,
) -> Option<ConditionProgress> {
    match condition {
        AchievementCondition::All { conditions } => select_condition_progress(
            conditions,
            |condition| evaluate_compiled_condition(book, definitions, definition, condition),
            false,
        ),
        AchievementCondition::Any { conditions } => select_condition_progress(
            conditions,
            |condition| evaluate_compiled_condition(book, definitions, definition, condition),
            true,
        ),
        AchievementCondition::Counter { counter, op, value } => Some(compare_compiled_progress(
            compiled_counter_value(book, definitions, definition, counter),
            *op,
            *value,
            progress_label(counter),
        )),
        AchievementCondition::Sum {
            counters,
            op,
            value,
        } => Some(compare_compiled_progress(
            counters
                .iter()
                .map(|counter| compiled_counter_value(book, definitions, definition, counter))
                .sum(),
            *op,
            *value,
            progress_label(&counters.join(" + ")),
        )),
        AchievementCondition::Max {
            counters,
            op,
            value,
        } => Some(compare_compiled_progress(
            counters
                .iter()
                .map(|counter| compiled_counter_value(book, definitions, definition, counter))
                .fold(0.0, f64::max),
            *op,
            *value,
            progress_label(&format!("max({})", counters.join(", "))),
        )),
        AchievementCondition::DistinctCount {
            key,
            filter,
            op,
            value,
        } => {
            let current = book
                .distinct_values
                .get(key)
                .map(|values| {
                    values
                        .iter()
                        .filter(|candidate| distinct_value_matches(candidate, filter))
                        .count() as f64
                })
                .unwrap_or(0.0);
            Some(compare_compiled_progress(
                current,
                *op,
                *value as f64,
                "去重数量",
            ))
        }
        AchievementCondition::CalendarDays {
            predicate,
            op,
            value,
        } => Some(compare_compiled_progress(
            book.daily_rollups
                .values()
                .filter(|rollup| compiled_daily_predicate_matches(rollup, predicate))
                .count() as f64,
            *op,
            *value as f64,
            "达标天数",
        )),
        AchievementCondition::ConsecutiveDays { predicate, days } => {
            Some(compare_compiled_progress(
                compiled_consecutive_day_count(book, predicate) as f64,
                AchievementOperator::GreaterThanOrEqual,
                *days as f64,
                "连续天数",
            ))
        }
        AchievementCondition::CalendarMonths {
            min_report_generated_days,
            months,
        } => Some(compare_compiled_progress(
            report_month_count(book, *min_report_generated_days) as f64,
            AchievementOperator::GreaterThanOrEqual,
            *months as f64,
            "达标月份",
        )),
        AchievementCondition::PerBucketMin {
            key,
            bucket_count,
            min_value,
        } => evaluate_compiled_bucket_min(book, definitions, key, *bucket_count, *min_value),
        AchievementCondition::ExcludeSelf { .. } => None,
    }
}

fn select_condition_progress(
    conditions: &[AchievementCondition],
    evaluate: impl Fn(&AchievementCondition) -> Option<ConditionProgress>,
    any: bool,
) -> Option<ConditionProgress> {
    let mut selected: Option<ConditionProgress> = None;
    let mut is_complete = !any;

    for condition in conditions {
        let progress = evaluate(condition)?;
        if any {
            is_complete |= progress.is_complete;
        } else {
            is_complete &= progress.is_complete;
        }
        let should_select = selected
            .as_ref()
            .map(|current| {
                if any {
                    progress.ratio() > current.ratio()
                } else {
                    progress.ratio() < current.ratio()
                }
            })
            .unwrap_or(true);
        if should_select {
            selected = Some(progress);
        }
    }

    selected.map(|mut progress| {
        progress.is_complete = is_complete;
        progress
    })
}

fn compiled_counter_value(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
    key: &str,
) -> f64 {
    match key {
        "achievement.unlocked.count" => definitions
            .iter()
            .filter(|candidate| book.unlocks.contains_key(&candidate.id))
            .count() as f64,
        "non_hidden_achievement.unlocked.count" => definitions
            .iter()
            .filter(|candidate| !candidate.is_hidden && book.unlocks.contains_key(&candidate.id))
            .count() as f64,
        "hidden_achievement.unlocked.count(excludeSelf=true)" => definitions
            .iter()
            .filter(|candidate| {
                candidate.id != definition.id
                    && candidate.is_hidden
                    && book.unlocks.contains_key(&candidate.id)
            })
            .count() as f64,
        _ => counter_value_with_fallback(book, key),
    }
}

fn distinct_value_matches(value: &String, filter: &AchievementDistinctFilter) -> bool {
    match filter {
        AchievementDistinctFilter::Any => true,
        AchievementDistinctFilter::In { values } => values.contains(value),
        AchievementDistinctFilter::NotEqual { value: blocked } => value != blocked,
    }
}

fn compiled_daily_predicate_matches(
    rollup: &AchievementDailyRollup,
    predicate: &AchievementDailyPredicate,
) -> bool {
    match predicate {
        AchievementDailyPredicate::All { predicates } => predicates
            .iter()
            .all(|predicate| compiled_daily_predicate_matches(rollup, predicate)),
        AchievementDailyPredicate::Numeric { key, op, value } => {
            comparison_is_satisfied(daily_metric_value(rollup, key), *op, *value)
        }
        AchievementDailyPredicate::TextEqual { key, value } => match key.as_str() {
            "report_day_type" => {
                rollup.report_day_type.as_deref() == Some(normalize_day_type(value).as_str())
            }
            "rarity_tier" => {
                rollup.rarity_tier.as_deref() == Some(normalize_rarity_tier(value).as_str())
            }
            "title_family" => {
                rollup.title_family.as_deref() == Some(normalize_title_family(value).as_str())
            }
            _ => false,
        },
    }
}

fn compiled_consecutive_day_count(
    book: &AchievementBook,
    predicate: &AchievementDailyPredicate,
) -> usize {
    consecutive_day_count_matching(book, |rollup| {
        compiled_daily_predicate_matches(rollup, predicate)
    })
}

fn report_month_count(book: &AchievementBook, minimum_days: u32) -> usize {
    let mut report_days_by_month: BTreeMap<String, u32> = BTreeMap::new();
    for (date, rollup) in &book.daily_rollups {
        if rollup.report_generated && date.len() >= 7 {
            *report_days_by_month
                .entry(date[..7].to_string())
                .or_default() += 1;
        }
    }
    report_days_by_month
        .values()
        .filter(|days| **days >= minimum_days)
        .count()
}

fn evaluate_compiled_bucket_min(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    key: &str,
    bucket_count: u32,
    target: f64,
) -> Option<ConditionProgress> {
    let (values, label): (Vec<f64>, &str) = match key {
        "workshop.parts" => (
            workshop_module_track_keys("parts")
                .iter()
                .map(|key| counter_value_with_fallback(book, key))
                .collect(),
            "零件轨",
        ),
        "workshop.process" => (
            workshop_module_track_keys("process")
                .iter()
                .map(|key| counter_value_with_fallback(book, key))
                .collect(),
            "工艺轨",
        ),
        "workshop.all_tracks" => (
            workshop_track_keys()
                .iter()
                .map(|key| counter_value_with_fallback(book, key))
                .collect(),
            "模块轨道",
        ),
        "achievement.categories" => (
            [
                "daily_use",
                "task_efficiency",
                "long_streak",
                "feature_exploration",
                "data_milestone",
                "workshop_growth",
                "hardware_health",
                "social_collaboration",
                "hidden_easter",
            ]
            .iter()
            .map(|category| unlocked_count_for_category(book, definitions, category) as f64)
            .collect(),
            "分类解锁",
        ),
        "achievement.difficulties" => (
            ["entry", "normal", "skilled", "elite", "epic", "legendary"]
                .iter()
                .map(|difficulty| {
                    unlocked_count_for_difficulty(book, definitions, difficulty) as f64
                })
                .collect(),
            "难度解锁",
        ),
        "worklog.report_day_types" => (
            report_day_types()
                .iter()
                .map(|day_type| {
                    book.daily_rollups
                        .values()
                        .filter(|rollup| rollup.report_day_type.as_deref() == Some(*day_type))
                        .count() as f64
                })
                .collect(),
            "工作日类型",
        ),
        "worklog.title_families" => (
            work_day_title_families()
                .iter()
                .map(|family| {
                    counter_value_with_fallback(book, &format!("worklog.title_level.{family}"))
                })
                .collect(),
            "工况职级",
        ),
        "cocat.animation_states" => (
            cocat_animation_states()
                .iter()
                .map(|state| {
                    counter_value_with_fallback(
                        book,
                        &format!("cocat.animation_seen.count(animationState='{state}')"),
                    )
                })
                .collect(),
            "动画见证",
        ),
        _ => return None,
    };
    if values.len() != bucket_count as usize {
        return None;
    }
    let current = values.into_iter().fold(f64::INFINITY, f64::min);
    Some(ConditionProgress::new(
        if current.is_finite() { current } else { 0.0 },
        target,
        format!("{label}最低值"),
        current >= target,
    ))
}

fn compare_compiled_progress(
    current: f64,
    operator: AchievementOperator,
    target: f64,
    label: impl Into<String>,
) -> ConditionProgress {
    let is_complete = comparison_is_satisfied(current, operator, target);
    let progress_current = match operator {
        AchievementOperator::GreaterThanOrEqual | AchievementOperator::GreaterThan => current,
        AchievementOperator::LessThanOrEqual | AchievementOperator::LessThan => {
            if is_complete {
                target
            } else {
                (target - (current - target)).max(0.0)
            }
        }
        AchievementOperator::Equal => {
            if is_complete {
                target
            } else {
                current
            }
        }
    };
    ConditionProgress::new(progress_current, target, label, is_complete)
}

fn comparison_is_satisfied(current: f64, operator: AchievementOperator, target: f64) -> bool {
    match operator {
        AchievementOperator::GreaterThanOrEqual => current >= target,
        AchievementOperator::LessThanOrEqual => current <= target,
        AchievementOperator::Equal => (current - target).abs() < f64::EPSILON,
        AchievementOperator::GreaterThan => current > target,
        AchievementOperator::LessThan => current < target,
    }
}

#[cfg(debug_assertions)]
fn condition_progress_matches(
    compiled: Option<&ConditionProgress>,
    legacy: Option<&ConditionProgress>,
) -> bool {
    match (compiled, legacy) {
        (Some(compiled), Some(legacy)) => compiled.is_complete == legacy.is_complete,
        (None, None) => true,
        _ => false,
    }
}

fn evaluate_legacy_condition(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
) -> Option<ConditionProgress> {
    let normalized = definition.condition_summary.replace('`', "");
    let parts = normalized
        .split('且')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let mut selected: Option<ConditionProgress> = None;
    let mut all_complete = true;

    for part in parts {
        let progress = evaluate_condition_part(book, definitions, definition, part)
            .unwrap_or_else(|| ConditionProgress::new(0.0, 1.0, "规则待接入", false));
        all_complete &= progress.is_complete;
        if selected
            .as_ref()
            .map(|current| progress.ratio() < current.ratio())
            .unwrap_or(true)
        {
            selected = Some(progress);
        }
    }

    selected.map(|mut progress| {
        progress.is_complete = all_complete;
        progress
    })
}

fn evaluate_condition_part(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    definition: &AchievementDefinition,
    condition: &str,
) -> Option<ConditionProgress> {
    evaluate_text_condition(book, definitions, condition)
        .or_else(|| evaluate_calendar_condition(book, condition))
        .or_else(|| evaluate_distinct_condition(book, condition))
        .or_else(|| evaluate_comparison_condition(book, definition, condition))
}

fn evaluate_text_condition(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    condition: &str,
) -> Option<ConditionProgress> {
    let (_, operator, target) = split_numeric_comparison(condition)?;

    if condition.contains("当前可见指标数") {
        return Some(compare_progress(
            counter_value(book, "settings.visible_monitor_metrics.count"),
            operator,
            target,
            "可见指标",
        ));
    }

    if condition.contains("12 条轨道") {
        return Some(evaluate_track_bucket_min(
            book,
            &workshop_track_keys(),
            target,
            "模块轨道",
        ));
    }

    if condition.contains("6 个模块") && condition.contains("parts") {
        return Some(evaluate_track_bucket_min(
            book,
            &workshop_module_track_keys("parts"),
            target,
            "零件轨",
        ));
    }

    if condition.contains("6 个模块") && condition.contains("process") {
        return Some(evaluate_track_bucket_min(
            book,
            &workshop_module_track_keys("process"),
            target,
            "工艺轨",
        ));
    }

    if condition.contains("任意模块任一升级轨等级") || condition.contains("第一条满级轨道")
    {
        return Some(compare_progress(
            counter_value(book, "workshop.module.max_level"),
            operator,
            target,
            "最高模块轨道",
        ));
    }

    if condition.contains("9 个分类") {
        let current = [
            "daily_use",
            "task_efficiency",
            "long_streak",
            "feature_exploration",
            "data_milestone",
            "workshop_growth",
            "hardware_health",
            "social_collaboration",
            "hidden_easter",
        ]
        .iter()
        .map(|category| unlocked_count_for_category(book, definitions, category) as f64)
        .fold(f64::INFINITY, f64::min);
        let current = if current.is_finite() { current } else { 0.0 };
        return Some(ConditionProgress::new(
            current,
            target,
            "分类解锁最低值",
            current >= target,
        ));
    }

    if condition.contains("6 个难度") {
        let current = ["entry", "normal", "skilled", "elite", "epic", "legendary"]
            .iter()
            .map(|difficulty| unlocked_count_for_difficulty(book, definitions, difficulty) as f64)
            .fold(f64::INFINITY, f64::min);
        let current = if current.is_finite() { current } else { 0.0 };
        return Some(ConditionProgress::new(
            current,
            target,
            "难度解锁最低值",
            current >= target,
        ));
    }

    if condition.contains("7 个 report_day_type") {
        return Some(evaluate_report_day_type_bucket(book, target));
    }

    if condition.contains("7 个 title_family") {
        let keys: Vec<String> = work_day_title_families()
            .iter()
            .map(|family| format!("worklog.title_level.{family}"))
            .collect();
        return Some(evaluate_track_bucket_min(book, &keys, target, "工况职级"));
    }

    if condition.contains("18 个 CoCat 动画状态") {
        let keys: Vec<String> = cocat_animation_states()
            .iter()
            .map(|state| format!("cocat.animation_seen.count(animationState='{state}')"))
            .collect();
        return Some(evaluate_track_bucket_min(book, &keys, target, "动画见证"));
    }

    if condition.contains("hidden_achievement.unlocked.count")
        && !condition.contains("non_hidden_achievement.unlocked.count")
    {
        let count = definitions
            .iter()
            .filter(|candidate| candidate.is_hidden && book.unlocks.contains_key(&candidate.id))
            .count() as f64;
        return Some(compare_progress(count, operator, target, "隐藏成就"));
    }

    if condition.contains("non_hidden_achievement.unlocked.count") {
        let count = definitions
            .iter()
            .filter(|candidate| !candidate.is_hidden && book.unlocks.contains_key(&candidate.id))
            .count() as f64;
        return Some(compare_progress(count, operator, target, "可见成就"));
    }

    if condition.contains("achievement.unlocked.count") {
        let count = definitions
            .iter()
            .filter(|candidate| book.unlocks.contains_key(&candidate.id))
            .count() as f64;
        return Some(compare_progress(count, operator, target, "已解锁成就"));
    }

    None
}

fn evaluate_calendar_condition(
    book: &AchievementBook,
    condition: &str,
) -> Option<ConditionProgress> {
    let (left, operator, target) = split_numeric_comparison(condition)?;
    if let Some(predicate) = function_argument(&left, "calendar.days") {
        let current = book
            .daily_rollups
            .values()
            .filter(|rollup| daily_rollup_matches(rollup, &predicate))
            .count() as f64;
        return Some(compare_progress(current, operator, target, "达标天数"));
    }

    if let Some(predicate) = function_argument(&left, "calendar.consecutive_days") {
        let current = consecutive_day_count(book, &predicate) as f64;
        return Some(compare_progress(current, operator, target, "连续天数"));
    }

    if let Some(predicate) = function_argument(&left, "calendar.months") {
        let current = month_count(book, &predicate) as f64;
        return Some(compare_progress(current, operator, target, "达标月份"));
    }

    None
}

fn evaluate_distinct_condition(
    book: &AchievementBook,
    condition: &str,
) -> Option<ConditionProgress> {
    let (left, operator, target) = split_numeric_comparison(condition)?;
    let expression = function_argument(&left, "distinct_count")?;
    let (key, filter) = parse_distinct_expression(&expression);
    let current = book
        .distinct_values
        .get(&key)
        .map(|values| values.iter().filter(|value| filter(value)).count() as f64)
        .unwrap_or(0.0);
    Some(compare_progress(current, operator, target, "去重数量"))
}

fn evaluate_comparison_condition(
    book: &AchievementBook,
    definition: &AchievementDefinition,
    condition: &str,
) -> Option<ConditionProgress> {
    let (left, operator, target) = split_numeric_comparison(condition)?;
    let current = expression_value(book, definition, &left);
    Some(compare_progress(
        current,
        operator,
        target,
        progress_label(&left),
    ))
}

fn compare_progress(
    current: f64,
    operator: ComparisonOperator,
    target: f64,
    label: impl Into<String>,
) -> ConditionProgress {
    let is_complete = match operator {
        ComparisonOperator::GreaterThanOrEqual => current >= target,
        ComparisonOperator::LessThanOrEqual => current <= target,
        ComparisonOperator::Equal => (current - target).abs() < f64::EPSILON,
    };
    let progress_current = match operator {
        ComparisonOperator::GreaterThanOrEqual => current,
        ComparisonOperator::LessThanOrEqual => {
            if is_complete {
                target
            } else {
                (target - (current - target)).max(0.0)
            }
        }
        ComparisonOperator::Equal => {
            if is_complete {
                target
            } else {
                current
            }
        }
    };
    ConditionProgress::new(progress_current, target, label, is_complete)
}

#[derive(Debug, Clone, Copy)]
enum ComparisonOperator {
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
}

fn split_numeric_comparison(condition: &str) -> Option<(String, ComparisonOperator, f64)> {
    let trimmed = condition.trim().trim_end_matches('。').trim();
    for (operator_text, operator) in [
        (">=", ComparisonOperator::GreaterThanOrEqual),
        ("<=", ComparisonOperator::LessThanOrEqual),
        ("=", ComparisonOperator::Equal),
    ] {
        if let Some(index) = find_top_level_operator(trimmed, operator_text) {
            let left = trimmed[..index].trim().to_string();
            let right = trimmed[index + operator_text.len()..]
                .trim()
                .trim_end_matches('。')
                .trim();
            if let Some(target) = parse_leading_number(right) {
                return Some((left, operator, target));
            }
        }
    }
    None
}

fn find_top_level_operator(input: &str, operator: &str) -> Option<usize> {
    let mut depth = 0_i32;
    let mut in_quote = false;
    let mut index = 0_usize;
    while index < input.len() {
        let rest = &input[index..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch == '\'' {
            in_quote = !in_quote;
        } else if !in_quote {
            if ch == '(' || ch == '[' {
                depth += 1;
            } else if ch == ')' || ch == ']' {
                depth -= 1;
            } else if depth == 0 && rest.starts_with(operator) {
                return Some(index);
            }
        }
        index += ch.len_utf8();
    }
    None
}

fn parse_leading_number(value: &str) -> Option<f64> {
    let number: String = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect();
    number.parse::<f64>().ok()
}

fn expression_value(
    book: &AchievementBook,
    definition: &AchievementDefinition,
    expression: &str,
) -> f64 {
    let expression = expression.trim();
    if expression.contains(" + ") {
        return expression
            .split(" + ")
            .map(|part| expression_value(book, definition, part))
            .sum();
    }

    if let Some(argument) = function_argument(expression, "max") {
        return argument
            .split(',')
            .map(|part| expression_value(book, definition, part))
            .fold(0.0, f64::max);
    }

    let key = normalize_counter_key(expression, definition);
    counter_value_with_fallback(book, &key)
}

fn normalize_counter_key(expression: &str, definition: &AchievementDefinition) -> String {
    let trimmed = expression.trim().trim_end_matches('。').trim();
    if trimmed == "当前可见指标数" {
        return "settings.visible_monitor_metrics.count".to_string();
    }
    if trimmed == "hidden_achievement.unlocked.count(excludeSelf=true)" {
        return "hidden_achievement.unlocked.count".to_string();
    }
    if trimmed == "achievement.unlocked.count" {
        return "achievement.unlocked.count".to_string();
    }
    if trimmed == "non_hidden_achievement.unlocked.count" {
        return "non_hidden_achievement.unlocked.count".to_string();
    }
    if trimmed == "self.points" {
        return definition.points.to_string();
    }
    trimmed.to_string()
}

fn counter_value_with_fallback(book: &AchievementBook, key: &str) -> f64 {
    if let Ok(value) = key.parse::<f64>() {
        return value;
    }
    if let Some(value) = book.counters.get(key) {
        return *value;
    }
    if let Some((base, _)) = key.split_once('(') {
        let fallback = base.trim().to_string();
        if let Some(value) = book.counters.get(&fallback) {
            return *value;
        }
    }
    0.0
}

fn function_argument(expression: &str, function_name: &str) -> Option<String> {
    let prefix = format!("{function_name}(");
    let trimmed = expression.trim();
    trimmed
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(|value| value.trim().to_string())
}

fn parse_distinct_expression(expression: &str) -> (String, Box<dyn Fn(&String) -> bool>) {
    if let Some((key, allowed_values)) = expression.split_once(" in ") {
        let allowed: BTreeSet<String> = allowed_values
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|value| value.trim().trim_matches('\'').to_string())
            .filter(|value| !value.is_empty())
            .collect();
        return (
            key.trim().to_string(),
            Box::new(move |value| allowed.contains(value)),
        );
    }

    if let Some((key, filter)) = expression.split_once(" where ") {
        if filter.contains("!=") {
            let blocked = filter
                .split("!=")
                .nth(1)
                .unwrap_or_default()
                .trim()
                .trim_matches('\'')
                .to_string();
            return (
                key.trim().to_string(),
                Box::new(move |value| value != &blocked),
            );
        }
    }

    let key = expression.trim().to_string();
    (key, Box::new(|_| true))
}

fn daily_rollup_matches(rollup: &AchievementDailyRollup, predicate: &str) -> bool {
    predicate
        .split(" AND ")
        .map(str::trim)
        .all(|part| daily_rollup_part_matches(rollup, part))
}

fn daily_rollup_part_matches(rollup: &AchievementDailyRollup, predicate: &str) -> bool {
    if let Some((left, operator, target)) = split_numeric_comparison(predicate) {
        let current = daily_metric_value(rollup, &left);
        return match operator {
            ComparisonOperator::GreaterThanOrEqual => current >= target,
            ComparisonOperator::LessThanOrEqual => current <= target,
            ComparisonOperator::Equal => (current - target).abs() < f64::EPSILON,
        };
    }

    if let Some((left, right)) = predicate.split_once('=') {
        if left.trim() == "report_day_type" {
            let expected = normalize_day_type(right.trim().trim_matches('\''));
            return rollup.report_day_type.as_deref() == Some(expected.as_str());
        }
        if left.trim() == "rarity_tier" {
            let expected = normalize_rarity_tier(right.trim().trim_matches('\''));
            return rollup.rarity_tier.as_deref() == Some(expected.as_str());
        }
        if left.trim() == "title_family" {
            let expected = normalize_title_family(right.trim().trim_matches('\''));
            return rollup.title_family.as_deref() == Some(expected.as_str());
        }
    }

    false
}

fn daily_metric_value(rollup: &AchievementDailyRollup, key: &str) -> f64 {
    match key.trim() {
        "active_seconds" => rollup.active_seconds,
        "high_load_seconds" => rollup.high_load_seconds,
        "thermal_warning_seconds" => rollup.thermal_warning_seconds,
        "active_00_05_seconds" => rollup.active_00_05_seconds,
        "low_power_mode_enabled_seconds" => rollup.low_power_mode_enabled_seconds,
        "report_score" => rollup.report_score.unwrap_or(0.0),
        "rarity_rank" => rollup.rarity_rank.unwrap_or(0.0),
        "rarity_score" => rollup.rarity_score.unwrap_or(0.0),
        "title_level" => rollup.title_level.unwrap_or(0.0),
        "title_progress" => rollup.title_progress.unwrap_or(0.0),
        "storage.corruption_rebuilt.count" => rollup.storage_corruption_rebuilt_count,
        _ => 0.0,
    }
}

fn consecutive_day_count(book: &AchievementBook, predicate: &str) -> usize {
    consecutive_day_count_matching(book, |rollup| daily_rollup_matches(rollup, predicate))
}

fn consecutive_day_count_matching(
    book: &AchievementBook,
    matches: impl Fn(&AchievementDailyRollup) -> bool,
) -> usize {
    let mut dates: Vec<(NaiveDate, &AchievementDailyRollup)> = book
        .daily_rollups
        .iter()
        .filter_map(|(date, rollup)| {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .ok()
                .map(|parsed| (parsed, rollup))
        })
        .collect();
    dates.sort_by_key(|(date, _)| *date);

    let mut current = 0_usize;
    let mut best = 0_usize;
    let mut previous_date: Option<NaiveDate> = None;

    for (date, rollup) in dates {
        if matches(rollup) {
            if previous_date
                .map(|previous| date.signed_duration_since(previous).num_days() == 1)
                .unwrap_or(false)
            {
                current += 1;
            } else {
                current = 1;
            }
            best = best.max(current);
            previous_date = Some(date);
        } else {
            current = 0;
            previous_date = Some(date);
        }
    }

    best
}

fn month_count(book: &AchievementBook, predicate: &str) -> usize {
    let Some((left, operator, target)) = split_numeric_comparison(predicate) else {
        return 0;
    };
    if left.trim() != "report_generated_days" {
        return 0;
    }

    let mut report_days_by_month: BTreeMap<String, f64> = BTreeMap::new();
    for (date, rollup) in &book.daily_rollups {
        if rollup.report_generated && date.len() >= 7 {
            *report_days_by_month
                .entry(date[..7].to_string())
                .or_insert(0.0) += 1.0;
        }
    }

    report_days_by_month
        .values()
        .filter(|current| match operator {
            ComparisonOperator::GreaterThanOrEqual => **current >= target,
            ComparisonOperator::LessThanOrEqual => **current <= target,
            ComparisonOperator::Equal => (**current - target).abs() < f64::EPSILON,
        })
        .count()
}

fn evaluate_track_bucket_min(
    book: &AchievementBook,
    keys: &[String],
    target: f64,
    label: &str,
) -> ConditionProgress {
    let current = keys
        .iter()
        .map(|key| counter_value_with_fallback(book, key))
        .fold(f64::INFINITY, f64::min);
    let current = if current.is_finite() { current } else { 0.0 };
    ConditionProgress::new(current, target, format!("{label}最低值"), current >= target)
}

fn evaluate_report_day_type_bucket(book: &AchievementBook, target: f64) -> ConditionProgress {
    let current = report_day_types()
        .iter()
        .map(|day_type| {
            book.daily_rollups
                .values()
                .filter(|rollup| rollup.report_day_type.as_deref() == Some(*day_type))
                .count() as f64
        })
        .fold(f64::INFINITY, f64::min);
    let current = if current.is_finite() { current } else { 0.0 };
    ConditionProgress::new(current, target, "工作日类型最低天数", current >= target)
}

fn unlocked_count_for_category(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    category_key: &str,
) -> usize {
    definitions
        .iter()
        .filter(|definition| {
            definition.category.key() == category_key && book.unlocks.contains_key(&definition.id)
        })
        .count()
}

fn unlocked_count_for_difficulty(
    book: &AchievementBook,
    definitions: &[AchievementDefinition],
    difficulty_key: &str,
) -> usize {
    definitions
        .iter()
        .filter(|definition| {
            definition.difficulty.key() == difficulty_key
                && book.unlocks.contains_key(&definition.id)
        })
        .count()
}

fn workshop_module_track_keys(track: &str) -> Vec<String> {
    ["cpu", "gpu", "ram", "network", "temperature", "disk"]
        .iter()
        .map(|module| format!("workshop.module_level.{module}.{track}"))
        .collect()
}

fn workshop_track_keys() -> Vec<String> {
    ["parts", "process"]
        .iter()
        .flat_map(|track| workshop_module_track_keys(track))
        .collect()
}

fn report_day_types() -> [&'static str; 7] {
    [
        "deepFocus",
        "buildBurst",
        "archiveFlow",
        "pressureRepair",
        "stableMaintenance",
        "fragmentedSwitching",
        "lowLoadCompanion",
    ]
}

fn work_day_title_families() -> [&'static str; 7] {
    [
        "focus", "build", "archive", "pressure", "steady", "switch", "quiet",
    ]
}

fn cocat_animation_states() -> [&'static str; 18] {
    [
        "bootWake",
        "idle",
        "hover",
        "click",
        "dragging",
        "dropLanding",
        "panelOpen",
        "panelClose",
        "temperatureCheck",
        "memoryCrowded",
        "repairing",
        "dataSorting",
        "sleep",
        "celebrate",
        "updateInstalling",
        "achievementPop",
        "errorGlitch",
        "lowPowerStatic",
    ]
}

fn normalize_rarity_tier(tier: &str) -> String {
    match tier.trim().to_ascii_uppercase().as_str() {
        "SS" => "SS".to_string(),
        "S" => "S".to_string(),
        "A" => "A".to_string(),
        "B" => "B".to_string(),
        _ => "C".to_string(),
    }
}

fn rarity_rank(tier: &str) -> f64 {
    match normalize_rarity_tier(tier).as_str() {
        "SS" => 5.0,
        "S" => 4.0,
        "A" => 3.0,
        "B" => 2.0,
        _ => 1.0,
    }
}

fn normalize_title_family(family: &str) -> String {
    match family.trim() {
        "focus" | "deepFocus" | "DeepFocus" => "focus".to_string(),
        "build" | "buildBurst" | "BuildBurst" => "build".to_string(),
        "archive" | "archiveFlow" | "ArchiveFlow" => "archive".to_string(),
        "pressure" | "pressureRepair" | "PressureRepair" => "pressure".to_string(),
        "steady" | "stableMaintenance" | "StableMaintenance" => "steady".to_string(),
        "switch" | "fragmentedSwitching" | "FragmentedSwitching" => "switch".to_string(),
        "quiet" | "lowLoadCompanion" | "LowLoadCompanion" => "quiet".to_string(),
        _ => "observe".to_string(),
    }
}

fn normalize_day_type(day_type: &str) -> String {
    match day_type.trim() {
        "DeepFocus" | "deep_focus" | "deepFocus" => "deepFocus".to_string(),
        "BuildBurst" | "build_burst" | "buildBurst" => "buildBurst".to_string(),
        "ArchiveFlow" | "archive_flow" | "archiveFlow" => "archiveFlow".to_string(),
        "PressureRepair" | "pressure_repair" | "pressureRepair" => "pressureRepair".to_string(),
        "StableMaintenance" | "stable_maintenance" | "stableMaintenance" => {
            "stableMaintenance".to_string()
        }
        "FragmentedSwitching" | "fragmented_switching" | "fragmentedSwitching" => {
            "fragmentedSwitching".to_string()
        }
        "LowLoadCompanion" | "low_load_companion" | "lowLoadCompanion" => {
            "lowLoadCompanion".to_string()
        }
        "Unknown" | "unknown" => "unknown".to_string(),
        value => value.to_string(),
    }
}

fn progress_label(expression: &str) -> String {
    let expression = expression.trim();
    if expression.contains("total_online_seconds") {
        "累计在线秒数".to_string()
    } else if expression.contains("parts_earned") {
        "累计零件".to_string()
    } else if expression.contains("insight_earned") {
        "累计灵感".to_string()
    } else if expression.contains("keyboard_press_count")
        || expression.contains("mouse_click_count")
    {
        "输入次数".to_string()
    } else if expression.contains("disk_bytes_total") || expression.contains("network_bytes_total")
    {
        "数据流量".to_string()
    } else if expression.contains("rarity_rank") || expression.contains("worklog.rarity.max_rank") {
        "工况卡等级".to_string()
    } else if expression.contains("rarity_score") || expression.contains("worklog.rarity.max_score")
    {
        "工况卡稀有度分".to_string()
    } else if expression.contains("title_level") {
        "工况职级等级".to_string()
    } else if expression.contains("workshop.level") {
        "工坊等级".to_string()
    } else {
        expression.to_string()
    }
}

fn increment_payload_counter(
    book: &mut AchievementBook,
    record: &AchievementEventRecord,
    payload_key: &str,
    counter_key: &str,
) {
    if let Some(value) = payload_f64(record, payload_key) {
        increment_counter(book, counter_key, value);
    }
}

fn increment_counter(book: &mut AchievementBook, key: &str, delta: f64) {
    let value = book.counters.entry(key.to_string()).or_insert(0.0);
    *value += delta.max(0.0);
}

fn set_counter(book: &mut AchievementBook, key: &str, value: f64) {
    book.counters.insert(key.to_string(), value.max(0.0));
}

fn set_counter_max(book: &mut AchievementBook, key: &str, value: f64) {
    let current = counter_value(book, key);
    if value > current {
        set_counter(book, key, value);
    }
}

fn counter_value(book: &AchievementBook, key: &str) -> f64 {
    book.counters.get(key).copied().unwrap_or(0.0)
}

fn insert_distinct_value(book: &mut AchievementBook, key: &str, value: &str) {
    book.distinct_values
        .entry(key.to_string())
        .or_default()
        .insert(value.to_string());
}

fn payload_f64(record: &AchievementEventRecord, key: &str) -> Option<f64> {
    record.payload.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_u64().map(|number| number as f64))
            .or_else(|| value.as_i64().map(|number| number.max(0) as f64))
    })
}

fn payload_string(record: &AchievementEventRecord, key: &str) -> Option<String> {
    record
        .payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn payload_bool(record: &AchievementEventRecord, key: &str) -> Option<bool> {
    record.payload.get(key).and_then(Value::as_bool)
}

fn trim_events(book: &mut AchievementBook) {
    if book.events.len() <= MAX_STORED_EVENTS {
        return;
    }
    let overflow = book.events.len() - MAX_STORED_EVENTS;
    let expired_keys = book
        .events
        .drain(0..overflow)
        .map(|event| event.idempotency_key)
        .collect::<Vec<_>>();
    for key in expired_keys {
        book.idempotency_keys.remove(&key);
    }
}

fn increment_bucket(
    buckets: &mut BTreeMap<String, AchievementBucketSummary>,
    key: &str,
    is_unlocked: bool,
) {
    let bucket = buckets
        .entry(key.to_string())
        .or_insert(AchievementBucketSummary {
            unlocked: 0,
            total: 0,
        });
    bucket.total += 1;
    if is_unlocked {
        bucket.unlocked += 1;
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::achievements::{load_seed_definitions, validate_definitions};

    fn stored_event(index: usize) -> AchievementEventRecord {
        AchievementEventRecord {
            event_id: format!("event-{index}"),
            event_name: "test".to_string(),
            occurred_at: index as i64,
            received_at: index as i64,
            source: "test".to_string(),
            idempotency_key: format!("key-{index}"),
            payload: json!({}),
            app_version: "test".to_string(),
        }
    }

    fn populate_compiled_condition(book: &mut AchievementBook, condition: &AchievementCondition) {
        match condition {
            AchievementCondition::All { conditions } | AchievementCondition::Any { conditions } => {
                for condition in conditions {
                    populate_compiled_condition(book, condition);
                }
            }
            AchievementCondition::Counter { counter, op, value } => {
                book.counters
                    .insert(counter.clone(), satisfying_value(*op, *value));
            }
            AchievementCondition::Sum {
                counters,
                op,
                value,
            }
            | AchievementCondition::Max {
                counters,
                op,
                value,
            } => {
                for counter in counters {
                    book.counters
                        .insert(counter.clone(), satisfying_value(*op, *value));
                }
            }
            AchievementCondition::DistinctCount {
                key, filter, value, ..
            } => {
                let values = book.distinct_values.entry(key.clone()).or_default();
                match filter {
                    AchievementDistinctFilter::Any => {
                        for index in 0..*value {
                            values.insert(format!("value-{index}"));
                        }
                    }
                    AchievementDistinctFilter::In { values: allowed } => {
                        values.extend(allowed.iter().take(*value as usize).cloned());
                    }
                    AchievementDistinctFilter::NotEqual { value: blocked } => {
                        for index in 0..*value {
                            values.insert(format!("allowed-{index}-{blocked}"));
                        }
                    }
                }
            }
            AchievementCondition::CalendarDays { .. }
            | AchievementCondition::ConsecutiveDays { .. }
            | AchievementCondition::CalendarMonths { .. }
            | AchievementCondition::PerBucketMin { .. }
            | AchievementCondition::ExcludeSelf { .. } => {}
        }
    }

    fn satisfying_value(operator: AchievementOperator, target: f64) -> f64 {
        match operator {
            AchievementOperator::GreaterThanOrEqual | AchievementOperator::Equal => target,
            AchievementOperator::GreaterThan => target + 1.0,
            AchievementOperator::LessThanOrEqual => target,
            AchievementOperator::LessThan => (target - 1.0).max(0.0),
        }
    }

    fn saturated_book(definitions: &[AchievementDefinition]) -> AchievementBook {
        let mut book = AchievementBook::default();
        for definition in definitions {
            populate_compiled_condition(&mut book, &definition.condition);
            book.unlocks.insert(
                definition.id.clone(),
                AchievementUnlockRecord {
                    unlock_id: format!("unlock-{}", definition.id),
                    achievement_id: definition.id.clone(),
                    unlocked_at: 1,
                    points_awarded: definition.points,
                    source_event_id: "fixture".to_string(),
                    progress_snapshot: BTreeMap::new(),
                },
            );
        }

        for key in workshop_track_keys() {
            book.counters.insert(key, 100.0);
        }
        for family in work_day_title_families() {
            book.counters
                .insert(format!("worklog.title_level.{family}"), 5.0);
        }
        for state in cocat_animation_states() {
            book.counters.insert(
                format!("cocat.animation_seen.count(animationState='{state}')"),
                500.0,
            );
        }

        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let day_types = report_day_types();
        for offset in 0..420 {
            let date = start + chrono::Duration::days(offset);
            book.daily_rollups.insert(
                date.format("%Y-%m-%d").to_string(),
                AchievementDailyRollup {
                    active_seconds: 10_000.0,
                    high_load_seconds: 10_000.0,
                    thermal_warning_seconds: 0.0,
                    active_00_05_seconds: 4_000.0,
                    low_power_mode_enabled_seconds: 4_000.0,
                    report_generated: true,
                    report_score: Some(100.0),
                    report_day_type: Some(day_types[offset as usize % day_types.len()].to_string()),
                    rarity_rank: Some(5.0),
                    ..Default::default()
                },
            );
        }
        book
    }

    fn assert_compiled_matches_legacy(
        book: &AchievementBook,
        definitions: &[AchievementDefinition],
    ) {
        for definition in definitions {
            let compiled =
                evaluate_compiled_condition(book, definitions, definition, &definition.condition);
            let legacy = evaluate_legacy_condition(book, definitions, definition);
            assert_eq!(
                compiled.as_ref().map(|progress| progress.is_complete),
                legacy.as_ref().map(|progress| progress.is_complete),
                "compiled mismatch for {}: {} (compiled={compiled:?}, legacy={legacy:?})",
                definition.id,
                definition.condition_summary
            );
        }
    }

    #[test]
    fn all_seed_conditions_compile_and_match_legacy_results() {
        let definitions = load_seed_definitions().unwrap();
        assert_compiled_matches_legacy(&AchievementBook::default(), &definitions);

        let book = saturated_book(&definitions);
        assert_compiled_matches_legacy(&book, &definitions);
        for definition in &definitions {
            assert!(
                evaluate_compiled_condition(
                    &book,
                    &definitions,
                    definition,
                    &definition.condition,
                )
                .is_some_and(|progress| progress.is_complete),
                "saturated fixture did not satisfy {}: {}",
                definition.id,
                definition.condition_summary
            );
        }
    }

    #[test]
    fn compaction_discards_unbounded_legacy_idempotency_keys() {
        let mut book = AchievementBook {
            events: vec![stored_event(1), stored_event(2)],
            idempotency_keys: ["key-1", "key-2", "stale-key"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            ..Default::default()
        };

        assert!(compact_achievement_book(&mut book));
        assert_eq!(
            book.idempotency_keys,
            ["key-1", "key-2"].into_iter().map(str::to_string).collect()
        );
        assert!(!compact_achievement_book(&mut book));
    }

    #[test]
    fn event_retention_also_bounds_idempotency_keys() {
        let events = (0..=MAX_STORED_EVENTS)
            .map(stored_event)
            .collect::<Vec<_>>();
        let mut book = AchievementBook {
            idempotency_keys: events
                .iter()
                .map(|event| event.idempotency_key.clone())
                .collect(),
            events,
            ..Default::default()
        };

        trim_events(&mut book);

        assert_eq!(book.events.len(), MAX_STORED_EVENTS);
        assert_eq!(book.idempotency_keys.len(), MAX_STORED_EVENTS);
        assert!(!book.idempotency_keys.contains("key-0"));
    }

    #[test]
    fn event_unlocks_simple_counter_achievement_once() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();
        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "app.launch".to_string(),
                occurred_at: 1,
                idempotency_key: "launch-1".to_string(),
                payload: json!({}),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response.accepted);
        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A001"));
        assert!(response
            .unlocked
            .iter()
            .any(|event| event.cocat_animation_state == "achievementPop"));
        assert!(book.unlocks.contains_key("A001"));

        let duplicate = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "app.launch".to_string(),
                occurred_at: 1,
                idempotency_key: "launch-1".to_string(),
                payload: json!({}),
                source: "test".to_string(),
            },
            2,
            "0.1.9",
        );

        assert!(!duplicate.accepted);
        assert_eq!(book.counters.get("app.launch.count"), Some(&1.0));
    }

    #[test]
    fn event_unlocks_online_seconds_achievement() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "app.active_minute".to_string(),
                occurred_at: 1,
                idempotency_key: "active-1".to_string(),
                payload: json!({ "seconds": 1_800 }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A002"));
    }

    #[test]
    fn page_view_updates_page_specific_and_distinct_counters() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "page.view".to_string(),
                occurred_at: 1,
                idempotency_key: "page-dashboard".to_string(),
                payload: json!({ "pageKey": "dashboard" }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A004"));
        assert_eq!(
            book.counters.get("page.view.count(pageKey='dashboard')"),
            Some(&1.0)
        );
        assert_eq!(
            book.distinct_values
                .get("page.view.pageKey")
                .map(BTreeSet::len),
            Some(1)
        );
    }

    #[test]
    fn summary_excludes_locked_hidden_cards() {
        let definitions = load_seed_definitions().unwrap();
        validate_definitions(&definitions).unwrap();
        let book = AchievementBook::default();

        let cards = list_achievement_cards(&book, &definitions, true);
        let summary = summarize_achievements(&book, &definitions);

        assert_eq!(cards.len(), 123);
        assert_eq!(summary.hidden_total_count, 9);
        assert_eq!(summary.total_points, 0);
    }

    #[test]
    fn distinct_count_condition_unlocks_gallery_explorer() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();
        let pages = [
            "dashboard",
            "workshop",
            "devices",
            "worklog",
            "settings",
            "about",
            "achievements",
        ];

        for (index, page) in pages.iter().enumerate() {
            record_achievement_event(
                &mut book,
                &definitions,
                TrackAchievementEventRequest {
                    event_name: "page.view".to_string(),
                    occurred_at: index as i64,
                    idempotency_key: format!("page-{page}"),
                    payload: json!({ "pageKey": page }),
                    source: "test".to_string(),
                },
                index as i64 + 1,
                "0.1.9",
            );
        }

        assert!(book.unlocks.contains_key("A026"));
    }

    #[test]
    fn achievement_cards_include_counter_progress() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "app.active_minute".to_string(),
                occurred_at: 1,
                idempotency_key: "active-half-hour-progress".to_string(),
                payload: json!({ "seconds": 900, "date": "2026-01-01" }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        let cards = list_achievement_cards(&book, &definitions, true);
        let companion = cards
            .iter()
            .find(|card| card.achievement_id == "A002")
            .expect("A002 card should be visible");
        let progress = companion.progress.as_ref().expect("progress exists");

        assert_eq!(progress.current, 900.0);
        assert_eq!(progress.target, 1_800.0);
        assert_eq!(progress.percent, 50.0);
        assert!(!progress.is_complete);
    }

    #[test]
    fn sum_condition_unlocks_input_milestone() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "hardware.segment_rollup".to_string(),
                occurred_at: 1,
                idempotency_key: "input-sum-1".to_string(),
                payload: json!({
                    "keyboardPressCount": 1_000,
                    "mouseClickCount": 500,
                    "date": "2026-01-01"
                }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A039"));
    }

    #[test]
    fn valid_click_bursts_unlock_a080_and_invalid_bursts_do_not_count() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        for index in 0..9 {
            let response = record_achievement_event(
                &mut book,
                &definitions,
                TrackAchievementEventRequest {
                    event_name: "pet.click_burst".to_string(),
                    occurred_at: index + 1,
                    idempotency_key: format!("valid-click-burst-{index}"),
                    payload: json!({ "clicks": 3, "windowMs": 1_500 }),
                    source: "test".to_string(),
                },
                index + 1,
                "0.1.9",
            );
            assert!(!response
                .unlocked
                .iter()
                .any(|event| event.achievement_id == "A080"));
        }

        for (index, payload) in [
            json!({ "clicks": 2, "windowMs": 1_000 }),
            json!({ "clicks": 3, "windowMs": 2_001 }),
        ]
        .into_iter()
        .enumerate()
        {
            record_achievement_event(
                &mut book,
                &definitions,
                TrackAchievementEventRequest {
                    event_name: "pet.click_burst".to_string(),
                    occurred_at: 20 + index as i64,
                    idempotency_key: format!("invalid-click-burst-{index}"),
                    payload,
                    source: "test".to_string(),
                },
                20 + index as i64,
                "0.1.9",
            );
        }
        assert!(!book.unlocks.contains_key("A080"));

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "pet.click_burst".to_string(),
                occurred_at: 30,
                idempotency_key: "valid-click-burst-10".to_string(),
                payload: json!({ "clicks": 3, "windowMs": 1_999 }),
                source: "test".to_string(),
            },
            30,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A080"));
        assert_eq!(
            book.counters
                .get("pet.click_burst.count(clicks >= 3, windowMs <= 2000)"),
            Some(&10.0),
        );
    }

    #[test]
    fn consecutive_days_condition_unlocks_streak() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        for (index, date) in ["2026-01-01", "2026-01-02", "2026-01-03"]
            .iter()
            .enumerate()
        {
            record_achievement_event(
                &mut book,
                &definitions,
                TrackAchievementEventRequest {
                    event_name: "app.active_minute".to_string(),
                    occurred_at: index as i64,
                    idempotency_key: format!("active-day-{date}"),
                    payload: json!({ "seconds": 1_800, "date": date }),
                    source: "test".to_string(),
                },
                index as i64 + 1,
                "0.1.9",
            );
        }

        assert!(book.unlocks.contains_key("A022"));
        assert!(book.unlocks.contains_key("A023"));
    }

    #[test]
    fn daily_work_rarity_unlocks_worklog_card_achievements() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "worklog.daily_generated".to_string(),
                occurred_at: 1,
                idempotency_key: "worklog-rarity-ss".to_string(),
                payload: json!({
                    "date": "2026-01-01",
                    "score": 95,
                    "dayType": "PressureRepair",
                    "rarityTier": "SS",
                    "rarityScore": 90,
                    "titleFamily": "pressure",
                    "titleName": "高压修复师",
                    "titleLevel": 2,
                    "titleProgress": 3
                }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A124"));
        assert!(book.unlocks.contains_key("A121"));
        assert!(book.unlocks.contains_key("A122"));
        assert!(book.unlocks.contains_key("A123"));
        assert!(book.unlocks.contains_key("A124"));
    }

    #[test]
    fn daily_work_title_unlocks_pressure_repair_title_achievement() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();

        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "worklog.daily_generated".to_string(),
                occurred_at: 1,
                idempotency_key: "worklog-pressure-title-lv2".to_string(),
                payload: json!({
                    "date": "2026-01-02",
                    "score": 78,
                    "dayType": "PressureRepair",
                    "rarityTier": "A",
                    "rarityScore": 72,
                    "titleFamily": "pressure",
                    "titleName": "高压修复师",
                    "titleLevel": 2,
                    "titleProgress": 3
                }),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert!(response
            .unlocked
            .iter()
            .any(|event| event.achievement_id == "A127"));
        assert!(book.unlocks.contains_key("A127"));
    }

    #[test]
    fn weekly_goals_are_stable_until_the_iso_week_changes() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();
        book.counters
            .insert("lifetime.total_online_seconds".to_string(), 10_000.0);
        book.counters
            .insert("worklog.daily_generated.count".to_string(), 20.0);
        book.counters
            .insert("page.view.count(pageKey='workshop')".to_string(), 30.0);
        let monday = Local
            .with_ymd_and_hms(2026, 8, 31, 9, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();

        let (first, changed) = ensure_weekly_goals(&mut book, &definitions, monday);
        assert!(changed);
        assert_eq!(first.goals.len(), 3);
        let first_ids = first
            .goals
            .iter()
            .map(|goal| goal.goal_id.clone())
            .collect::<Vec<_>>();
        assert!(first.goals.iter().all(|goal| goal.percent == 0.0));

        let progress = WeeklyProgress { online_seconds: 7200.0, ..Default::default() };
        let (same_week, changed) =
            ensure_weekly_goals_with_progress(&mut book, &definitions, monday + 86_400_000, &progress);
        assert!(!changed);
        assert_eq!(
            same_week
                .goals
                .iter()
                .map(|goal| goal.goal_id.clone())
                .collect::<Vec<_>>(),
            first_ids
        );
        assert!(same_week.goals[0].is_complete);
        assert_eq!(same_week.goals[0].percent, 100.0);

        let (next_week, changed) =
            ensure_weekly_goals(&mut book, &definitions, monday + 7 * 86_400_000);
        assert!(changed);
        assert_ne!(next_week.week_key, first.week_key);
        assert_eq!(next_week.goals.len(), 3);
        assert_eq!(
            next_week
                .goals
                .iter()
                .map(|goal| goal.goal_id.clone())
                .collect::<Vec<_>>(),
            first_ids
        );
        assert!(next_week.goals.iter().all(|goal| goal.percent == 0.0));
    }

    #[test]
    fn legacy_weekly_goal_plan_is_regenerated_with_weekly_baselines() {
        let definitions = load_seed_definitions().unwrap();
        let now = Local
            .with_ymd_and_hms(2026, 8, 31, 9, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let mut book = AchievementBook {
            weekly_goal_plan: Some(WeeklyGoalPlan {
                week_key: local_iso_week(now).0,
                achievement_ids: vec!["A002".to_string(), "A003".to_string(), "A005".to_string()],
                generated_at: now - 1,
                ..Default::default()
            }),
            ..Default::default()
        };

        let (goals, changed) = ensure_weekly_goals(&mut book, &definitions, now);

        assert!(changed);
        assert_eq!(goals.goals.len(), 3);
        let plan = book.weekly_goal_plan.unwrap();
        assert!(plan.achievement_ids.is_empty());
        assert_eq!(plan.goal_keys.len(), 3);
        assert_eq!(plan.baselines.len(), 3);
    }

    #[test]
    fn unlock_notifications_are_persisted_and_marked_seen() {
        let definitions = load_seed_definitions().unwrap();
        let mut book = AchievementBook::default();
        let response = record_achievement_event(
            &mut book,
            &definitions,
            TrackAchievementEventRequest {
                event_name: "app.launch".to_string(),
                occurred_at: 1,
                idempotency_key: "launch-notification".to_string(),
                payload: json!({}),
                source: "test".to_string(),
            },
            1,
            "0.1.9",
        );

        assert_eq!(response.unlocked.len(), 1);
        let summary = summarize_achievements(&book, &definitions);
        assert_eq!(summary.pending_notification_count, 1);

        let unlock_id = response.unlocked[0].unlock_id.clone();
        let changed =
            mark_achievement_notifications_seen(&mut book, Some(vec![unlock_id.clone()]), 2);
        assert!(changed >= 1);
        assert_eq!(
            summarize_achievements(&book, &definitions).pending_notification_count,
            0
        );
        assert!(!book.notification_queue.contains(&unlock_id));
        assert!(book
            .notifications
            .values()
            .any(|notification| notification.unlock_id == unlock_id
                && notification.state == AchievementNotificationState::Seen
                && notification.seen_at == Some(2)));
    }
}
