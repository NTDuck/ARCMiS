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

/// RFC 3339 UTC timestamp with second precision.
fn timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    let seconds = now.unix_timestamp();
    let millis = now.millisecond();
    format!("{}.{millis:03}Z", rfc3339_seconds(seconds))
}

/// Render Unix seconds as an RFC 3339 UTC timestamp (no offset math needed:
/// pure seconds-to-Y-M-D conversion).
fn rfc3339_seconds(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let time = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}", time / 3600, (time % 3600) / 60, time % 60)
}

/// Days-since-epoch to civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 {
        mp + 3
    } else {
        mp - 9
    } as u32;
    (
        if m <= 2 {
            y + 1
        } else {
            y
        },
        m,
        d,
    )
}
