//! On-disk evidence readers for the round triage (ADR 0028): aggregate yml
//! scalars, ledger JSONL counters, and the shared field shape. Pure file
//! reading with no laya and no policy.

use std::path::Path;

use anyhow::Context;
use serde_json::Value;

/// Read `result/aggregate.yml` scalars into the shared field shape.
pub fn read_aggregate(aggregate_path: &Path) -> anyhow::Result<AggregateFields> {
    aggregate_fields(aggregate_path)
}

/// Aggregate fields the round triage and the CLI subcommand share.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateFields {
    pub final_phase: String,
    pub completed: bool,
    pub stop_reason: String,
    pub rounds: usize,
    pub delegations: usize,
    pub tasks_done: usize,
    pub tasks_total: usize,
    pub wall_seconds: Option<u64>,
}

fn parse_bool(value: &str) -> bool {
    value == "true"
}

pub(crate) fn aggregate_fields(path: &Path) -> anyhow::Result<AggregateFields> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AggregateFields {
                final_phase: String::new(),
                completed: false,
                stop_reason: String::new(),
                rounds: 0,
                delegations: 0,
                tasks_done: 0,
                tasks_total: 0,
                wall_seconds: None,
            })
        },
        Err(error) => return Err(error.into()),
    };
    Ok(AggregateFields {
        final_phase: scalar(&text, "final_phase"),
        completed: parse_bool(&scalar(&text, "completed")),
        stop_reason: scalar(&text, "stop_reason"),
        rounds: scalar(&text, "rounds").parse().unwrap_or(0),
        delegations: scalar(&text, "delegations").parse().unwrap_or(0),
        tasks_done: scalar(&text, "tasks_done").parse().unwrap_or(0),
        tasks_total: scalar(&text, "tasks_total").parse().unwrap_or(0),
        wall_seconds: scalar(&text, "duration_seconds").parse().ok(),
    })
}

/// One scalar from the flat aggregate yml. `stop_reason` values are prose
/// and quote-free in the harness's own writer, so a plain trim is enough.
fn scalar(text: &str, key: &str) -> String {
    for line in text.lines() {
        let stripped = line.split('#').next().unwrap_or_default().trim();
        if let Some(rest) = stripped.strip_prefix(key) {
            if let Some(value) = rest.strip_prefix(':') {
                return value.trim().trim_matches('"').to_owned();
            }
        }
    }
    String::new()
}

/// Count observation rows of one kind. A missing file counts zero: an
/// uninstrumented round stays uninstrumented, never fabricated.
pub(crate) fn count_kind(path: &Path, kind: &str) -> anyhow::Result<usize> {
    let mut n = 0;
    for row in read_jsonl(path)? {
        if row.get("kind").and_then(Value::as_str) == Some(kind) {
            n += 1;
        }
    }
    Ok(n)
}

/// Count decision rows of one action.
pub(crate) fn count_action(path: &Path, action: &str) -> anyhow::Result<usize> {
    let mut n = 0;
    for row in read_jsonl(path)? {
        if row.get("action").and_then(Value::as_str) == Some(action) {
            n += 1;
        }
    }
    Ok(n)
}

/// Root-cause strings of every failure row.
pub(crate) fn read_failure_causes(path: &Path) -> anyhow::Result<Vec<String>> {
    Ok(read_jsonl(path)?
        .into_iter()
        .filter_map(|row| row.get("root_cause").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

/// Read one JSONL file into values. A missing file is an empty list.
fn read_jsonl(path: &Path) -> anyhow::Result<Vec<Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("ledger row parse"))
        .collect()
}
