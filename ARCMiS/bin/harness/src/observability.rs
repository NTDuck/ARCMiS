//! Observability: the run's JSONL event log at `{output_dir}/events.jsonl`.
//! One line per harness event: phase transitions, delegations, verdicts,
//! breaker states, and the final summary. Meta-harness tooling reads this
//! file after the run.

use anyhow::Context as _;
use serde_json::Value;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

/// Append-only event log.
pub struct EventLog {
    path: PathBuf,
}

impl EventLog {
    /// Bind the log at `{dir}/events.jsonl`, creating the directory.
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            path: dir.join("events.jsonl"),
        })
    }

    /// Append one event. `event` is the kind; `fields` carries the payload.
    pub fn record(&self, event: &str, fields: Value) -> anyhow::Result<()> {
        let record = serde_json::json!({
            "at": timestamp(),
            "event": event,
            "fields": fields,
        });
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open {}", self.path.display()))?;
        handle.write_all(serde_json::to_string(&record)?.as_bytes())?;
        handle.write_all(b"\n")?;
        handle.flush()?;
        Ok(())
    }
}

/// RFC 3339 UTC timestamp with millisecond precision.
fn timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    let millis = now.millisecond();
    format!("{}.{millis:03}Z", now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default())
}
