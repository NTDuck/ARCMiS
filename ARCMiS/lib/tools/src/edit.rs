//! `edit` applies one hashline patch to one file inside the sandbox root.

use std::path::PathBuf;
use std::sync::Arc;

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};

/// `edit` applies hashline patches and consumes snapshot tags for validation.
pub struct Edit {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
    /// Shared snapshot cache. Tags must match the last read or write.
    pub snapshots: Arc<crate::util::snapshots::SnapshotStore>,
}

impl Tool for Edit {
    const NAME: &'static str = "edit";
    type Error = ToolExecutionError;
    type Args = EditArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Apply one hashline patch to one file inside the sandbox root.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "One patch: a '¶PATH#TAG' header, then ops 'replace N..M:' with '+TEXT' rows, 'delete N..M', or 'insert before N:' / 'insert after N:' / 'insert head:' / 'insert tail:'."
                }
            },
            "required": ["input"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let patch = parse_patch(&args.input).map_err(ToolExecutionError::other)?;
        let requested = patch.path.clone();
        let absolute = crate::util::path::path_sanitize(&self.root, &requested).map_err(ToolExecutionError::other)?;
        let snapshot = self
            .snapshots
            .lookup(&patch.tag)
            .ok_or_else(|| stale_tag_error(&requested, &patch.tag))
            .map_err(ToolExecutionError::other)?;
        let (snapshot_path, lines) = snapshot;
        if snapshot_path != requested {
            return Err(ToolExecutionError::other(format!(
                "tag '{}' belongs to '{snapshot_path}', not '{requested}'. Re-read the file.",
                patch.tag
            )));
        }
        let output = apply_ops(&lines, &patch.ops)
            .map_err(|error| ToolExecutionError::other(format!("edit failed for '{requested}': {error}")))?;
        let text = join_lines(&output);
        tokio::fs::write(&absolute, text.as_bytes())
            .await
            .map_err(|error| ToolExecutionError::other(format!("edit failed for '{requested}': {error}")))?;
        let tag = self.snapshots.mint(&requested, &text);
        Ok(ToolOutput::text(format!(
            "¶{requested}#{tag}\nApplied {} op(s) to '{requested}'. Re-read before the next edit.",
            patch.ops.len()
        )))
    }
}

/// Arguments for `edit`.
#[derive(Debug, serde::Deserialize)]
pub struct EditArgs {
    pub input: String,
}

/// One parsed patch operation.
#[derive(Debug)]
struct PatchOp {
    kind: OpKind,
    start: usize,
    end: usize,
    rows: Vec<String>,
}

/// Kind of one patch operation.
#[derive(Debug)]
enum OpKind {
    Replace,
    Delete,
    InsertBefore,
    InsertAfter,
    InsertHead,
    InsertTail,
}

/// One parsed patch: header plus operations.
#[derive(Debug)]
struct Patch {
    path: String,
    tag: String,
    ops: Vec<PatchOp>,
}

/// Parse the `¶PATH#TAG` header and the op lines from the patch input.
fn parse_patch(input: &str) -> Result<Patch, String> {
    let mut lines = input.lines();
    let header = lines.next().ok_or_else(|| "empty patch input. Start with a '¶PATH#TAG' header line.".to_owned())?;
    let header = header.trim();
    let body = header.strip_prefix('¶').ok_or_else(|| "patch must start with a '¶PATH#TAG' header line.".to_owned())?;
    let (path, tag) =
        body.rsplit_once('#').ok_or_else(|| "patch header must end with '#TAG'. Re-read the file.".to_owned())?;
    let path = path.trim().to_owned();
    let tag = tag.trim().to_owned();
    if path.is_empty() || tag.is_empty() {
        return Err("patch header is incomplete. Re-read the file.".to_owned());
    }
    let mut ops = Vec::new();
    let mut index = 0usize;
    let remaining: Vec<&str> = lines.collect();
    while index < remaining.len() {
        let line = remaining[index].trim_end();
        if line.trim().is_empty() {
            index += 1;
            continue;
        }
        let op = parse_op(line, &remaining[index + 1..])?;
        let consumed = op.rows.len();
        ops.push(op);
        index += 1 + header_rows_consumed(line, consumed);
    }
    if ops.is_empty() {
        return Err("patch has no operations after the header.".to_owned());
    }
    Ok(Patch {
        path,
        tag,
        ops,
    })
}

/// Count how many body rows belong to the op header just parsed.
fn header_rows_consumed(header: &str, row_count: usize) -> usize {
    let _ = row_count;
    if header.trim_end().ends_with(':') || header.trim().starts_with("delete ") {
        0
    } else {
        0
    }
}

