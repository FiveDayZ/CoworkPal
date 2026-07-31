//! Local, rule-based suggestion engine.
//!
//! Generates advice from the user's own history + current snapshot. No network,
//! no telemetry — every rule is a pure predicate over local data. The output
//! reuses the `Suggestion` shape so the frontend can render it like the
//! existing `AssessmentInsight` lists.
//!
//! Rules are intentionally conservative: when there isn't enough signal (e.g.
//! fewer than 3 days of history), rules return `None` rather than guessing.

use chrono::{Datelike, Duration, Local};

use crate::models::{
    current_timestamp_ms, today_key, HardwareSnapshot, Suggestion, SuggestionCategory,
    SuggestionSeverity, TodaySuggestions, WorkLogBook, WorkLogEntry, WorkLogReport,
};

/// Maximum suggestions returned in `all`, and in `top`.
const MAX_ALL: usize = 5;
const MAX_TOP: usize = 3;

/// Recent-history window the rules look at.
const HISTORY_DAYS: i64 = 7;

impl TodaySuggestions {
    /// Build today's suggestions from local state. `book` is the full work-log
    /// history; `current` is the latest hardware snapshot (may be None at boot).
    pub fn from_local(book: &WorkLogBook, current: Option<&HardwareSnapshot>) -> Self {
        let today = today_key();
        let recent = recent_entries(book, HISTORY_DAYS);
        let today_entry = book.entries.get(&today);

        let mut suggestions: Vec<Suggestion> = Vec::new();
        // Each rule is a pure function returning Option<Suggestion>; push if fired.
        push_if(&mut suggestions, rule_focus_gap(&recent));
        push_if(&mut suggestions, rule_sustained_high_load(&recent));
        push_if(&mut suggestions, rule_memory_pressure(&recent, current));
        push_if(&mut suggestions, rule_thermal_rhythm(&recent));
        push_if(&mut suggestions, rule_streak_encouragement(&recent));
        push_if(&mut suggestions, rule_idle_long_session(today_entry, current));
        push_if(&mut suggestions, rule_stability_drop(&recent));

        // Sort by priority desc, then truncate. Keep only the best per category
        // so the list stays varied.
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        dedupe_by_category(&mut suggestions);
        suggestions.truncate(MAX_ALL);

        let top = suggestions.iter().take(MAX_TOP).cloned().collect();

        Self {
            date: today,
            top,
            all: suggestions,
            generated_at: current_timestamp_ms(),
        }
    }
}

fn push_if(out: &mut Vec<Suggestion>, suggestion: Option<Suggestion>) {
    if let Some(s) = suggestion {
        out.push(s);
    }
}

/// Entries from the last `days` calendar days, oldest first, with signal only.
fn recent_entries(book: &WorkLogBook, days: i64) -> Vec<(String, WorkLogEntry)> {
    let today = Local::now().date_naive();
    let mut out = Vec::new();
    for offset in (0..days).rev() {
        let key = (today - Duration::days(offset)).format("%Y-%m-%d").to_string();
        if let Some(entry) = book.entries.get(&key) {
            if entry_has_signal(entry) {
                out.push((key, entry.clone()));
            }
        }
    }
    out
}

fn entry_has_signal(entry: &WorkLogEntry) -> bool {
    entry.sample_count > 0
        || entry.active_seconds > 0
        || entry.mouse_click_count > 0
        || entry.keyboard_press_count > 0
}

/// Drop lower-priority suggestions that share a category with a higher one.
fn dedupe_by_category(suggestions: &mut Vec<Suggestion>) {
    // Input is already priority-sorted; keep the first occurrence of each category.
    let mut seen = std::collections::HashSet::new();
    suggestions.retain(|s| seen.insert(s.category));
}

// --- helpers for reading derived metrics off an entry ---

