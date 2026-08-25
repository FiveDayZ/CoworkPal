use std::collections::{BTreeMap, BTreeSet};

use chrono::{Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::definitions::AchievementDefinition;

const MAX_STORED_EVENTS: usize = 2_000;

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
            schema_version: 1,
            events: Vec::new(),
            counters: BTreeMap::new(),
            daily_rollups: BTreeMap::new(),
            distinct_values: BTreeMap::new(),
            unlocks: BTreeMap::new(),
            notifications: BTreeMap::new(),
            notification_queue: Vec::new(),
            idempotency_keys: BTreeSet::new(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementNotificationState {
    Pending,
    Seen,
}

impl Default for AchievementNotificationState {
    fn default() -> Self {
        Self::Pending
    }
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

    if !book.idempotency_keys.insert(request.idempotency_key.clone()) {
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
    let hidden_total_count = definitions.iter().filter(|definition| definition.is_hidden).count();
    let visible_total_count = definitions.iter().filter(|definition| !definition.is_hidden).count();
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

        increment_bucket(
            &mut by_difficulty,
            definition.difficulty.key(),
            is_unlocked,
        );
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
                increment_counter(
                    book,
                    &format!("page.view.count(pageKey='{page_key}')"),
                    1.0,
                );
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
                    set_counter_max(
                        book,
                        &format!("workshop.module_level.{track}"),
                        level,
                    );
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
            let rarity_tier = payload_string(record, "rarityTier")
                .map(|tier| normalize_rarity_tier(&tier));
            let rarity_rank_value = rarity_tier.as_deref().map(rarity_rank);
            let rarity_score = payload_f64(record, "rarityScore");
            let title_family = payload_string(record, "titleFamily")
                .map(|family| normalize_title_family(&family));
            let title_level = payload_f64(record, "titleLevel");
            let title_progress = payload_f64(record, "titleProgress");
            let day_type = payload_string(record, "dayType")
                .map(|day_type| normalize_day_type(&day_type));

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

    increment_payload_counter(book, record, "highLoadSeconds", "lifetime.high_load_seconds");
    increment_payload_counter(book, record, "cpuOver50Seconds", "lifetime.cpu_over_50_seconds");
    increment_payload_counter(
        book,
        record,
        "memoryOver70Seconds",
        "lifetime.memory_over_70_seconds",
    );
    increment_payload_counter(book, record, "gpuOver70Seconds", "lifetime.gpu_over_70_seconds");
    increment_payload_counter(
        book,
        record,
        "thermalWarningSeconds",
        "lifetime.thermal_warning_seconds",
    );
    increment_payload_counter(book, record, "diskBytesTotal", "lifetime.disk_bytes_total");
    increment_payload_counter(book, record, "networkBytesTotal", "lifetime.network_bytes_total");
    increment_payload_counter(book, record, "mouseClickCount", "lifetime.mouse_click_count");
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
    let normalized = definition.condition_summary.replace('`', "");
    let parts = normalized
        .split(" 且 ")
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

    if condition.contains("任意模块任一升级轨等级")
        || condition.contains("第一条满级轨道")
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

    if condition.contains("hidden_achievement.unlocked.count") {
        let count = definitions
            .iter()
            .filter(|candidate| {
                candidate.is_hidden && book.unlocks.contains_key(&candidate.id)
            })
            .count() as f64;
        return Some(compare_progress(
            count,
            operator,
            target,
            "隐藏成就",
        ));
    }

    if condition.contains("non_hidden_achievement.unlocked.count") {
        let count = definitions
            .iter()
            .filter(|candidate| {
                !candidate.is_hidden && book.unlocks.contains_key(&candidate.id)
            })
            .count() as f64;
        return Some(compare_progress(
            count,
            operator,
            target,
            "可见成就",
        ));
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
    Some(compare_progress(current, operator, target, progress_label(&left)))
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
        if daily_rollup_matches(rollup, predicate) {
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
    ConditionProgress::new(
        current,
        target,
        format!("{label}最低值"),
        current >= target,
    )
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
            definition.difficulty.key() == difficulty_key && book.unlocks.contains_key(&definition.id)
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
        "PressureRepair" | "pressure_repair" | "pressureRepair" => {
            "pressureRepair".to_string()
        }
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
    } else if expression.contains("keyboard_press_count") || expression.contains("mouse_click_count") {
        "输入次数".to_string()
    } else if expression.contains("disk_bytes_total") || expression.contains("network_bytes_total") {
        "数据流量".to_string()
    } else if expression.contains("rarity_rank") || expression.contains("worklog.rarity.max_rank") {
        "工况卡等级".to_string()
    } else if expression.contains("rarity_score") || expression.contains("worklog.rarity.max_score") {
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
        let changed = mark_achievement_notifications_seen(
            &mut book,
            Some(vec![unlock_id.clone()]),
            2,
        );
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
