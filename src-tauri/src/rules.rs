use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MatchLogic {
    All,
    Any,
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StringOp {
    Is,
    IsNot,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NumberOp {
    GreaterThan,
    LessThan,
    Equals,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Condition {
    Extension { operator: StringOp, text: String },
    Name { operator: StringOp, text: String },
    Size { operator: NumberOp, bytes: u64 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum Action {
    Move { target_folder: String },
    Rename { pattern: String },
    Delete { delay_days: u32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub match_type: MatchLogic,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

pub fn load_rules<P: AsRef<Path>>(path: P) -> Result<Vec<Rule>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read rules file: {}", e))?;
    let rules: Vec<Rule> =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
    Ok(rules)
}

pub fn save_rules(path: &PathBuf, rules: &[Rule]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(rules).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn get_matching_rule(rules: &[Rule], file_path: &Path) -> Option<Rule> {
    for rule in rules {
        if !rule.active {
            continue;
        }

        let mut pass_count = 0;
        let total = rule.conditions.len();

        for condition in &rule.conditions {
            let passed = match condition {
                Condition::Extension { operator, text } => {
                    let ext = file_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    eval_string(&ext, operator, &text.to_lowercase())
                }
                Condition::Name { operator, text } => {
                    let name = file_path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    eval_string(&name, operator, &text.to_lowercase())
                }
                Condition::Size { operator, bytes } => {
                    let file_size = file_path.metadata().map(|m| m.len()).unwrap_or(0);
                    eval_number(file_size, operator, *bytes)
                }
            };
            if passed {
                pass_count += 1;
            }
        }

        let rule_matches = match rule.match_type {
            MatchLogic::All => pass_count == total && total > 0,
            MatchLogic::Any => pass_count > 0,
            MatchLogic::None => pass_count == 0 && total > 0,
        };

        if rule_matches {
            return Some(rule.clone());
        }
    }
    None
}

fn eval_string(actual: &str, op: &StringOp, expected: &str) -> bool {
    let tokens: Vec<&str> = expected
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }

    match op {
        StringOp::Is => tokens.iter().any(|&t| actual == t),
        StringOp::IsNot => tokens.iter().all(|&t| actual != t),
        StringOp::Contains => tokens.iter().any(|&t| actual.contains(t)),
        StringOp::StartsWith => tokens.iter().any(|&t| actual.starts_with(t)),
        StringOp::EndsWith => tokens.iter().any(|&t| actual.ends_with(t)),
    }
}

fn eval_number(actual: u64, op: &NumberOp, expected: u64) -> bool {
    match op {
        NumberOp::GreaterThan => actual > expected,
        NumberOp::LessThan => actual < expected,
        NumberOp::Equals => actual == expected,
    }
}
