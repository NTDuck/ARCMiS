//! `ast_edit` stages AST pattern rewrites as preview diffs.

use crate::util::paths::{relative_path, resolve_roots};
use ast_grep_core::language::Language as _;
use ast_grep_core::tree_sitter::LanguageExt as _;
use ast_grep_language::SupportLang;
use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

/// `ast_edit` stages AST rewrites and returns a preview diff for approval.
pub struct AstEdit {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
}

impl Tool for AstEdit {
    const NAME: &'static str = "ast_edit";
    type Error = ToolExecutionError;
    type Args = AstEditArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Stage AST pattern rewrites as a preview diff without changing files.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "ops": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "pat": { "type": "string", "description": "AST pattern with metavars." },
                            "out": { "type": "string", "description": "Rewrite template using the same metavars." }
                        },
                        "required": ["pat", "out"]
                    },
                    "description": "Rewrite operations to stage."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files or directories relative to the sandbox root."
                }
            },
            "required": ["ops", "paths"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.ops.is_empty() {
            return Err(ToolExecutionError::other("ast_edit needs at least one op with 'pat' and 'out'."));
        }
        if args.paths.is_empty() {
            return Err(ToolExecutionError::other("ast_edit needs at least one path."));
        }
        let roots = resolve_roots(&self.root, Some(&args.paths)).map_err(ToolExecutionError::other)?;
        let mut output = String::new();
        let mut changed = 0usize;
        for root in roots {
            let report = preview_root(&root, &self.root, &args.ops, &mut changed).map_err(|error| {
                ToolExecutionError::other(format!("ast_edit failed for '{}': {error}", root.display()))
            })?;
            output.push_str(&report);
            if output.len() > MAX_OUTPUT_BYTES {
                output.push_str("... ast_edit output truncated at 50KiB\n");
                break;
            }
        }
        if changed == 0 {
            output.push_str("No file matches the pattern. Nothing is staged.\n");
        } else {
            output.push_str(&format!(
                "{changed} file(s) staged. Review the diff, then call the resolve tool to apply it.\n"
            ));
        }
        Ok(ToolOutput::text(output))
    }
}

/// Arguments for `ast_edit`.
#[derive(Debug, serde::Deserialize)]
pub struct AstEditArgs {
    pub ops: Vec<OpSpec>,
    pub paths: Vec<String>,
}

/// One requested pattern rewrite.
#[derive(Debug, serde::Deserialize)]
pub struct OpSpec {
    pub pat: String,
    pub out: String,
}

/// Maximum bytes emitted per ast_edit call.
const MAX_OUTPUT_BYTES: usize = crate::util::proc::OUTPUT_LIMIT;
/// Maximum staged rewrites shown per file.
const MAX_MATCHES_PER_FILE: usize = 50;

/// Preview the rewrites over one root path and return diff text.
fn preview_root(root: &Path, sandbox: &Path, ops: &[OpSpec], changed: &mut usize) -> Result<String, String> {
    let mut output = String::new();
    if root.is_file() {
        let relative = relative_path(sandbox, root);
        let report = preview_file(root, &relative, ops, changed)?;
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
        let report = preview_file(entry.path(), &relative, ops, changed)?;
        output.push_str(&report);
    }
    Ok(output)
}

/// Compute the staged diff for one file, applying nothing.
fn preview_file(path: &Path, relative: &str, ops: &[OpSpec], changed: &mut usize) -> Result<String, String> {
    let source = match read_to_string(path) {
        Ok(source) => source,
        Err(_) => return Ok(String::new()),
    };
    let language = match SupportLang::from_path(relative) {
        Some(language) => language,
        None => return Ok(String::new()),
    };
    let parsed = language.ast_grep(&source);
    let mut diff = String::new();
    for op in ops {
        let pattern = ast_grep_core::Pattern::try_new(&op.pat, language)
            .map_err(|error| format!("bad pattern '{}': {error}", op.pat))?;
        let mut count = 0usize;
        for matched in parsed.root().find_all(&pattern) {
            if count >= MAX_MATCHES_PER_FILE {
                diff.push_str("... more matches elided\n");
                break;
            }
            count += 1;
            let line = matched.start_pos().line() + 1;
            diff.push_str(&format!("-{line}:{}\n", matched.text()));
            diff.push_str(&format!("+{line}:{}\n", op.out.trim_end()));
        }
    }
    if diff.is_empty() {
        return Ok(String::new());
    }
    *changed += 1;
    Ok(format!("staged (not applied): {relative}\n{diff}\n"))
}
