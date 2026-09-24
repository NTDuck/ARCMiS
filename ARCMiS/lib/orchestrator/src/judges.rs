//! Failure classification parsed from the failure analyst's `DIAGNOSIS`
//! line, plus the repair budget the breaker enforces.

use serde::Serialize;

/// Failure category. Mirrors the failure-analyst prompt's taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    /// Build, compile, linker, or dependency failure.
    Toolchain,
    /// An assertion or behavior check failed.
    Test,
    /// Missing tool, permission, network, or disk.
    Environment,
    /// A specialist misread the contract or produced a stub.
    Model,
    /// The plan is wrong: cycle, missing module, impossible order.
    Plan,
}

/// One parsed diagnosis.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnosis {
    /// Category.
    pub category: FailureCategory,
    /// Root cause sentence.
    pub root_cause: String,
    /// Suggested action sentence.
    pub suggested_action: String,
}

/// Parse the failure analyst's final line:
/// `DIAGNOSIS: <category> | <root cause> | <action>`.
#[must_use]
pub fn parse_diagnosis(text: &str) -> Option<Diagnosis> {
    let line = text.lines().rev().find(|line| line.trim_start().to_ascii_uppercase().starts_with("DIAGNOSIS:"))?;
    let body = line.trim();
    let body = body.split_once(':').map(|(_, rest)| rest).unwrap_or(body).trim();
    let mut parts = body.split('|').map(str::trim);
    let category_raw = parts.next()?;
    let root_cause = parts.next().unwrap_or("unspecified").to_owned();
    let suggested_action = parts.next().unwrap_or("retry").to_owned();
    let category = match category_raw.to_ascii_lowercase().as_str() {
        "toolchain" => FailureCategory::Toolchain,
        "test" => FailureCategory::Test,
        "environment" => FailureCategory::Environment,
        "model" => FailureCategory::Model,
        "plan" => FailureCategory::Plan,
        _ => return None,
    };
    Some(Diagnosis {
        category,
        root_cause,
        suggested_action,
    })
}

/// Parse the validator's or critic's verdict line:
/// `VALIDATION: pass|fail | <reason>` / `CRITIQUE: pass|fail | <reason>`.
#[must_use]
pub fn parse_verdict(text: &str, keyword: &str) -> Option<(bool, String)> {
    let prefix = format!("{keyword}:");
    let line = text
        .lines()
        .rev()
        .find(|line| line.trim_start().to_ascii_uppercase().starts_with(&prefix.to_ascii_uppercase()))?;
    let body = line.trim_start()[prefix.len()..].trim();
    let mut parts = body.splitn(2, '|');
    let verdict = parts.next()?.trim().to_ascii_lowercase();
    let reason = parts.next().unwrap_or("").trim().to_owned();
    match verdict.as_str() {
        "pass" => Some((true, reason)),
        "fail" => Some((false, reason)),
        _ => None,
    }
}

/// Parse the repairer's line: `REPAIR: applied|conflict|failed | <text>`.
#[must_use]
pub fn parse_repair(text: &str) -> Option<(&'static str, String)> {
    let line = text.lines().rev().find(|line| line.trim_start().to_ascii_uppercase().starts_with("REPAIR:"))?;
    let body = line.trim_start()["REPAIR:".len()..].trim();
    let mut parts = body.splitn(2, '|');
    let verdict = parts.next()?.trim().to_ascii_lowercase();
    let detail = parts.next().unwrap_or("").trim().to_owned();
    let word = match verdict.as_str() {
        "applied" => "applied",
        "conflict" => "conflict",
        "failed" => "failed",
        _ => return None,
    };
    Some((word, reason_of(word, detail)))
}

fn reason_of(word: &str, detail: String) -> String {
    if detail.is_empty() {
        word.to_owned()
    } else {
        detail
    }
}
