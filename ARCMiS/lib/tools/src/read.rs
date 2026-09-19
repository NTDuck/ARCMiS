//! `read` reads files, directories, SQLite, and URLs for the agent.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use std::fs::{metadata, read, read_dir};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// `read` returns file content, directory listings, and structured data with
/// hashline tags for later edits.
pub struct Read {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
    /// Shared snapshot cache. Minted tags back the `¶PATH#TAG` headers.
    pub snapshots: Arc<crate::util::snapshots::SnapshotStore>,
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
        let absolute = path.clone();
        let output = tokio::task::spawn_blocking(move || -> Result<String, String> {
            read_local(&absolute, &relative, &selector, &target)
        })
        .await
        .map_err(|error| ToolExecutionError::other(format!("read join failed: {error}")))?
        .map_err(ToolExecutionError::other)?;
        Ok(ToolOutput::text(output))
    }
}

/// Arguments for `read`.
#[derive(Debug, serde::Deserialize)]
pub struct ReadArgs {
    pub path: String,
}

/// Maximum number of lines emitted per file read.
const MAX_LINES: usize = 3000;
/// Maximum byte size emitted per read.
const MAX_BYTES: usize = 50 * 1024;
/// Cap on lines shown per directory listing.
const MAX_DIR_ENTRIES: usize = 3000;
/// Cap on table rows emitted for one SQLite table.
const MAX_TABLE_ROWS: usize = 100;

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

/// Render the relative path shown in the `¶` header.
fn relative_display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Dispatch a local read by kind: directory, SQLite, archive, or text.
fn read_local(path: &Path, relative: &str, selector: &str, _target: &str) -> Result<String, String> {
    let metadata = metadata(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    if metadata.is_dir() {
        return read_directory(path, relative);
    }
    if metadata.is_file() {
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
            return Ok(format!("¶{relative}#0000\narchive container with no inline member support\n"));
        }
        return read_text(path, relative, selector);
    }
    Err(format!("read failed for '{relative}': not a regular file or directory"))
}

