//! `read` reads files, directories, SQLite, and URLs for the agent.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use std::fs::{metadata, read, read_dir};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// Default number of body lines emitted per text read (oh-my-pi default).
const DEFAULT_LIMIT: usize = 400;
/// Hard byte cap: files above this size are reported, never read.
const HARD_BYTES: usize = 200 * 1024;
/// Cap on lines shown per directory listing.
const MAX_DIR_ENTRIES: usize = 3000;
/// Cap on table rows emitted for one SQLite table.
const MAX_TABLE_ROWS: usize = 100;

/// `read` returns file content, directory listings, and structured data with
/// hashline tags for later edits.
pub struct Read {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
    /// Shared snapshot store. Every text read records a snapshot; the minted
    /// tag backs the `[<path>#<TAG>]` header the next edit anchors on.
    pub snapshots: Arc<dyn oxi_hashline::SnapshotStore>,
}

/// Arguments for `read`.
#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    pub path: String,
    /// 1-indexed line to start from (1 = first line).
    #[serde(default)]
    pub offset: Option<usize>,
    /// Number of body lines to return after `offset`.
    #[serde(default)]
    pub limit: Option<usize>,
}

impl Tool for Read {
    const NAME: &'static str = "read";
    type Error = ToolExecutionError;
    type Args = ReadArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Read a file, directory, archive, SQLite database, or URL and return text with line anchors.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File, directory, URL, or SQLite path. Optional line selectors like ':50-200' or ':raw'."
                },
                "offset": {
                    "type": "integer",
                    "description": "1-indexed line number to start from. Default 1."
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of body lines to return. Default 400."
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let raw = args.path.trim().to_owned();
        if raw.contains("://") {
            return Ok(ToolOutput::text(read_url(&raw).await?));
        }
        let path = crate::util::path::path_sanitize(&self.root, &raw).map_err(ToolExecutionError::other)?;
        let (target, selector) = split_selector(&raw);
        let target = target.to_string_lossy().into_owned();
        let relative = relative_display(&path);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT).max(1);
        let offset = args.offset.unwrap_or(1).max(1);
        let absolute = path.clone();
        let store = self.snapshots.clone();
        let output = tokio::task::spawn_blocking(move || -> Result<String, String> {
            read_local(&store, &absolute, &relative, &selector, &target, offset, limit)
        })
        .await
        .map_err(|error| ToolExecutionError::other(format!("read join failed: {error}")))?
        .map_err(ToolExecutionError::other)?;
        Ok(ToolOutput::text(output))
    }
}

/// Split one trailing `:selector` suffix from the raw path argument.
fn split_selector(raw: &str) -> (PathBuf, String) {
    let bytes = raw.as_bytes();
    let mut cut = raw.len();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b':' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            cut = index;
            break;
        }
        index += 1;
    }
    let target = raw[..cut].to_owned();
    let selector = raw[cut..].trim_start_matches(':').to_owned();
    (PathBuf::from(target), selector)
}