fn high_load_ratio(entry: &WorkLogEntry) -> f64 {
    if entry.active_seconds == 0 {
        0.0
    } else {
        // Clamp to [0, 1] in case of dirty data where high_load_seconds somehow
        // exceeds active_seconds (e.g. accumulated from overlapping ticks) — an
        // out-of-range ratio would mislead the sustained-high-load suggestion.
        (entry.high_load_seconds as f64 / entry.active_seconds as f64).clamp(0.0, 1.0)
    }
}

fn thermal_avg(entry: &WorkLogEntry) -> f64 {
    let samples = entry.sample_count.max(1) as f64;
    entry.thermal_pressure_points / samples
}

fn active_hours(entry: &WorkLogEntry) -> f64 {
    entry.active_seconds as f64 / 3600.0
}

// --- rules ---

/// If the user hasn't started a focus session in the last week, nudge them.
fn rule_focus_gap(recent: &[(String, WorkLogEntry)]) -> Option<Suggestion> {
    // We can't see focus sessions directly from WorkLogEntry, but a week with
    // high active hours and zero focus is a strong signal the ritual is unused.
    // Heuristic: 3+ days with >3h activity but we have no focus markers.
    // Since WorkLogEntry doesn't track focus starts, this rule keys off the
    // activity pattern: long busy days without breaks suggest no focus ritual.
    let busy_days = recent
        .iter()
        .filter(|(_, e)| active_hours(e) > 3.0)
        .count();
    if recent.len() >= 3 && busy_days >= 3 {
        Some(Suggestion {
            id: "focus_gap".to_string(),
            category: SuggestionCategory::FocusHabit,
            priority: 60,
            title: "试试专注仪式".to_string(),
            body: "最近几天工作时长都不短，但似乎还没用过专注模式。25 分钟番茄钟能帮你保持节奏。".to_string(),
            action_hint: "在控制台点开「专注仪式」，挑一个任务开始。".to_string(),
            severity: SuggestionSeverity::Neutral,
        })
    } else {
        None
    }
}

/// Several consecutive days with high_load_ratio > 0.4 → suggest a light day.
fn rule_sustained_high_load(recent: &[(String, WorkLogEntry)]) -> Option<Suggestion> {
    let consecutive_high = recent
        .iter()
        .rev()
        .take_while(|(_, e)| high_load_ratio(e) > 0.4)
        .count();
    if consecutive_high >= 3 {
        Some(Suggestion {
            id: "sustained_high_load".to_string(),
            category: SuggestionCategory::Workload,
            priority: 75,
            title: "本周持续高负载".to_string(),
            body: format!(
                "已经连续 {} 天高负载占比偏高，硬件和人都需要喘口气。",
                consecutive_high
            ),
            action_hint: "今天安排一个轻量任务，或让 CoreCat 待机降温。".to_string(),
            severity: SuggestionSeverity::Warning,
        })
    } else {
        None
    }
}

/// Memory crowded right now, or repeatedly over the last few days.
fn rule_memory_pressure(
    recent: &[(String, WorkLogEntry)],
    current: Option<&HardwareSnapshot>,
) -> Option<Suggestion> {
    // Live pressure takes precedence.
    let live_high = current.and_then(|s| s.memory_usage_percent).is_some_and(|m| m > 80.0);
    let hot_days = recent
        .iter()
        .filter(|(_, e)| {
            let samples = e.sample_count.max(1) as f64;
            e.memory_load_points / samples > 75.0
        })
        .count();
    if live_high || hot_days >= 3 {
        Some(Suggestion {
            id: "memory_pressure".to_string(),
            category: SuggestionCategory::Memory,
            priority: if live_high { 80 } else { 55 },
            title: "内存吃紧".to_string(),
            body: if live_high {
                "当前内存占用偏高，CoreCat 被挤到角落了。".to_string()
            } else {
                format!("近 {} 天内存常处于高位，可能有常驻进程占资源。", hot_days)
            },
            action_hint: "用托盘菜单或设置面板执行一次内存释放，或检查后台进程。".to_string(),
            severity: SuggestionSeverity::Warning,
        })
    } else {
        None
    }
}