/// List one directory newest-first, grouped by directory entry.
fn read_directory(path: &Path, relative: &str) -> Result<String, String> {
    let entries = read_dir(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    let mut rows: Vec<(String, bool, u64)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read failed for '{relative}': {error}"))?;
        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|meta| meta.is_dir()).unwrap_or(false);
        let size = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
        rows.push((entry.file_name().to_string_lossy().into_owned(), is_dir, size));
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let mut output = format!("¶{relative}#0000\n");
    let mut shown = 0;
    for (name, is_dir, size) in rows {
        if shown >= MAX_DIR_ENTRIES {
            output.push_str(&format!("... {} more entries elided\n", shown));
            break;
        }
        let suffix = if is_dir {
            "/"
        } else {
            ""
        };
        output.push_str(&format!("{}{} ({})\n", name, suffix, format_size(size)));
        shown += 1;
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

/// Read a text file and emit the hashline format with elided ranges.
fn read_text(path: &Path, relative: &str, selector: &str) -> Result<String, String> {
    let bytes = read(path).map_err(|error| format!("read failed for '{relative}': {error}"))?;
    if bytes.len() > MAX_BYTES * 4 {
        return Ok(format!("¶{relative}#0000\nfile exceeds the 200KiB hard cap and is not read\n"));
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let all = text.lines().collect::<Vec<_>>();
    if selector == "raw" {
        return Ok(format!("¶{relative}#0000\n{text}"));
    }
    let windows = parse_ranges(selector)?;
    let mut output = format!("¶{relative}#0000\n");
    let mut emitted = 0usize;
    if windows.is_empty() {
        let end = all.len().max(1);
        let mut one_based = 1usize;
        while one_based <= end && emitted < MAX_LINES {
            if let Some(line) = all.get(one_based - 1) {
                output.push_str(&format!("{one_based}:{line}\n"));
                emitted += 1;
            }
            one_based += 1;
        }
        if end > MAX_LINES {
            output.push_str(&format!("... lines {} to {} elided ({} lines)\n", MAX_LINES + 1, end, end - MAX_LINES));
        }
        let total_bytes = text.len();
        if total_bytes > MAX_BYTES {
            output.push_str(&format!("... output truncated at {MAX_BYTES} bytes\n"));
        }
        return Ok(output);
    }
    let mut last_emitted = 0usize;
    for (start, end) in windows {
        let lo = start.max(1);
        let hi = end.min(all.len().max(1)).max(lo);
        if last_emitted > 0 && lo > last_emitted + 1 {
            output.push_str(&format!("... lines {} to {} elided\n", last_emitted + 1, lo - 1));
        }
        let mut one_based = lo;
        while one_based <= hi && emitted < MAX_LINES {
            if let Some(line) = all.get(one_based - 1) {
                output.push_str(&format!("{one_based}:{line}\n"));
                emitted += 1;
            }
            one_based += 1;
        }
        last_emitted = hi;
        if emitted >= MAX_LINES {
            break;
        }
    }
    if last_emitted < all.len().max(1) {
        output.push_str(&format!(
            "... lines {} to {} elided ({} total lines)\n",
            last_emitted + 1,
            all.len(),
            all.len()
        ));
    }
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
        if part.is_empty() {
            continue;
        }
        if let Some(minus) = part.find('-') {
            let start: usize = part[..minus].trim().parse().map_err(|_| format!("bad line selector '{part}'"))?;
            let end: usize = part[minus + 1..].trim().parse().map_err(|_| format!("bad line selector '{part}'"))?;
            if end < start {
                return Err(format!("line selector '{part}' has end before start"));
            }
            windows.push((start, end));
        } else if let Some(plus) = part.find('+') {
            let start: usize = part[..plus].trim().parse().map_err(|_| format!("bad line selector '{part}'"))?;
            let count: usize = part[plus + 1..].trim().parse().map_err(|_| format!("bad line selector '{part}'"))?;
            windows.push((start, start.saturating_add(count).saturating_sub(1)));
        } else {
            let start: usize = part.parse().map_err(|_| format!("bad line selector '{part}'"))?;
            windows.push((start, start));
        }
    }
    Ok(windows)
}

/// Read one SQLite database: table list, one table, or a custom query.
fn read_sqlite(path: &Path, relative: &str, selector: &str) -> Result<String, String> {
    let connection = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("sqlite open failed for '{relative}': {error}"))?;
    if selector.is_empty() {
        return sqlite_tables(&connection, relative);
    }
    let (table, key) = match selector.split_once(':') {
        Some((table, key)) => (table, Some(key)),
        None => (selector, None),
    };
    match key {
        None => sqlite_rows(&connection, relative, table, None),
        Some(key) => sqlite_rows(&connection, relative, table, Some(key)),
    }
}

/// Emit one table list for the database.
fn sqlite_tables(connection: &rusqlite::Connection, relative: &str) -> Result<String, String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .map_err(|error| format!("sqlite query failed for '{relative}': {error}"))?;
    let names: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("sqlite query failed for '{relative}': {error}"))?
        .filter_map(Result::ok)
        .collect();
    let mut output = format!("¶{relative}#0000\n");
    for name in names {
        output.push_str(&format!("{name}\n"));
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
    let safe_table = table.replace('\'', "''");
    let sql = match key {
        Some(_) => format!("SELECT * FROM '{safe_table}' WHERE rowid = ?1 LIMIT 1"),
        None => format!("SELECT * FROM '{safe_table}' LIMIT {MAX_TABLE_ROWS}"),
    };
    let mut statement =
        connection.prepare(&sql).map_err(|error| format!("sqlite query failed for '{relative}': {error}"))?;
    let column_names: Vec<String> = statement.column_names().iter().map(|name| name.to_string()).collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mapped = statement.query_map(rusqlite_params(key), |row| {
        let mut cells: Vec<String> = Vec::with_capacity(column_names.len());
        for index in 0..column_names.len() {
            let value: rusqlite::types::Value = row.get(index).unwrap_or(rusqlite::types::Value::Null);
            cells.push(format_sql_value(&value));
        }
        Ok(cells)
    });
    let mapped = mapped.map_err(|error| format!("sqlite query failed for '{relative}': {error}"))?;
    for row in mapped {
        let row = row.map_err(|error| format!("sqlite query failed for '{relative}': {error}"))?;
        rows.push(row);
    }
    let mut output = format!("¶{relative}#0000\n");
    output.push_str(&column_names.join(" | "));
    output.push('\n');
    for row in rows {
        output.push_str(&row.join(" | "));
        output.push('\n');
    }
    Ok(output)
}

/// Map an optional primary key to the rusqlite params list.
fn rusqlite_params(key: Option<&str>) -> [rusqlite::types::Value; 1] {
    match key {
        Some(value) => [rusqlite::types::Value::Text(value.to_string())],
        None => [rusqlite::types::Value::Integer(0)],
    }
}

/// Render one SQLite cell value as short text.
fn format_sql_value(value: &rusqlite::types::Value) -> String {
    match value {
        rusqlite::types::Value::Null => "NULL".to_owned(),
        rusqlite::types::Value::Integer(number) => number.to_string(),
        rusqlite::types::Value::Real(number) => number.to_string(),
        rusqlite::types::Value::Text(text) => text.clone(),
        rusqlite::types::Value::Blob(bytes) => format!("<{} bytes>", bytes.len()),
    }
}

/// Fetch one URL and return cleaned text or raw HTML.
async fn read_url(raw: &str) -> Result<String, ToolExecutionError> {
    Err(ToolExecutionError::other(format!("read does not fetch remote URLs in this build: {raw}")))
}
