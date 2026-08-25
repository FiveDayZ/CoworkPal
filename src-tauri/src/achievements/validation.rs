use std::{collections::HashSet, fmt};

use super::definitions::{AchievementDefinition, AchievementDifficulty, AchievementRepeatPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementValidationError {
    EmptyDefinitions,
    InvalidId(String),
    DuplicateId(String),
    DuplicateCode(String),
    DuplicateBadgeKey(String),
    InvalidBadgeKey(String),
    InvalidPoints {
        id: String,
        difficulty: AchievementDifficulty,
        expected: u32,
        actual: u32,
    },
    InvalidRepeatPolicy(String),
    EmptyTitle(String),
    EmptyConditionSummary(String),
}

impl fmt::Display for AchievementValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDefinitions => write!(formatter, "achievement definitions are empty"),
            Self::InvalidId(id) => write!(formatter, "invalid achievement id: {id}"),
            Self::DuplicateId(id) => write!(formatter, "duplicate achievement id: {id}"),
            Self::DuplicateCode(code) => write!(formatter, "duplicate achievement code: {code}"),
            Self::DuplicateBadgeKey(badge_key) => {
                write!(formatter, "duplicate achievement badge key: {badge_key}")
            }
            Self::InvalidBadgeKey(badge_key) => {
                write!(formatter, "invalid achievement badge key: {badge_key}")
            }
            Self::InvalidPoints {
                id,
                difficulty,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid points for {id} ({:?}): expected {expected}, actual {actual}",
                difficulty
            ),
            Self::InvalidRepeatPolicy(id) => write!(formatter, "invalid repeat policy for {id}"),
            Self::EmptyTitle(id) => write!(formatter, "empty title for {id}"),
            Self::EmptyConditionSummary(id) => {
                write!(formatter, "empty condition summary for {id}")
            }
        }
    }
}

impl std::error::Error for AchievementValidationError {}

pub fn validate_definitions(
    definitions: &[AchievementDefinition],
) -> Result<(), AchievementValidationError> {
    if definitions.is_empty() {
        return Err(AchievementValidationError::EmptyDefinitions);
    }

    let mut ids = HashSet::new();
    let mut codes = HashSet::new();
    let mut badge_keys = HashSet::new();

    for definition in definitions {
        if !is_valid_achievement_id(&definition.id) {
            return Err(AchievementValidationError::InvalidId(definition.id.clone()));
        }
        if !ids.insert(definition.id.clone()) {
            return Err(AchievementValidationError::DuplicateId(
                definition.id.clone(),
            ));
        }
        if !codes.insert(definition.code.clone()) {
            return Err(AchievementValidationError::DuplicateCode(
                definition.code.clone(),
            ));
        }
        if !badge_keys.insert(definition.badge_key.clone()) {
            return Err(AchievementValidationError::DuplicateBadgeKey(
                definition.badge_key.clone(),
            ));
        }
        if !is_valid_badge_key(&definition.badge_key) {
            return Err(AchievementValidationError::InvalidBadgeKey(
                definition.badge_key.clone(),
            ));
        }
        let expected_points = definition.difficulty.points();
        if definition.points != expected_points {
            return Err(AchievementValidationError::InvalidPoints {
                id: definition.id.clone(),
                difficulty: definition.difficulty,
                expected: expected_points,
                actual: definition.points,
            });
        }
        if definition.repeat_policy != AchievementRepeatPolicy::Once {
            return Err(AchievementValidationError::InvalidRepeatPolicy(
                definition.id.clone(),
            ));
        }
        if definition.title.trim().is_empty() {
            return Err(AchievementValidationError::EmptyTitle(
                definition.id.clone(),
            ));
        }
        if definition.condition_summary.trim().is_empty() {
            return Err(AchievementValidationError::EmptyConditionSummary(
                definition.id.clone(),
            ));
        }
    }

    Ok(())
}

fn is_valid_achievement_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 4
        && bytes[0] == b'A'
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
}

fn is_valid_badge_key(badge_key: &str) -> bool {
    badge_key.len() <= 80
        && badge_key.starts_with("cwp_badge_")
        && badge_key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::achievements::{
        load_seed_definitions, AchievementCategory, AchievementCondition, AchievementOperator,
    };

    #[test]
    fn seed_contains_132_unique_achievements() {
        let definitions = load_seed_definitions().unwrap();

        assert_eq!(definitions.len(), 132);
        validate_definitions(&definitions).unwrap();
    }

    #[test]
    fn seed_has_at_least_twenty_definitions_per_difficulty() {
        let definitions = load_seed_definitions().unwrap();
        let mut counts = BTreeMap::new();

        for definition in definitions {
            *counts.entry(definition.difficulty.key()).or_insert(0) += 1;
        }

        assert!(counts.get("entry").copied().unwrap_or_default() >= 20);
        assert!(counts.get("normal").copied().unwrap_or_default() >= 20);
        assert!(counts.get("skilled").copied().unwrap_or_default() >= 20);
        assert!(counts.get("elite").copied().unwrap_or_default() >= 20);
        assert!(counts.get("epic").copied().unwrap_or_default() >= 20);
        assert!(counts.get("legendary").copied().unwrap_or_default() >= 20);
    }

    #[test]
    fn seed_covers_all_categories_and_hidden_flags() {
        let definitions = load_seed_definitions().unwrap();
        let mut categories = HashSet::new();
        let hidden_count = definitions
            .iter()
            .filter(|definition| definition.is_hidden)
            .count();

        for definition in definitions {
            categories.insert(definition.category.key());
        }

        assert_eq!(categories.len(), 9);
        assert_eq!(hidden_count, 9);
        assert!(categories.contains(AchievementCategory::HiddenEaster.key()));
    }

    #[test]
    fn validation_rejects_duplicate_badge_key() {
        let mut definitions = load_seed_definitions().unwrap();
        definitions[1].badge_key = definitions[0].badge_key.clone();

        let error = validate_definitions(&definitions).unwrap_err();

        assert!(matches!(
            error,
            AchievementValidationError::DuplicateBadgeKey(_)
        ));
    }

    #[test]
    fn condition_dsl_serializes_counter_node() {
        let condition = AchievementCondition::Counter {
            counter: "lifetime.total_online_seconds".to_string(),
            op: AchievementOperator::GreaterThanOrEqual,
            value: 1800.0,
        };

        let json = serde_json::to_value(condition).unwrap();

        assert_eq!(json["type"], "counter");
        assert_eq!(json["counter"], "lifetime.total_online_seconds");
        assert_eq!(json["value"], 1800.0);
    }
}
