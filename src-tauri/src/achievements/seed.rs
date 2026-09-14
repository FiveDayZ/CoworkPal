use std::{fmt, sync::OnceLock};

use super::definitions::{
    AchievementCategory, AchievementCondition, AchievementDailyPredicate, AchievementDefinition,
    AchievementDifficulty, AchievementDistinctFilter, AchievementOperator,
};

const ACHIEVEMENT_SPEC_MD: &str = include_str!("seed.md");
static DEFINITIONS: OnceLock<Result<Vec<AchievementDefinition>, AchievementSeedError>> =
    OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementSeedError {
    DefinitionBeforeDifficulty {
        line: usize,
    },
    InvalidRow {
        line: usize,
        content: String,
    },
    UnknownCategory {
        line: usize,
        label: String,
    },
    InvalidPoints {
        line: usize,
        value: String,
    },
    InvalidHiddenFlag {
        line: usize,
        value: String,
    },
    MissingBadgeKey {
        line: usize,
    },
    InvalidCondition {
        line: usize,
        value: String,
        reason: String,
    },
    NoDefinitions,
}

impl fmt::Display for AchievementSeedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefinitionBeforeDifficulty { line } => {
                write!(
                    formatter,
                    "achievement definition before difficulty at line {line}"
                )
            }
            Self::InvalidRow { line, content } => {
                write!(
                    formatter,
                    "invalid achievement row at line {line}: {content}"
                )
            }
            Self::UnknownCategory { line, label } => {
                write!(
                    formatter,
                    "unknown achievement category '{label}' at line {line}"
                )
            }
            Self::InvalidPoints { line, value } => {
                write!(
                    formatter,
                    "invalid achievement points '{value}' at line {line}"
                )
            }
            Self::InvalidHiddenFlag { line, value } => {
                write!(
                    formatter,
                    "invalid achievement hidden flag '{value}' at line {line}"
                )
            }
            Self::MissingBadgeKey { line } => {
                write!(formatter, "missing achievement badge key at line {line}")
            }
            Self::InvalidCondition {
                line,
                value,
                reason,
            } => write!(
                formatter,
                "invalid achievement condition at line {line}: {value} ({reason})"
            ),
            Self::NoDefinitions => write!(formatter, "no achievement definitions found"),
        }
    }
}

impl std::error::Error for AchievementSeedError {}

pub fn load_seed_definitions() -> Result<Vec<AchievementDefinition>, AchievementSeedError> {
    seed_definitions().map(<[AchievementDefinition]>::to_vec)
}

pub fn seed_definitions() -> Result<&'static [AchievementDefinition], AchievementSeedError> {
    match DEFINITIONS.get_or_init(|| parse_seed_definitions(ACHIEVEMENT_SPEC_MD)) {
        Ok(definitions) => Ok(definitions),
        Err(error) => Err(error.clone()),
    }
}

