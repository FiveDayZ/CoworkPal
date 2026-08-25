use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementDifficulty {
    Entry,
    Normal,
    Skilled,
    Elite,
    Epic,
    Legendary,
}

impl AchievementDifficulty {
    pub fn key(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Normal => "normal",
            Self::Skilled => "skilled",
            Self::Elite => "elite",
            Self::Epic => "epic",
            Self::Legendary => "legendary",
        }
    }

    pub fn points(self) -> u32 {
        match self {
            Self::Entry => 5,
            Self::Normal => 10,
            Self::Skilled => 20,
            Self::Elite => 35,
            Self::Epic => 60,
            Self::Legendary => 100,
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            Self::Entry => 1,
            Self::Normal => 2,
            Self::Skilled => 4,
            Self::Elite => 7,
            Self::Epic => 12,
            Self::Legendary => 20,
        }
    }

    pub fn from_section_title(title: &str) -> Option<Self> {
        if title.contains("入门") {
            Some(Self::Entry)
        } else if title.contains("进阶") {
            Some(Self::Normal)
        } else if title.contains("熟练") {
            Some(Self::Skilled)
        } else if title.contains("精英") {
            Some(Self::Elite)
        } else if title.contains("史诗") {
            Some(Self::Epic)
        } else if title.contains("传说") {
            Some(Self::Legendary)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementCategory {
    DailyUse,
    TaskEfficiency,
    LongStreak,
    FeatureExploration,
    DataMilestone,
    WorkshopGrowth,
    HardwareHealth,
    SocialCollaboration,
    HiddenEaster,
}

impl AchievementCategory {
    pub fn key(self) -> &'static str {
        match self {
            Self::DailyUse => "daily_use",
            Self::TaskEfficiency => "task_efficiency",
            Self::LongStreak => "long_streak",
            Self::FeatureExploration => "feature_exploration",
            Self::DataMilestone => "data_milestone",
            Self::WorkshopGrowth => "workshop_growth",
            Self::HardwareHealth => "hardware_health",
            Self::SocialCollaboration => "social_collaboration",
            Self::HiddenEaster => "hidden_easter",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim() {
            "日常使用类" => Some(Self::DailyUse),
            "任务效率类" => Some(Self::TaskEfficiency),
            "长期打卡类" => Some(Self::LongStreak),
            "功能探索类" => Some(Self::FeatureExploration),
            "数据里程碑类" => Some(Self::DataMilestone),
            "工坊养成类" => Some(Self::WorkshopGrowth),
            "硬件健康类" => Some(Self::HardwareHealth),
            "社交协作类" => Some(Self::SocialCollaboration),
            "隐藏彩蛋类" => Some(Self::HiddenEaster),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementRepeatPolicy {
    Once,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementGrantType {
    Auto,
    AdminRepair,
    Migration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementScope {
    Lifetime,
    Daily,
    Monthly,
    Rolling,
    Distinct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementSortMode {
    DisplayOrder,
    UnlockedAtDesc,
    DifficultyDesc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementOperator {
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AchievementCondition {
    All {
        conditions: Vec<AchievementCondition>,
    },
    Any {
        conditions: Vec<AchievementCondition>,
    },
    Counter {
        counter: String,
        op: AchievementOperator,
        value: f64,
    },
    DistinctCount {
        key: String,
        op: AchievementOperator,
        value: u32,
    },
    ConsecutiveDays {
        key: String,
        min_daily_value: f64,
        days: u32,
    },
    PerBucketMin {
        key: String,
        bucket_count: u32,
        min_value: f64,
    },
    ExcludeSelf {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementDefinition {
    pub id: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub category: AchievementCategory,
    pub difficulty: AchievementDifficulty,
    pub points: u32,
    pub badge_key: String,
    pub is_hidden: bool,
    pub repeat_policy: AchievementRepeatPolicy,
    pub condition_summary: String,
    pub enabled_from_version: String,
    pub enabled_to_version: Option<String>,
    pub definition_version: u32,
    pub display_order: u32,
}

impl AchievementDefinition {
    pub fn new(
        id: String,
        title: String,
        category: AchievementCategory,
        difficulty: AchievementDifficulty,
        points: u32,
        badge_key: String,
        is_hidden: bool,
        condition_summary: String,
        display_order: u32,
    ) -> Self {
        let code = id.to_ascii_lowercase();
        let description = condition_summary.clone();

        Self {
            id,
            code,
            title,
            description,
            category,
            difficulty,
            points,
            badge_key,
            is_hidden,
            repeat_policy: AchievementRepeatPolicy::Once,
            condition_summary,
            enabled_from_version: "0.1.9".to_string(),
            enabled_to_version: None,
            definition_version: 1,
            display_order,
        }
    }
}