/// Thermal pressure concentrated — if the last few days all ran hot, advise
/// cooling/airflow. Uses thermal_pressure_points average.
fn rule_thermal_rhythm(recent: &[(String, WorkLogEntry)]) -> Option<Suggestion> {
    let hot_days = recent.iter().filter(|(_, e)| thermal_avg(e) > 30.0).count();
    if recent.len() >= 3 && hot_days >= 3 {
        Some(Suggestion {
            id: "thermal_rhythm".to_string(),
            category: SuggestionCategory::Thermal,
            priority: 70,
            title: "近期温度偏高".to_string(),
            body: format!("最近 {} 天散热压力较大，长时间高温会影响硬件寿命和性能。", hot_days),
            action_hint: "检查进风口/风扇，或在高负载时段降低环境温度。".to_string(),
            severity: SuggestionSeverity::Warning,
        })
    } else {
        None
    }
}

/// Encourage consistency: if the user has a 3+ day streak, cheer them on.
fn rule_streak_encouragement(recent: &[(String, WorkLogEntry)]) -> Option<Suggestion> {
    // `recent` is oldest-first and already filtered to signal-days, but days
    // without data are absent from the vec — so consecutive entries don't imply
    // consecutive calendar days. Count distinct days as a proxy.
    if recent.len() >= 5 {
        Some(Suggestion {
            id: "streak_encouragement".to_string(),
            category: SuggestionCategory::Streak,
            priority: 40,
            title: "坚持得不错！".to_string(),
            body: format!("最近一周有 {} 天和 CoreCat 一起工作，保持这个节奏。", recent.len()),
            action_hint: "继续保持，连续打卡能解锁更多成就。".to_string(),
            severity: SuggestionSeverity::Positive,
        })
    } else {
        None
    }
}

/// Currently in a long idle stretch with the machine still on — suggest a break.
fn rule_idle_long_session(
    today_entry: Option<&WorkLogEntry>,
    current: Option<&HardwareSnapshot>,
) -> Option<Suggestion> {
    // Low current input activity (CPU low) + already several hours logged today.
    let cpu_low = current.and_then(|s| s.cpu_usage_percent).is_some_and(|c| c < 15.0);
    let hours_today = today_entry.map(active_hours).unwrap_or(0.0);
    if cpu_low && hours_today > 5.0 {
        Some(Suggestion {
            id: "idle_long_session".to_string(),
            category: SuggestionCategory::FocusHabit,
            priority: 50,
            title: "歇一会儿吧".to_string(),
            body: format!("今天已经在线 {:.1} 小时，现在负载很低，适合起身活动一下。", hours_today),
            action_hint: "离开屏幕 5 分钟，CoreCat 会替你看着。".to_string(),
            severity: SuggestionSeverity::Neutral,
        })
    } else {
        None
    }
}

/// Stability score dropped notably vs the start of the window.
fn rule_stability_drop(recent: &[(String, WorkLogEntry)]) -> Option<Suggestion> {
    if recent.len() < 4 {
        return None;
    }
    // Compare the average stability dimension of the most recent 2 days vs the
    // earlier days in the window.
    let stability_of = |e: &WorkLogEntry| {
        WorkLogReport::from_entry(e.clone())
            .dimensions
            .iter()
            .find(|d| d.key == "stability")
            .map(|d| d.score)
            .unwrap_or(0) as f64
    };
    let recent_avg: f64 =
        recent.iter().rev().take(2).map(|(_, e)| stability_of(e)).sum::<f64>() / 2.0;
    let earlier: Vec<&WorkLogEntry> = recent.iter().rev().skip(2).map(|(_, e)| e).collect();
    let earlier_avg: f64 = earlier.iter().map(|e| stability_of(e)).sum::<f64>() / earlier.len() as f64;
    if earlier_avg - recent_avg >= 3.0 {
        Some(Suggestion {
            id: "stability_drop".to_string(),
            category: SuggestionCategory::Thermal,
            priority: 65,
            title: "系统稳定度下降".to_string(),
            body: format!(
                "近两天的稳定度比之前低了约 {:.0} 分，可能是散热或电源管理变化。",
                earlier_avg - recent_avg
            ),
            action_hint: "关注机箱风道、电源模式，或检查是否有新进程常驻。".to_string(),
            severity: SuggestionSeverity::Warning,
        })
    } else {
        None
    }
}

