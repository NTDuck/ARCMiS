//! `ast_grep` finds AST pattern matches across source files.

use ::ast_grep_core::language::Language as _;
use ::ast_grep_core::tree_sitter::LanguageExt as _;
use ::ast_grep_language::SupportLang;

/// `ast_grep` finds AST pattern matches and returns tagged per-file output.
pub struct AstGrep {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for AstGrep {
    const NAME: &'static str = "ast_grep";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = AstGrepArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Find AST pattern matches in source files under the sandbox root.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
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

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let roots = resolve_roots(&self.root, &args.paths).map_err(::rig::tool::ToolExecutionError::other)?;
        let skip = args.skip.unwrap_or(0).max(0) as usize;
        let mut output = ::std::string::String::new();
        let mut remaining = skip;
        for root in roots {
            let report =
                grep_root(&root, &self.root, &args.pat, args.lang.as_deref(), &mut remaining).map_err(|error| {
                    ::rig::tool::ToolExecutionError::other(::std::format!(
                        "ast_grep failed for '{}': {error}",
                        root.display()
                    ))
                })?;
            output.push_str(&report);
            if output.len() > MAX_OUTPUT_BYTES {
                output.push_str("... ast_grep output truncated at 50KiB\n");
                break;
            }
        }
        if output.is_empty() {
            output.push_str(&::std::format!("no AST matches for '{}'\n", args.pat));
        }
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(output))
    }
}

/// Arguments for `ast_grep`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct AstGrepArgs {
    pub pat: ::std::string::String,
    pub paths: Option<Vec<::std::string::String>>,
    pub lang: Option<::std::string::String>,
    pub skip: Option<i64>,
}

/// Maximum bytes emitted per ast_grep call.
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
/// Maximum matches shown per call.
const MAX_MATCHES: usize = 50;

/// Resolve one root path to match.
fn resolve_roots(
    root: &::std::path::Path,
    paths: &Option<Vec<::std::string::String>>,
) -> ::core::result::Result<Vec<::std::path::PathBuf>, ::std::string::String> {
    let requested = match paths {
        ::core::option::Option::Some(paths) if !paths.is_empty() => paths.clone(),
        _ => ::std::vec![".".to_owned()],
    };
    let mut resolved = Vec::with_capacity(requested.len());
    for path in requested {
        resolved.push(crate::util::path::path_sanitize(root, &path)?);
    }
    ::core::result::Result::Ok(resolved)
}

/// Run the pattern over one root path and build the report.
fn grep_root(
    root: &::std::path::Path,
    sandbox: &::std::path::Path,
    pattern: &str,
    language: Option<&str>,
    remaining: &mut usize,
) -> ::core::result::Result<String, ::std::string::String> {
    let mut output = ::std::string::String::new();
    if root.is_file() {
        let relative = relative_path(sandbox, root);
        let report = grep_file(root, &relative, pattern, language, remaining)?;
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
        let report = grep_file(entry.path(), &relative, pattern, language, remaining)?;
        output.push_str(&report);
        if output.len() > MAX_OUTPUT_BYTES {
            break;
        }
    }
    ::core::result::Result::Ok(output)
}

/// Match one file and append its `*LINE:text` rows when it matches.
fn grep_file(
    path: &::std::path::Path,
    relative: &str,
    pattern: &str,
    language: Option<&str>,
    remaining: &mut usize,
) -> ::core::result::Result<String, ::std::string::String> {
    let source = match ::std::fs::read_to_string(path) {
        ::core::result::Result::Ok(source) => source,
        ::core::result::Result::Err(_) => return ::core::result::Result::Ok(::std::string::String::new()),
    };
    let language = resolve_language(language, relative)?;
    let parsed = language.ast_grep(&source);
    let pattern =
        ::ast_grep_core::Pattern::try_new(pattern, language).map_err(|error| ::std::format!("bad pattern: {error}"))?;
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
        rows.push(::std::format!("*{line}:{}", matched.text()));
    }
    if rows.is_empty() {
        return ::core::result::Result::Ok(::std::string::String::new());
    }
    let mut output = ::std::format!("¶{relative}#0000\n");
    for row in rows {
        output.push_str(&row);
        output.push('\n');
    }
    ::core::result::Result::Ok(output)
}

/// Resolve one explicit language name or infer one from the file extension.
fn resolve_language(
    language: Option<&str>,
    relative: &str,
) -> ::core::result::Result<SupportLang, ::std::string::String> {
    if let ::core::option::Option::Some(name) = language {
        return name.parse::<SupportLang>().map_err(|error| ::std::format!("unsupported language '{name}': {error:?}"));
    }
    SupportLang::from_path(relative)
        .ok_or_else(|| ::std::format!("cannot infer a language for '{relative}'. Pass 'lang'."))
}

/// Render one relative display path for a file under the sandbox.
fn relative_path(sandbox: &::std::path::Path, path: &::std::path::Path) -> ::std::string::String {
    path.strip_prefix(sandbox)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}
