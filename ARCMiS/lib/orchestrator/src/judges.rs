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
/// Models often render the format's `|` separator as an em-dash in prose
/// (`CRITIQUE: PASS — the code matches`); accept any dash run after the
/// verdict word as the separator, so a correct verdict is not scored as a
/// missing line.
#[must_use]
pub fn parse_verdict(text: &str, keyword: &str) -> Option<(bool, String)> {
    let prefix = format!("{keyword}:");
    let line = text
        .lines()
        .rev()
        .find(|line| line.trim_start().to_ascii_uppercase().starts_with(&prefix.to_ascii_uppercase()))?;
    let body = line.trim_start()[prefix.len()..].trim();
    // The verdict word ends at the format's `|`, a dash run (em-dash, en-dash,
    // or `--`), or the end of the line. `char_indices` keeps the split on a
    // char boundary (em-dash is 3 bytes).
    let split = body
        .char_indices()
        .find(|(_, character)| matches!(character, '|' | '—' | '–'))
        .map(|(index, character)| (index, character.len_utf8()));
    let (verdict, reason) = match split {
        Some((index, dash_len)) => {
            (&body[..index], body[index + dash_len..].trim_start_matches(['-', '—', '–', ' ']).to_owned())
        },
        None => (body, String::new()),
    };
    match verdict.trim().to_ascii_lowercase().as_str() {
        "pass" => Some((true, reason)),
        "fail" => Some((false, reason)),
        _ => None,
    }
}

/// Parse the repairer's line: `REPAIR: applied|conflict|failed | <text>`.
/// The verdict word may be followed by a dash run instead of `|` (models
/// render the format's pipe as an em-dash in prose).
#[must_use]
pub fn parse_repair(text: &str) -> Option<(&'static str, String)> {
    let line = text.lines().rev().find(|line| line.trim_start().to_ascii_uppercase().starts_with("REPAIR:"))?;
    let body = line.trim_start()["REPAIR:".len()..].trim();
    let split = body
        .char_indices()
        .find(|(_, character)| matches!(character, '|' | '—' | '–'))
        .map(|(index, character)| (index, character.len_utf8()));
    let (verdict, detail) = match split {
        Some((index, dash_len)) => {
            (&body[..index], body[index + dash_len..].trim_start_matches(['-', '—', '–', ' ']).to_owned())
        },
        None => (body, String::new()),
    };
    let word = match verdict.trim().to_ascii_lowercase().as_str() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipe_verdict() {
        let (passed, reason) =
            parse_verdict("analysis\nCRITIQUE: pass | all good", "CRITIQUE").expect("pipe verdict parses");
        assert!(passed);
        assert_eq!(reason, "all good");
    }

    #[test]
    fn parses_em_dash_verdict() {
        // Observed on the GildedRose e2e: the model writes the format's pipe
        // as an em-dash, and the run scored a correct PASS as "no verdict
        // line" three times before the run stalled out.
        let text = "review body\nCRITIQUE: PASS — the repaired update_quality matches the C source";
        let (passed, reason) = parse_verdict(text, "CRITIQUE").expect("em-dash verdict parses");
        assert!(passed);
        assert!(reason.contains("matches the C source"));
    }

    #[test]
    fn parses_em_dash_fail_verdict() {
        let (passed, _) = parse_verdict("CRITIQUE: FAIL — quality drifts after expiry", "CRITIQUE").expect("parses");
        assert!(!passed);
    }

    #[test]
    fn still_rejects_garbage_verdict_word() {
        assert!(parse_verdict("CRITIQUE: maybe — dunno", "CRITIQUE").is_none());
    }

    #[test]
    fn repair_line_with_em_dash_parses() {
        let (word, _) = parse_repair("REPAIR: applied — rewrote update_quality").expect("parses");
        assert_eq!(word, "applied");
    }
}
