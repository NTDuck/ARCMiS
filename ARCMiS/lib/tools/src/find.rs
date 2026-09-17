//! `find` walks the sandbox root and lists files by glob pattern.

/// `find` lists files that match globs, newest first, grouped by directory.
pub struct Find {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for Find {
    const NAME: &'static str = "find";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = FindArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "List files under the sandbox root that match one glob, newest first.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Glob patterns or directories relative to the sandbox root. Defaults to all files."
                },
                "hidden": {
                    "type": "boolean",
                    "description": "Include hidden files and directories."
                },
                "gitignore": {
                    "type": "boolean",
                    "description": "Respect .gitignore rules. Defaults to true."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of files to list."
                }
            },
            "required": []
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let limit = args.limit.unwrap_or(2000).max(1) as usize;
        let hidden = args.hidden.unwrap_or(false);
        let gitignore = args.gitignore.unwrap_or(true);
        let matchers = build_matchers(&args.paths).map_err(::rig::tool::ToolExecutionError::other)?;
        let mut files = walk_files(&self.root, hidden, gitignore, &matchers, limit)
            .map_err(::rig::tool::ToolExecutionError::other)?;
        files.sort_by(|left, right| right.modified.cmp(&left.modified));
        files.truncate(limit);
        let output = render_list(&files);
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(output))
    }
}

/// Arguments for `find`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct FindArgs {
    pub paths: Option<Vec<::std::string::String>>,
    pub hidden: Option<bool>,
    pub gitignore: Option<bool>,
    pub limit: Option<i64>,
}

/// One matched file with its modification time.
#[derive(::core::fmt::Debug)]
struct FoundFile {
    relative: ::std::string::String,
    modified: ::std::time::SystemTime,
}

/// Compile the requested glob patterns, or one match-all glob.
fn build_matchers(
    paths: &Option<Vec<::std::string::String>>,
) -> ::core::result::Result<::globset::GlobSet, ::std::string::String> {
    let mut builder = ::globset::GlobSetBuilder::new();
    match paths {
        Some(paths) if !paths.is_empty() => {
            for pattern in paths {
                let glob =
                    ::globset::Glob::new(pattern).map_err(|error| ::std::format!("bad glob '{pattern}': {error}"))?;
                builder.add(glob);
            }
        },
        _ => {
            let glob = ::globset::Glob::new("**/*").map_err(|error| ::std::format!("bad default glob: {error}"))?;
            builder.add(glob);
        },
    }
    builder.build().map_err(|error| ::std::format!("glob build failed: {error}"))
}

/// Walk the sandbox root and collect matching files.
fn walk_files(
    root: &::std::path::Path,
    hidden: bool,
    gitignore: bool,
    matchers: &::globset::GlobSet,
    limit: usize,
) -> ::core::result::Result<Vec<FoundFile>, ::std::string::String> {
    let mut files: Vec<FoundFile> = Vec::new();
    let walker = ::walkdir::WalkDir::new(root).follow_links(false);
    let _ = gitignore;
    for entry in walker.into_iter().filter_map(::core::result::Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = match entry.path().strip_prefix(root) {
            ::core::result::Result::Ok(relative) => relative.to_string_lossy().into_owned(),
            ::core::result::Result::Err(_) => continue,
        };
        if !hidden && relative.split('/').any(|part| part.starts_with('.')) {
            continue;
        }
        if !matchers.is_match(&relative) {
            continue;
        }
        let modified =
            entry.metadata().ok().and_then(|metadata| metadata.modified().ok()).unwrap_or(::std::time::UNIX_EPOCH);
        files.push(FoundFile {
            relative,
            modified,
        });
        if files.len() > limit * 4 {
            break;
        }
    }
    ::core::result::Result::Ok(files)
}

/// Render the grouped directory listing, newest file first per group.
fn render_files(files: &[FoundFile]) -> ::std::string::String {
    let mut groups: Vec<(String, Vec<&FoundFile>)> = Vec::new();
    for file in files {
        let directory = match file.relative.rfind('/') {
            ::core::option::Option::Some(index) => file.relative[..index].to_owned(),
            ::core::option::Option::None => ".".to_owned(),
        };
        match groups.last_mut() {
            ::core::option::Option::Some((current, rows)) if *current == directory => rows.push(file),
            _ => groups.push((directory, ::std::vec![file])),
        }
    }
    let mut output = ::std::string::String::new();
    for (directory, rows) in groups {
        output.push_str(&::std::format!("{directory}/\n"));
        for file in rows {
            let name = file.relative.rsplit('/').next().unwrap_or(&file.relative).to_owned();
            output.push_str(&::std::format!("  {name}\n"));
        }
    }
    output
}

/// Render the full listing with a leading count line.
fn render_list(files: &[FoundFile]) -> ::std::string::String {
    if files.is_empty() {
        return "no files matched\n".to_owned();
    }
    let mut output = ::std::format!("{} file(s) matched\n", files.len());
    output.push_str(&render_files(files));
    output
}
