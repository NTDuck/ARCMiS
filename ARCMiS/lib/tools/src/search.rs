//! `search` finds regex matches across files and directories.

use grep_matcher::Matcher as _;
use std::fs::read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// `search` finds regex matches and returns hashline-tagged context.
pub struct Search {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
    /// Shared snapshot cache. Search records snapshots for edited files.
    pub snapshots: Arc<crate::util::snapshots::SnapshotStore>,
}

impl rig::tool::Tool for Search {
    const NAME: &'static str = "search";
    type Error = rig::tool::ToolExecutionError;
    type Args = SearchArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> String {
        "Search files under the sandbox root for one regex and return matches with context.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Rust regex pattern to search for."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files, directories, or globs relative to the sandbox root. Defaults to the root."
                },
                "i": {
                    "type": "boolean",
                    "description": "Case-insensitive match."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn call(&self, _context: &mut rig::tool::ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let pattern = grep_regex::RegexMatcherBuilder::new()
            .case_insensitive(args.i.unwrap_or(false))
            .build(&args.pattern)
            .map_err(|error| rig::tool::ToolExecutionError::other(format!("bad regex '{}': {error}", args.pattern)))?;
        let roots = resolve_roots(&self.root, &args.paths).map_err(rig::tool::ToolExecutionError::other)?;
        let mut output = String::new();
        for root in roots {
            let report = search_root(&root, &self.root, &pattern)
                .map_err(|error| rig::tool::ToolExecutionError::other(format_root_error(&root, &error)))?;
            output.push_str(&report);
        }
        if output.is_empty() {
            output.push_str(&format!("no matches for '{}'\n", args.pattern));
        }
        Ok(rig::tool::ToolOutput::text(output))
    }
}

/// Arguments for `search`.
#[derive(Debug, serde::Deserialize)]
pub struct SearchArgs {
    pub pattern: String,
    pub paths: Option<Vec<String>>,
    pub i: Option<bool>,
}

/// Maximum bytes emitted per search.
const MAX_SEARCH_BYTES: usize = 50 * 1024;
/// Context lines shown before each match.
const BEFORE_CONTEXT: usize = 1;
/// Context lines shown after each match.
const AFTER_CONTEXT: usize = 3;

/// Resolve the requested roots under the sandbox root.
fn resolve_roots(root: &Path, paths: &Option<Vec<String>>) -> Result<Vec<PathBuf>, String> {
    let requested = match paths {
        Some(paths) if !paths.is_empty() => paths.clone(),
        _ => vec![".".to_owned()],
    };
    let mut resolved = Vec::with_capacity(requested.len());
    for path in requested {
        resolved.push(crate::util::path::path_sanitize(root, &path)?);
    }
    Ok(resolved)
}

/// Search one root path and return the tagged report text.
fn search_root(root: &Path, sandbox: &Path, matcher: &grep_regex::RegexMatcher) -> Result<String, String> {
    let mut output = String::new();
    if root.is_file() {
        let relative = relative_path(sandbox, root);
        let report = search_file(root, &relative, matcher)?;
        output.push_str(&report);
        return Ok(output);
    }
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file());
    for entry in walker {
        let relative = relative_path(sandbox, entry.path());
        let report = search_file(entry.path(), &relative, matcher)?;
        output.push_str(&report);
        if output.len() > MAX_SEARCH_BYTES {
            output.push_str("... search output truncated at 50KiB\n");
            break;
        }
    }
    Ok(output)
}

/// Search one file and return its `¶PATH#TAG` section with context lines.
fn search_file(path: &Path, relative: &str, matcher: &grep_regex::RegexMatcher) -> Result<String, String> {
    let content = match read(path) {
        Ok(content) => content,
        Err(_) => return Ok(String::new()),
    };
    let text = String::from_utf8_lossy(&content);
    let lines = text.lines().collect::<Vec<_>>();
    let mut matched_rows: Vec<usize> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if matcher.is_match(line.as_bytes()).unwrap_or(false) {
            matched_rows.push(index);
        }
    }
    if matched_rows.is_empty() {
        return Ok(String::new());
    }
    let snapshot = crate::util::snapshots::SnapshotStore::new();
    let tag = snapshot.mint(relative, &text);
    let mut output = format!("¶{relative}#{tag}\n");
    let mut last_end: Option<usize> = None;
    for matched in matched_rows {
        let start = matched.saturating_sub(BEFORE_CONTEXT);
        let end = (matched + AFTER_CONTEXT).min(lines.len().saturating_sub(1));
        if let Some(previous) = last_end {
            if start > previous + 1 {
                output.push_str(&format!("*{}:...\n", previous + 2));
            }
        }
        for row in start..=end {
            let marker = if row == matched {
                "*"
            } else {
                " "
            };
            output.push_str(&format!("{marker}{}:{}\n", row + 1, lines[row]));
        }
        last_end = Some(end);
    }
    Ok(output)
}

/// Render one relative display path for a file under the sandbox.
fn relative_path(sandbox: &Path, path: &Path) -> String {
    path.strip_prefix(sandbox)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

/// Format one root-level error message for the model.
fn format_root_error(root: &Path, error: &str) -> String {
    format!("search failed for '{}': {error}", root.display())
}
