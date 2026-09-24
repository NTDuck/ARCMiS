//! CLI sink: one line per orchestration event to stdout. Human-facing
//! progress; the JSON observability trace is separate.

use std::io::Write as _;

/// Emit one event line. Never panics; a broken stdout pipe is logged.
pub fn emit(event: &str, detail: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "[{event}] {detail}");
    let _ = stdout.flush();
}