fn parse_seed_definitions(
    markdown: &str,
) -> Result<Vec<AchievementDefinition>, AchievementSeedError> {
    let mut definitions = Vec::new();
    let mut current_difficulty = None;

    for (index, line) in markdown.lines().enumerate() {
        let line_number = index + 1;
        if line.starts_with("### 4.") {
            current_difficulty = AchievementDifficulty::from_section_title(line);
            continue;
        }

        if !line.starts_with("| A") {
            continue;
        }

        let Some(difficulty) = current_difficulty else {
            return Err(AchievementSeedError::DefinitionBeforeDifficulty { line: line_number });
        };

        let columns = parse_markdown_table_row(line);
        if columns.len() != 7 {
            return Err(AchievementSeedError::InvalidRow {
                line: line_number,
                content: line.to_string(),
            });
        }

        let id = columns[0].to_string();
        let category = AchievementCategory::from_label(columns[1]).ok_or_else(|| {
            AchievementSeedError::UnknownCategory {
                line: line_number,
                label: columns[1].to_string(),
            }
        })?;
        let title = columns[2].to_string();
        let points =
            columns[3]
                .parse::<u32>()
                .map_err(|_| AchievementSeedError::InvalidPoints {
                    line: line_number,
                    value: columns[3].to_string(),
                })?;
        let is_hidden = match columns[4] {
            "是" => true,
            "否" => false,
            value => {
                return Err(AchievementSeedError::InvalidHiddenFlag {
                    line: line_number,
                    value: value.to_string(),
                })
            }
        };
        let badge_key = strip_inline_code(columns[5])
            .ok_or(AchievementSeedError::MissingBadgeKey { line: line_number })?
            .to_string();
        let condition_summary = normalize_condition_summary(columns[6]);
        let condition = parse_condition(&condition_summary).map_err(|reason| {
            AchievementSeedError::InvalidCondition {
                line: line_number,
                value: condition_summary.clone(),
                reason,
            }
        })?;

        definitions.push(AchievementDefinition::new(
            id,
            title,
            category,
            difficulty,
            points,
            badge_key,
            is_hidden,
            condition_summary,
            condition,
            definitions.len() as u32 + 1,
        ));
    }

    if definitions.is_empty() {
        return Err(AchievementSeedError::NoDefinitions);
    }

    Ok(definitions)
}

fn parse_markdown_table_row(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

fn strip_inline_code(value: &str) -> Option<&str> {
    value.strip_prefix('`')?.strip_suffix('`')
}

fn trim_sentence_end(value: &str) -> &str {
    value.trim().trim_end_matches('。').trim()
}

fn normalize_condition_summary(value: &str) -> String {
    trim_sentence_end(value).replace('`', "")
}

fn parse_condition(value: &str) -> Result<AchievementCondition, String> {
    let parts = value
        .split('且')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() > 1 {
        return Ok(AchievementCondition::All {
            conditions: parts
                .into_iter()
                .map(parse_condition_part)
                .collect::<Result<Vec<_>, _>>()?,
        });
    }

    parse_condition_part(value)
}

fn parse_condition_part(value: &str) -> Result<AchievementCondition, String> {
    let (left, op, target) = split_numeric_comparison(value)
        .ok_or_else(|| "missing supported numeric comparison".to_string())?;

    if value.contains("12 条轨道") {
        return per_bucket("workshop.all_tracks", 12, op, target);
    }
    if value.contains("6 个模块") && value.contains("parts") {
        return per_bucket("workshop.parts", 6, op, target);
    }
    if value.contains("6 个模块") && value.contains("process") {
        return per_bucket("workshop.process", 6, op, target);
    }
    if value.contains("任意模块任一升级轨等级") || value.contains("第一条满级轨道")
    {
        return Ok(counter_condition("workshop.module.max_level", op, target));
    }
    if value.contains("9 个分类") {
        return per_bucket("achievement.categories", 9, op, target);
    }
    if value.contains("6 个难度") {
        return per_bucket("achievement.difficulties", 6, op, target);
    }
    if value.contains("7 个 report_day_type") {
        return per_bucket("worklog.report_day_types", 7, op, target);
    }
    if value.contains("7 个 title_family") {
        return per_bucket("worklog.title_families", 7, op, target);
    }
    if value.contains("18 个 CoCat 动画状态") {
        return per_bucket("cocat.animation_states", 18, op, target);
    }

    if left == "当前可见指标数" {
        return Ok(counter_condition(
            "settings.visible_monitor_metrics.count",
            op,
            target,
        ));
    }

    if let Some(predicate) = function_argument(&left, "calendar.days") {
        return Ok(AchievementCondition::CalendarDays {
            predicate: parse_daily_predicate(&predicate)?,
            op,
            value: integer_target(target)?,
        });
    }
    if let Some(predicate) = function_argument(&left, "calendar.consecutive_days") {
        require_greater_or_equal(op, "calendar.consecutive_days")?;
        return Ok(AchievementCondition::ConsecutiveDays {
            predicate: parse_daily_predicate(&predicate)?,
            days: integer_target(target)?,
        });
    }
    if let Some(predicate) = function_argument(&left, "calendar.months") {
        require_greater_or_equal(op, "calendar.months")?;
        let (metric, predicate_op, minimum) = split_numeric_comparison(&predicate)
            .ok_or_else(|| "invalid calendar.months predicate".to_string())?;
        if metric != "report_generated_days"
            || predicate_op != AchievementOperator::GreaterThanOrEqual
        {
            return Err("calendar.months only supports report_generated_days >= N".to_string());
        }
        return Ok(AchievementCondition::CalendarMonths {
            min_report_generated_days: integer_target(minimum)?,
            months: integer_target(target)?,
        });
    }
    if let Some(expression) = function_argument(&left, "distinct_count") {
        let (key, filter) = parse_distinct_expression(&expression)?;
        return Ok(AchievementCondition::DistinctCount {
            key,
            filter,
            op,
            value: integer_target(target)?,
        });
    }
    if let Some(expression) = function_argument(&left, "max") {
        let counters = split_counter_list(&expression, ',')?;
        return Ok(AchievementCondition::Max {
            counters,
            op,
            value: target,
        });
    }
    if left.contains(" + ") {
        return Ok(AchievementCondition::Sum {
            counters: split_counter_list(&left, '+')?,
            op,
            value: target,
        });
    }

    validate_counter_expression(&left)?;
    Ok(counter_condition(&left, op, target))
}

fn counter_condition(counter: &str, op: AchievementOperator, value: f64) -> AchievementCondition {
    AchievementCondition::Counter {
        counter: counter.trim().to_string(),
        op,
        value,
    }
}

fn validate_counter_expression(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || !value.is_ascii() {
        return Err("counter expressions must use the structured ASCII DSL".to_string());
    }
    if !value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '.' | '_' | '(' | ')' | ',' | '=' | '<' | '>' | '\'' | ' ' | '-'
            )
    }) {
        return Err("counter expression contains unsupported characters".to_string());
    }
    Ok(())
}

