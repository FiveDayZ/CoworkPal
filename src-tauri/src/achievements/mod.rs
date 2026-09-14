pub mod definitions;
pub mod runtime;
pub mod seed;
pub mod validation;

pub use definitions::{
    AchievementCategory, AchievementCondition, AchievementDefinition, AchievementDifficulty,
    AchievementGrantType, AchievementOperator, AchievementRepeatPolicy, AchievementScope,
    AchievementSortMode,
};
pub use runtime::{
    compact_achievement_book, ensure_weekly_goals, get_achievement_card, list_achievement_cards,
    mark_achievement_notifications_seen, record_achievement_event, summarize_achievements,
    AchievementBook, AchievementCard, AchievementDailyRollup, AchievementEventRecord,
    AchievementNotificationRecord, AchievementNotificationState, AchievementSummary,
    AchievementUnlockRecord, AchievementUnlockedEvent, TrackAchievementEventRequest,
    TrackAchievementEventResponse, WeeklyGoal, WeeklyGoalPlan, WeeklyGoals,
    ACHIEVEMENT_BOOK_SCHEMA_VERSION,
};
pub use seed::{load_seed_definitions, seed_definitions, AchievementSeedError};
pub use validation::{validate_definitions, AchievementValidationError};
