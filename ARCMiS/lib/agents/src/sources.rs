//! Source collection for the default agent. The harness walks the input
//! root and passes every readable file to the agent as content. The agent
//! module renders the map; it never names files itself.

/// Ordered map of input-file path to content. Keys are paths relative to
/// the input root; the sort order keeps the rendered prompt stable.
pub type Sources = ::std::collections::BTreeMap<::std::string::String, ::std::string::String>;

/// Per-file cap in bytes. The caller derives it from the model context
/// window so the cap scales with the configured budget:
///
/// per_file_cap_bytes = num_ctx_tokens / 4 * BYTES_PER_TOKEN
///
/// One quarter of the window leaves room for the preamble, the other
/// files, the transcript, and the reply. Tokens here average 4 bytes of
/// source text.
pub const BYTES_PER_TOKEN: u64 = 4;

/// Walk `root` and collect every readable text file. Files that fail
/// `read_to_string` count as binary and are skipped. One file larger than
/// `per_file_cap` bytes is truncated to the cap. The root itself must be
/// readable.
pub fn collect(root: &::std::path::Path, per_file_cap: u64) -> ::core::result::Result<Sources, ::std::string::String> {
    let mut sources = Sources::new();
    let walk = Walk {
        root: ::std::path::PathBuf::from(root),
        per_file_cap,
    };
    walk.dir(root, &mut sources)?;
    ::tracing::info!(root = %root.display(), files = sources.len(), per_file_cap, "collected input sources");
    ::core::result::Result::Ok(sources)
}

/// Walk state: the input root and the per-file byte cap.
struct Walk {
    root: ::std::path::PathBuf,
    per_file_cap: u64,
}

impl Walk {
    /// Read `dir` recursively, collecting readable text files into `sources`.
    fn dir(&self, dir: &::std::path::Path, sources: &mut Sources) -> ::core::result::Result<(), ::std::string::String> {
        let entries = ::std::fs::read_dir(dir)
            .map_err(|error| ::std::format!("input root not readable at {}: {error}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| ::std::format!("input root entry read failed at {}: {error}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                self.dir(&path, sources)?;
                continue;
            }
            self.file(&path, sources)?;
        }
        ::core::result::Result::Ok(())
    }

    /// Read one file, truncate to the cap, and insert it into `sources`.
    fn file(
        &self,
        path: &::std::path::Path,
        sources: &mut Sources,
    ) -> ::core::result::Result<(), ::std::string::String> {
        let content = match ::std::fs::read_to_string(path) {
            ::core::result::Result::Ok(text) => text,
            ::core::result::Result::Err(_) => return ::core::result::Result::Ok(()),
        };
        let rel = path.strip_prefix(&self.root).map_err(|error| {
            ::std::format!("input path {} outside root {}: {error}", path.display(), self.root.display())
        })?;
        let truncated = truncate(&content, self.per_file_cap);
        if truncated.len() < content.len() {
            ::tracing::warn!(file = %rel.display(), per_file_cap = self.per_file_cap, "input file truncated to context cap");
        }
        sources.insert(rel.to_string_lossy().into_owned(), truncated);
        ::core::result::Result::Ok(())
    }
}

/// Cut `text` to at most `max_bytes` bytes at a char boundary.
fn truncate(text: &str, max_bytes: u64) -> ::std::string::String {
    if text.len() as u64 <= max_bytes {
        return ::std::string::String::from(text);
    }
    let mut end = max_bytes as usize;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    ::std::string::String::from(&text[..end])
}

/// Render the sources into one prompt block. Each file gets a header line
/// with its relative path. Empty maps render as an empty block; the caller
/// decides whether that is an error.
pub fn render(sources: &Sources) -> ::std::string::String {
    let mut out = ::std::string::String::new();
    for (rel, content) in sources {
        out.push_str(&::std::format!("=== FILE {rel} ===\n{content}\n"));
    }
    out
}