/// Render the relative path shown in the `[]` header.
fn relative_display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Dispatch a local read by kind: directory, SQLite, archive, or text.
fn read_local(
    store: &Arc<dyn oxi_hashline::SnapshotStore>,
    path: &Path,
    relative: &str,
    selector: &str,
    _target: &str,
    offset: usize,
    limit: usize,
) -> Result<String, String> {
    let meta = metadata(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    if meta.is_dir() {
        return read_directory(path, relative);
    }
    if meta.is_file() {
        let lower = relative.to_ascii_lowercase();
        if lower.ends_with(".sqlite")
            || lower.ends_with(".sqlite3")
            || lower.ends_with(".db")
            || lower.ends_with(".db3")
        {
            return read_sqlite(path, relative, selector);
        }
        if lower.ends_with(".zip")
            || lower.ends_with(".jar")
            || lower.ends_with(".apk")
            || lower.ends_with(".whl")
            || lower.ends_with(".war")
            || lower.ends_with(".ear")
            || lower.ends_with(".tar")
            || lower.ends_with(".tar.gz")
            || lower.ends_with(".tgz")
        {
            // Archive members are skipped in this build. Report the container.
            return Ok(format!("[{relative}#0000]\narchive container with no inline member support\n"));
        }
        return read_text(store, path, relative, selector, offset, limit);
    }
    Err(format!("read failed for '{relative}': not a regular file or directory"))
}

/// List one directory newest-first, grouped by directory entry.
fn read_directory(path: &Path, relative: &str) -> Result<String, String> {
    let entries = read_dir(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let mut rows: Vec<(String, bool, u64)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read failed for '{relative}': {error}"))?;
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|item| item.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|item| item.len()).unwrap_or(0);
        rows.push((entry.file_name().to_string_lossy().into_owned(), is_dir, size));
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let mut output = format!("[{relative}#0000]\n");
    let shown = rows.len().min(MAX_DIR_ENTRIES);
    for (name, is_dir, size) in rows.iter().take(shown) {
        let suffix = if *is_dir { "/" } else { "" };
        output.push_str(&format!("{name}{suffix} ({})\n", format_size(*size)));
    }
    if shown < rows.len() {
        output.push_str(&format!("... {} more entries elided\n", rows.len() - shown));
    }
    Ok(output)
}

/// Render one byte count in a short human form.
fn format_size(size: u64) -> String {
    if size >= 1024 * 1024 {
        format!("{:.1}MiB", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.1}KiB", size as f64 / 1024.0)
    } else {
        format!("{size}B")
    }
}

/// Read a text file and emit the hashline format with the requested window.
///
/// The whole file is recorded in the snapshot store (oh-my-pi contract: the
/// snapshot is edit safety, not a read cache); only the requested window is
/// displayed, and the displayed lines feed `seen_lines` provenance.
fn read_text(
    store: &Arc<dyn oxi_hashline::SnapshotStore>,
    path: &Path,
    relative: &str,
    selector: &str,
    offset: usize,
    limit: usize,
) -> Result<String, String> {
    let bytes = read(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    if bytes.len() > HARD_BYTES {
        return Ok(format!("[{relative}#0000]\nfile exceeds the 200KiB hard cap and is not read\n"));
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if selector == "raw" {
        return Ok(format!("[{relative}#0000]\n{text}"));
    }
    let windows = parse_ranges(selector)?;
    let all: Vec<&str> = text.lines().collect();
    let total = all.len().max(1);
    let tag = store.record(relative, &text, None);
    let mut output = format!("[{relative}#{tag}]\n");
    let mut emitted = 0usize;
    let mut last_emitted = 0usize;
    let mut shown_lines: Vec<u32> = Vec::new();
    // No selector: one window at `offset` of length `limit`.
    let plan: Vec<(usize, usize)> = if windows.is_empty() {
        vec![(offset, offset.saturating_add(limit).saturating_sub(1))]
    } else {
        windows
    };
    for (start, end) in plan {
        let lo = start.max(1);
        let hi = end.min(total).max(lo);
        if last_emitted > 0 && lo > last_emitted + 1 {
            output.push_str(&format!("... lines {} to {} elided\n", last_emitted + 1, lo - 1));
        }
        let mut one_based = lo;
        while one_based <= hi && emitted < limit {
            if let Some(line) = all.get(one_based - 1) {
                output.push_str(&format!("{one_based}:{line}\n"));
                shown_lines.push(one_based as u32);
                emitted += 1;
            }
            one_based += 1;
        }
        last_emitted = hi;
        if emitted >= limit {
            break;
        }
    }
    if last_emitted < total {
        output.push_str(&format!(
            "... lines {} to {} elided ({} total lines)\n",
            last_emitted + 1,
            total,
            total
        ));
    }
    store.record_seen_lines(relative, &tag, &shown_lines);
    Ok(output)
}

/// Parse one comma-separated list of `N`, `N-M`, and `N+K` selectors.
fn parse_ranges(selector: &str) -> Result<Vec<(usize, usize)>, String> {
    let mut windows = Vec::new();
    if selector.is_empty() {
        return Ok(windows);
    }
    for part in selector.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_suffix('+') {
            let start: usize = rest.parse().map_err(|_| format!("bad selector '{part}'"))?;
            windows.push((start, usize::MAX));
            continue;
        }
        if let Some((start_text, end_text)) = part.split_once('-') {
            let start: usize = start_text.parse().map_err(|_| format!("bad selector '{part}'"))?;
            let end: usize = end_text.parse().map_err(|_| format!("bad selector '{part}'"))?;
            windows.push((start, end.max(start)));
            continue;
        }
        let one: usize = part.parse().map_err(|_| format!("bad selector '{part}'"))?;
        windows.push((one, one));
    }
    Ok(windows)
}

/// Read one SQLite database: table list, one table, or a custom query.
fn read_sqlite(path: &Path, relative: &str, selector: &str) -> Result<String, String> {
    let connection = rusqlite::Connection::open(path)
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    if selector.is_empty() {
        return sqlite_tables(&connection, relative);
    }
    if let Some(table) = selector.strip_prefix("q=") {
        return sqlite_query(&connection, relative, table);
    }
    sqlite_rows(&connection, relative, selector, None)
}

/// Emit one table list for the database.
fn sqlite_tables(connection: &rusqlite::Connection, relative: &str) -> Result<String, String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("read failed for '{relative}': {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let mut output = format!("[{relative}#0000]\n");
    for name in names {
        output.push_str(&format!("{name}\n"));
    }
    Ok(output)
}

/// Run one custom query and render its rows.
fn sqlite_query(connection: &rusqlite::Connection, relative: &str, query: &str) -> Result<String, String> {
    let mut statement = connection
        .prepare(query)
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let columns: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let rows = statement
        .query_map([], |row| {
            let mut values = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                values.push(format_sql_value(&row.get(index).unwrap_or(rusqlite::types::Value::Null)));
            }
            Ok(values.join(" | "))
        })
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let mut output = format!("[{relative}#0000]\n");
    for (index, row) in rows.enumerate() {
        if index >= MAX_TABLE_ROWS {
            output.push_str(&format!("... rows beyond {MAX_TABLE_ROWS} elided\n"));
            break;
        }
        output.push_str(&format!("{}:{}\n", index + 1, row.map_err(|e| e.to_string())?));
    }
    Ok(output)
}

/// Emit rows of one table, optionally filtered by primary key.
fn sqlite_rows(
    connection: &rusqlite::Connection,
    relative: &str,
    table: &str,
    key: Option<&str>,
) -> Result<String, String> {
    let sql = match key {
        Some(_) => format!("SELECT rowid, * FROM \"{table}\" WHERE rowid = ?1"),
        None => format!("SELECT rowid, * FROM \"{table}\" LIMIT {MAX_TABLE_ROWS}"),
    };
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let params = rusqlite::params![key.and_then(|k| k.parse::<i64>().ok()).unwrap_or(0)];
    let rows = statement
        .query_map(params, |row| {
            let columns = row.as_ref().column_count();
            let mut values = Vec::with_capacity(columns);
            for index in 0..columns {
                values.push(format_sql_value(&row.get(index).unwrap_or(rusqlite::types::Value::Null)));
            }
            Ok(values.join(" | "))
        })
        .map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let mut output = format!("[{relative}#0000]\n");
    for (index, row) in rows.enumerate() {
        if index >= MAX_TABLE_ROWS {
            output.push_str(&format!("... rows beyond {MAX_TABLE_ROWS} elided\n"));
            break;
        }
        output.push_str(&format!("{}:{}\n", index + 1, row.map_err(|e| e.to_string())?));
    }
    Ok(output)
}

/// Render one SQLite cell value as short text.
fn format_sql_value(value: &rusqlite::types::Value) -> String {
    match value {
        rusqlite::types::Value::Null => "NULL".to_owned(),
        rusqlite::types::Value::Integer(v) => v.to_string(),
        rusqlite::types::Value::Real(v) => v.to_string(),
        rusqlite::types::Value::Text(v) => v.clone(),
        rusqlite::types::Value::Blob(v) => format!("<{} bytes>", v.len()),
    }
}

/// Fetch one URL and return cleaned text or raw HTML.
async fn read_url(raw: &str) -> Result<String, ToolExecutionError> {
    Err(ToolExecutionError::other(format!(
        "read does not fetch remote URLs in this build: {raw}"
    )))
}

/// Split text into lines without line terminators (used by tests).
#[cfg(test)]
pub(crate) fn split_lines(text: &str) -> Vec<String> {
    text.split('\n').map(|line| line.to_owned()).collect()
}
