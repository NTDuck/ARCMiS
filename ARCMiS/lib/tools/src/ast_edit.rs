//! `ast_edit` stages AST pattern rewrites as preview diffs.

use ::ast_grep_core::language::Language as _;
use ::ast_grep_core::tree_sitter::LanguageExt as _;
use ::ast_grep_language::SupportLang;

/// `ast_edit` stages AST rewrites and returns a preview diff for approval.
pub struct AstEdit {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for AstEdit {
    const NAME: &'static str = "ast_edit";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = AstEditArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Stage AST pattern rewrites as a preview diff without changing files.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
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

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        if args.ops.is_empty() {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::other(
                "ast_edit needs at least one op with 'pat' and 'out'.",
            ));
        }
        if args.paths.is_empty() {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::other(
                "ast_edit needs at least one path.",
            ));
        }
        let roots = resolve_roots(&self.root, &args.paths).map_err(::rig::tool::ToolExecutionError::other)?;
        let mut output = ::std::string::String::new();
        let mut changed = 0usize;
        for root in roots {
            let report = preview_root(&root, &self.root, &args.ops, &mut changed).map_err(|error| {
                ::rig::tool::ToolExecutionError::other(::std::format!(
                    "ast_edit failed for '{}': {error}",
                    root.display()
                ))
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
            output.push_str(&::std::format!(
                "{changed} file(s) staged. Review the diff, then call the resolve tool to apply it.\n"
            ));
        }
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(output))
    }
}

/// Arguments for `ast_edit`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct AstEditArgs {
    pub ops: Vec<OpSpec>,
    pub paths: Vec<::std::string::String>,
}

/// One requested pattern rewrite.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct OpSpec {
    pub pat: ::std::string::String,
    pub out: ::std::string::String,
}

/// Maximum bytes emitted per ast_edit call.
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
/// Maximum staged rewrites shown per file.
const MAX_MATCHES_PER_FILE: usize = 50;

/// Resolve requested roots under the sandbox root.
fn resolve_roots(
    root: &::std::path::Path,
    paths: &[::std::string::String],
) -> ::core::result::Result<Vec<::std::path::PathBuf>, ::std::string::String> {
    let mut resolved = Vec::with_capacity(paths.len());
    for path in paths {
        resolved.push(crate::util::path::path_sanitize(root, path)?);
    }
    ::core::result::Result::Ok(resolved)
}

/// Preview the rewrites over one root path and return diff text.
fn preview_root(
    root: &::std::path::Path,
    sandbox: &::std::path::Path,
    ops: &[OpSpec],
    changed: &mut usize,
) -> ::core::result::Result<String, ::std::string::String> {
    let mut output = ::std::string::String::new();
    if root.is_file() {
        let relative = relative_path(sandbox, root);
        let report = preview_file(root, &relative, ops, changed)?;
        output.push_str(&report);
        return ::core::result::Result::Ok(output);
    }
    let walker = ::walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(::core::result::Result::ok)
        .filter(|entry| entry.file_type().is_file());
    for entry in walker {
        let relative = relative_path(sandbox, entry.path());
        let report = preview_file(entry.path(), &relative, ops, changed)?;
        output.push_str(&report);
    }
    ::core::result::Result::Ok(output)
}

/// Compute the staged diff for one file, applying nothing.
fn preview_file(
    path: &::std::path::Path,
    relative: &str,
    ops: &[OpSpec],
    changed: &mut usize,
) -> ::core::result::Result<String, ::std::string::String> {
    let source = match ::std::fs::read_to_string(path) {
        ::core::result::Result::Ok(source) => source,
        ::core::result::Result::Err(_) => return ::core::result::Result::Ok(::std::string::String::new()),
    };
    let language = match SupportLang::from_path(relative) {
        ::core::option::Option::Some(language) => language,
        ::core::option::Option::None => return ::core::result::Result::Ok(::std::string::String::new()),
    };
    let parsed = language.ast_grep(&source);
    let mut diff = ::std::string::String::new();
    for op in ops {
        let pattern = ::ast_grep_core::Pattern::try_new(&op.pat, language)
            .map_err(|error| ::std::format!("bad pattern '{}': {error}", op.pat))?;
        let mut count = 0usize;
        for matched in parsed.root().find_all(&pattern) {
            if count >= MAX_MATCHES_PER_FILE {
                diff.push_str("... more matches elided\n");
                break;
            }
            count += 1;
            let line = matched.start_pos().line() + 1;
            diff.push_str(&::std::format!("-{line}:{}\n", matched.text()));
            diff.push_str(&::std::format!("+{line}:{}\n", op.out.trim_end()));
        }
    }
    if diff.is_empty() {
        return ::core::result::Result::Ok(::std::string::String::new());
    }
    *changed += 1;
    ::core::result::Result::Ok(::std::format!("staged (not applied): {relative}\n{diff}\n"))
}

/// Render one relative display path for a file under the sandbox.
fn relative_path(sandbox: &::std::path::Path, path: &::std::path::Path) -> ::std::string::String {
    path.strip_prefix(sandbox)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}