fn per_bucket(
    key: &str,
    bucket_count: u32,
    op: AchievementOperator,
    target: f64,
) -> Result<AchievementCondition, String> {
    require_greater_or_equal(op, "per-bucket minimum")?;
    Ok(AchievementCondition::PerBucketMin {
        key: key.to_string(),
        bucket_count,
        min_value: target,
    })
}

fn require_greater_or_equal(op: AchievementOperator, context: &str) -> Result<(), String> {
    if op == AchievementOperator::GreaterThanOrEqual {
        Ok(())
    } else {
        Err(format!("{context} only supports >= comparisons"))
    }
}

fn split_counter_list(value: &str, separator: char) -> Result<Vec<String>, String> {
    let counters = value
        .split(separator)
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if counters.len() < 2 {
        return Err("counter expression needs at least two operands".to_string());
    }
    Ok(counters)
}

fn parse_distinct_expression(
    expression: &str,
) -> Result<(String, AchievementDistinctFilter), String> {
    if let Some((key, allowed_values)) = expression.split_once(" in ") {
        let values = allowed_values
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .ok_or_else(|| "invalid distinct in-list".to_string())?
            .split(',')
            .map(|value| value.trim().trim_matches('\'').to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if values.is_empty() {
            return Err("distinct in-list is empty".to_string());
        }
        return Ok((
            key.trim().to_string(),
            AchievementDistinctFilter::In { values },
        ));
    }

    if let Some((key, filter)) = expression.split_once(" where ") {
        let (_, blocked) = filter
            .split_once("!=")
            .ok_or_else(|| "unsupported distinct filter".to_string())?;
        return Ok((
            key.trim().to_string(),
            AchievementDistinctFilter::NotEqual {
                value: blocked.trim().trim_matches('\'').to_string(),
            },
        ));
    }

    Ok((
        expression.trim().to_string(),
        AchievementDistinctFilter::Any,
    ))
}

fn parse_daily_predicate(value: &str) -> Result<AchievementDailyPredicate, String> {
    let parts = value
        .split(" AND ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() > 1 {
        return Ok(AchievementDailyPredicate::All {
            predicates: parts
                .into_iter()
                .map(parse_daily_predicate)
                .collect::<Result<Vec<_>, _>>()?,
        });
    }

    if let Some((key, op, target)) = split_numeric_comparison(value) {
        return Ok(AchievementDailyPredicate::Numeric {
            key,
            op,
            value: target,
        });
    }

    let (key, expected) = value
        .split_once('=')
        .ok_or_else(|| "unsupported daily predicate".to_string())?;
    Ok(AchievementDailyPredicate::TextEqual {
        key: key.trim().to_string(),
        value: expected.trim().trim_matches('\'').to_string(),
    })
}

fn split_numeric_comparison(value: &str) -> Option<(String, AchievementOperator, f64)> {
    let trimmed = value.trim().trim_end_matches('。').trim();
    for (operator_text, operator) in [
        (">=", AchievementOperator::GreaterThanOrEqual),
        ("<=", AchievementOperator::LessThanOrEqual),
        (">", AchievementOperator::GreaterThan),
        ("<", AchievementOperator::LessThan),
        ("=", AchievementOperator::Equal),
    ] {
        if let Some(index) = find_top_level_operator(trimmed, operator_text) {
            let left = trimmed[..index].trim().to_string();
            let right = trimmed[index + operator_text.len()..].trim();
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
        let ch = input[index..].chars().next()?;
        if ch == '\'' {
            in_quote = !in_quote;
        } else if !in_quote {
            if ch == '(' || ch == '[' {
                depth += 1;
            } else if ch == ')' || ch == ']' {
                depth -= 1;
            } else if depth == 0 && input[index..].starts_with(operator) {
                return Some(index);
            }
        }
        index += ch.len_utf8();
    }
    None
}

fn parse_leading_number(value: &str) -> Option<f64> {
    let number = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect::<String>();
    number.parse().ok()
}

fn integer_target(value: f64) -> Result<u32, String> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u32::MAX as f64 {
        Ok(value as u32)
    } else {
        Err(format!(
            "expected a non-negative integer target, got {value}"
        ))
    }
}

fn function_argument(expression: &str, function_name: &str) -> Option<String> {
    let prefix = format!("{function_name}(");
    expression
        .trim()
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(|value| value.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_reads_rows_from_current_difficulty_section() {
        let markdown = concat!(
            "### 4.1 入门（简单）\n",
            "| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |\n",
            "| A001 | 日常使用类 | 第一次唤醒 CoCat | 5 | 否 | ",
            "`cwp_badge_daily_first_launch_entry` | `app.launch.count >= 1`。 |\n",
        );

        let definitions = parse_seed_definitions(markdown).unwrap();

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].id, "A001");
        assert_eq!(definitions[0].difficulty, AchievementDifficulty::Entry);
        assert_eq!(definitions[0].category, AchievementCategory::DailyUse);
        assert_eq!(definitions[0].condition_summary, "app.launch.count >= 1");
    }

    #[test]
    fn seed_definitions_reuse_the_cached_allocation() {
        let first = seed_definitions().unwrap();
        let second = seed_definitions().unwrap();

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn unsupported_natural_language_condition_is_rejected() {
        let error = parse_condition("十二条轨道全部 >= 100").unwrap_err();

        assert_eq!(
            error,
            "counter expressions must use the structured ASCII DSL"
        );

        assert!(parse_condition("calendar.consecutive_days(active_seconds >= 1) <= 3").is_err());
    }
}