/// The weekday of a YYYY-MM-DD key (0=Mon..6=Sun), for future rhythm rules.
#[allow(dead_code)]
fn weekday_of_key(date: &str) -> Option<u32> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .map(|d| d.weekday().num_days_from_monday())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WorkLogBook;
    use std::collections::BTreeMap;

    fn entry(active_seconds: u64, high_load_seconds: u64, samples: u64, thermal: f64) -> WorkLogEntry {
        WorkLogEntry {
            active_seconds,
            high_load_seconds,
            sample_count: samples,
            thermal_pressure_points: thermal * samples.max(1) as f64,
            ..Default::default()
        }
    }

    fn book_with_recent(days: i64, mut make: impl FnMut(i64) -> WorkLogEntry) -> WorkLogBook {
        let today = Local::now().date_naive();
        let mut entries = BTreeMap::new();
        for offset in (0..days).rev() {
            let key = (today - Duration::days(offset)).format("%Y-%m-%d").to_string();
            entries.insert(key, make(offset));
        }
        WorkLogBook {
            schema_version: 1,
            entries,
        }
    }

    #[test]
    fn empty_book_yields_no_suggestions() {
        let book = WorkLogBook::default();
        let result = TodaySuggestions::from_local(&book, None);
        assert!(result.all.is_empty());
        assert!(result.top.is_empty());
    }

    #[test]
    fn sustained_high_load_fires() {
        // 4 consecutive days, each with high_load_ratio > 0.4.
        let book = book_with_recent(4, |_| entry(4 * 3600, 2 * 3600, 100, 10.0));
        let result = TodaySuggestions::from_local(&book, None);
        assert!(result.all.iter().any(|s| s.id == "sustained_high_load"));
    }

    #[test]
    fn low_load_does_not_fire_high_load_rule() {
        let book = book_with_recent(4, |_| entry(4 * 3600, 60, 100, 10.0));
        let result = TodaySuggestions::from_local(&book, None);
        assert!(!result.all.iter().any(|s| s.id == "sustained_high_load"));
    }

    #[test]
    fn streak_encouragement_fires_after_five_days() {
        let book = book_with_recent(5, |_| entry(2 * 3600, 60, 50, 10.0));
        let result = TodaySuggestions::from_local(&book, None);
        assert!(result.all.iter().any(|s| s.id == "streak_encouragement"));
    }

    #[test]
    fn live_memory_pressure_fires() {
        let book = book_with_recent(1, |_| entry(3600, 60, 50, 10.0));
        let snapshot = HardwareSnapshot {
            memory_usage_percent: Some(88.0),
            ..Default::default()
        };
        let result = TodaySuggestions::from_local(&book, Some(&snapshot));
        assert!(result.all.iter().any(|s| s.id == "memory_pressure" && s.severity == SuggestionSeverity::Warning));
    }

    #[test]
    fn top_is_subset_of_all_and_ordered() {
        let book = book_with_recent(5, |_| entry(4 * 3600, 2 * 3600, 100, 40.0));
        let result = TodaySuggestions::from_local(&book, None);
        assert!(result.top.len() <= MAX_TOP);
        assert!(result.top.len() <= result.all.len());
        // top priorities are non-increasing.
        for w in result.top.windows(2) {
            assert!(w[0].priority >= w[1].priority);
        }
    }

    #[test]
    fn all_has_no_duplicate_categories() {
        let book = book_with_recent(5, |_| entry(4 * 3600, 2 * 3600, 100, 40.0));
        let result = TodaySuggestions::from_local(&book, None);
        let mut cats: Vec<_> = result.all.iter().map(|s| s.category).collect();
        cats.sort();
        let before = cats.len();
        cats.dedup();
        assert_eq!(cats.len(), before, "duplicate categories present");
    }
}