/// Parse one op header plus its `+TEXT` body rows from the remaining lines.
fn parse_op(header: &str, rest: &[&str]) -> Result<PatchOp, String> {
    let trimmed = header.trim();
    if trimmed.starts_with("insert head:") {
        return Ok(PatchOp {
            kind: OpKind::InsertHead,
            start: 0,
            end: 0,
            rows: collect_rows(rest)?,
        });
    }
    if trimmed.starts_with("insert tail:") {
        return Ok(PatchOp {
            kind: OpKind::InsertTail,
            start: 0,
            end: 0,
            rows: collect_rows(rest)?,
        });
    }
    if let Some(target) = trimmed.strip_prefix("insert before ") {
        let line = parse_line_number(target.trim_end_matches(':'))?;
        return Ok(PatchOp {
            kind: OpKind::InsertBefore,
            start: line,
            end: line,
            rows: collect_rows(rest)?,
        });
    }
    if let Some(target) = trimmed.strip_prefix("insert after ") {
        let line = parse_line_number(target.trim_end_matches(':'))?;
        return Ok(PatchOp {
            kind: OpKind::InsertAfter,
            start: line,
            end: line,
            rows: collect_rows(rest)?,
        });
    }
    if let Some(range) = trimmed.strip_prefix("delete ") {
        let (start, end) = parse_range(range.trim_end_matches(':'))?;
        return Ok(PatchOp {
            kind: OpKind::Delete,
            start,
            end,
            rows: Vec::new(),
        });
    }
    if let Some(range) = trimmed.strip_prefix("replace ") {
        let (start, end) = parse_range(range.trim_end_matches(':'))?;
        return Ok(PatchOp {
            kind: OpKind::Replace,
            start,
            end,
            rows: collect_rows(rest)?,
        });
    }
    Err(format!("unknown op '{trimmed}'. Use 'replace N..M:', 'delete N..M', or 'insert before/after/head/tail:'."))
}

/// Collect the `+TEXT` body rows that follow one op header.
fn collect_rows(rest: &[&str]) -> Result<Vec<String>, String> {
    let mut rows = Vec::new();
    for line in rest {
        if !line.starts_with('+') {
            break;
        }
        rows.push(line[1..].to_owned());
    }
    Ok(rows)
}

/// Parse one 1-based line number.
fn parse_line_number(text: &str) -> Result<usize, String> {
    text.trim().parse::<usize>().map_err(|_| format!("bad line number '{text}' in patch op"))
}

/// Parse one `N..M` range with M optional (defaults to N).
fn parse_range(text: &str) -> Result<(usize, usize), String> {
    let (start_text, end_text) = match text.split_once("..") {
        Some((start, end)) => (start, Some(end)),
        None => (text, None),
    };
    let start = parse_line_number(start_text)?;
    let end = match end_text {
        Some(end) => parse_line_number(end)?,
        None => start,
    };
    if end < start {
        return Err(format!("range {start}..{end} has end before start"));
    }
    Ok((start, end))
}

/// Build the stale-tag error the model sees after the file changed.
fn stale_tag_error(path: &str, tag: &str) -> String {
    format!(
        "stale tag '{tag}' for '{path}'. The file changed since the last read. Re-read the file and use the new '¶PATH#TAG' header."
    )
}

/// Apply the operations in order to the snapshot lines.
fn apply_ops(lines: &[String], ops: &[PatchOp]) -> Result<Vec<String>, String> {
    let mut output: Vec<String> = lines.to_vec();
    for op in ops {
        match op.kind {
            OpKind::Replace => {
                let end = op.end.min(output.len());
                let start = op.start.min(end.saturating_add(1)).saturating_sub(1);
                if op.start > output.len() {
                    return Err(format!(
                        "replace {}..{} is past the end ({} lines). Re-read the file.",
                        op.start,
                        op.end,
                        output.len()
                    ));
                }
                let replacement: Vec<String> = op.rows.clone();
                output.splice(start..end, replacement);
            },
            OpKind::Delete => {
                if op.start > output.len() {
                    return Err(format!(
                        "delete {}..{} is past the end ({} lines). Re-read the file.",
                        op.start,
                        op.end,
                        output.len()
                    ));
                }
                let end = op.end.min(output.len());
                let start = op.start.saturating_sub(1);
                output.drain(start..end);
            },
            OpKind::InsertBefore => {
                let index = op.start.saturating_sub(1).min(output.len());
                splice_rows(&mut output, index, op);
            },
            OpKind::InsertAfter => {
                let index = op.start.min(output.len());
                splice_rows(&mut output, index, op);
            },
            OpKind::InsertHead => {
                splice_rows(&mut output, 0, op);
            },
            OpKind::InsertTail => {
                let index = output.len();
                splice_rows(&mut output, index, op);
            },
        }
    }
    Ok(output)
}

/// Insert one op's rows into the output at the given index.
fn splice_rows(output: &mut Vec<String>, index: usize, op: &PatchOp) {
    let mut insert: Vec<String> = Vec::with_capacity(op.rows.len());
    for row in &op.rows {
        insert.push(row.clone());
    }
    let tail = output.split_off(index);
    output.extend(insert);
    output.extend(tail);
}

/// Join lines back into one string with a trailing newline.
fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}
