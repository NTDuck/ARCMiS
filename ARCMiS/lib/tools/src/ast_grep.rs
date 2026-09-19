//! `ast_grep` finds AST pattern matches across source files.

use crate::util::paths::{relative_path, resolve_roots};
use ast_grep_core::language::Language as _;
use ast_grep_core::tree_sitter::LanguageExt as _;
use ast_grep_language::SupportLang;
use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

/// `ast_grep` finds AST pattern matches and returns tagged per-file output.
pub struct AstGrep {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
}

impl Tool for AstGrep {
    const NAME: &'static str = "ast_grep";
    type Error = ToolExecutionError;
    type Args = AstGrepArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Find AST pattern matches in source files under the sandbox root.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pat": {
                    "type": "string",
                    "description": "One AST pattern with $NAME metavars."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files or directories relative to the sandbox root."
                },
                "lang": {
                    "type": "string",
                    "description": "Language override such as 'cpp' for ambiguous '.h' files."
                },
                "skip": {
                    "type": "integer",
                    "description": "Matches to skip before collecting, for paging."
                }
            },
            "required": ["pat"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let roots = resolve_roots(&self.root, args.paths.as_deref()).map_err(ToolExecutionError::other)?;
        let skip = args.skip.unwrap_or(0).max(0) as usize;
        let mut output = String::new();
        let mut remaining = skip;
        for root in roots {
            let report =
                grep_root(&root, &self.root, &args.pat, args.lang.as_deref(), &mut remaining).map_err(|error| {
                    ToolExecutionError::other(format!("ast_grep failed for '{}': {error}", root.display()))
                })?;
            output.push_str(&report);
            if output.len() > MAX_OUTPUT_BYTES {
                output.push_str("... ast_grep output truncated at 50KiB\n");
                break;
            }
        }
        if output.is_empty() {
            output.push_str(&format!("no AST matches for '{}'\n", args.pat));
        }
        Ok(ToolOutput::text(output))
    }
}

/// Arguments for `ast_grep`.
#[derive(Debug, Deserialize)]
pub struct AstGrepArgs {
    pub pat: String,
    pub paths: Option<Vec<String>>,
    pub lang: Option<String>,
    pub skip: Option<i64>,
}

/// Maximum bytes emitted per ast_grep call.
const MAX_OUTPUT_BYTES: usize = crate::util::proc::OUTPUT_LIMIT;
/// Maximum matches shown per call.
const MAX_MATCHES: usize = 50;

/// Run the pattern over one root path and build the report.
fn grep_root(
    root: &Path,
    sandbox: &Path,
    pattern: &str,
    language: Option<&str>,
    remaining: &mut usize,
) -> Result<String, String> {
    let mut output = String::new();
    if root.is_file() {
        let relative = relative_path(sandbox, root);
        let report = grep_file(root, &relative, pattern, language, remaining)?;
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
        let report = grep_file(entry.path(), &relative, pattern, language, remaining)?;
        output.push_str(&report);
        if output.len() > MAX_OUTPUT_BYTES {
            break;
        }
    }
    Ok(output)
}

fn grep_file(
    path: &Path,
    relative: &str,
    pattern: &str,
    language: Option<&str>,
    remaining: &mut usize,
) -> Result<String, String> {
    let source = match read_to_string(path) {
        Ok(source) => source,
        Err(_) => return Ok(String::new()),
    };
    let language = resolve_language(language, relative)?;
    let parsed = language.ast_grep(&source);
    let pattern =
        ast_grep_core::Pattern::try_new(pattern, language).map_err(|error| format!("bad pattern: {error}"))?;
    let mut rows = Vec::new();
    for matched in parsed.root().find_all(&pattern) {
        if *remaining > 0 {
            *remaining -= 1;
            continue;
        }
        if rows.len() >= MAX_MATCHES {
            break;
        }
        let line = matched.start_pos().line() + 1;
        rows.push(format!("*{line}:{}", matched.text()));
    }
    if rows.is_empty() {
        return Ok(String::new());
    }
    let mut output = format!("¶{relative}#0000\n");
    for row in rows {
        output.push_str(&row);
        output.push('\n');
    }
    Ok(output)
}

/// Resolve one explicit language name or infer one from the file extension.
fn resolve_language(language: Option<&str>, relative: &str) -> Result<SupportLang, String> {
    if let Some(name) = language {
        return name.parse::<SupportLang>().map_err(|error| format!("unsupported language '{name}': {error:?}"));
    }
    SupportLang::from_path(relative).ok_or_else(|| format!("cannot infer a language for '{relative}'. Pass 'lang'."))
}
