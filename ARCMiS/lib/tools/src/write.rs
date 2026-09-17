//! `write` creates or overwrites one file inside the sandbox root.

/// `write` writes one file under the root and returns a fresh hashline tag.
pub struct Write {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
    /// Shared snapshot cache. The write mints a tag for the new content.
    pub snapshots: ::std::sync::Arc<crate::util::snapshots::SnapshotStore>,
}

impl ::rig::tool::Tool for Write {
    const NAME: &'static str = "write";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = WriteArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Create or overwrite one file inside the sandbox root.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path relative to the sandbox root."
                },
                "content": {
                    "type": "string",
                    "description": "Full file content. Hashline headers and line prefixes are stripped."
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let path =
            crate::util::path::path_sanitize(&self.root, &args.path).map_err(::rig::tool::ToolExecutionError::other)?;
        let cleaned = strip_echo(&args.content);
        let text = if cleaned.ends_with('\n') || cleaned.is_empty() {
            cleaned
        } else {
            ::std::format!("{cleaned}\n")
        };
        let requested = args.path.clone();
        let absolute = path.clone();
        let bytes = text.len();
        let parent = match path.parent() {
            ::core::option::Option::Some(parent) => parent.to_path_buf(),
            ::core::option::Option::None => self.root.clone(),
        };
        ::tokio::fs::create_dir_all(parent).await.map_err(|error| {
            ::rig::tool::ToolExecutionError::other(::std::format!("write failed for '{requested}': {error}"))
        })?;
        ::tokio::fs::write(&absolute, text.as_bytes()).await.map_err(|error| {
            ::rig::tool::ToolExecutionError::other(::std::format!("write failed for '{requested}': {error}"))
        })?;
        let tag = self.snapshots.mint(&requested, &text);
        let header = ::std::format!("¶{requested}#{tag}\n");
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(::std::format!(
            "{header}Wrote {bytes} bytes to '{requested}'."
        )))
    }
}

/// Arguments for `write`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct WriteArgs {
    pub path: ::std::string::String,
    pub content: ::std::string::String,
}

/// Strip pasted `¶PATH#TAG` headers, `LINE:` prefixes, and `+` body rows
/// that models echo back from `read` output.
fn strip_echo(content: &str) -> ::std::string::String {
    let mut lines: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.starts_with('¶') {
            continue;
        }
        let body = match line.find(':') {
            Some(index) if index <= 9 && line[..index].bytes().all(|byte| byte.is_ascii_digit()) => &line[index + 1..],
            _ => line,
        };
        lines.push(body.strip_prefix('+').unwrap_or(body));
    }
    let mut output = lines.join("\n");
    if !content.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }
    output
}
