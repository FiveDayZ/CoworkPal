use std::fmt;

use super::definitions::{AchievementCategory, AchievementDefinition, AchievementDifficulty};

const ACHIEVEMENT_SPEC_MD: &str = include_str!("seed.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementSeedError {
    DefinitionBeforeDifficulty { line: usize },
    InvalidRow { line: usize, content: String },
    UnknownCategory { line: usize, label: String },
    InvalidPoints { line: usize, value: String },
    InvalidHiddenFlag { line: usize, value: String },
    MissingBadgeKey { line: usize },
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
            Self::NoDefinitions => write!(formatter, "no achievement definitions found"),
        }
    }
}

impl std::error::Error for AchievementSeedError {}

pub fn load_seed_definitions() -> Result<Vec<AchievementDefinition>, AchievementSeedError> {
    parse_seed_definitions(ACHIEVEMENT_SPEC_MD)
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

        definitions.push(AchievementDefinition::new(
            id,
            title,
            category,
            difficulty,
            points,
            badge_key,
            is_hidden,
            condition_summary,
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
}
