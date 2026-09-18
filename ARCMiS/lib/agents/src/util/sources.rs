//! Source collection for the default agent. The harness walks the input
//! root and passes every readable file to the agent as content. The
//! harness serializes the collected sources into the typed prompt payload.

use anyhow::Context as _;

/// Ordered map of input-file path to content. Keys are paths relative to
/// the input root; the sort order keeps the rendered prompt stable.
pub type Sources = std::collections::BTreeMap<String, String>;

/// Walk `root` and collect every readable text file. Files that fail
/// `read_to_string` count as binary and are skipped. One file larger than
/// `per_file_cap` bytes is truncated to the cap. The root itself must be
/// readable.
pub fn collect(root: &std::path::Path, per_file_cap: u64) -> anyhow::Result<Sources> {
    let mut sources = Sources::new();
    let walk = Walk {
        root: std::path::PathBuf::from(root),
        per_file_cap,
    };
    walk.dir(root, &mut sources)?;
    tracing::info!(root = %root.display(), files = sources.len(), per_file_cap, "collected input sources");
    Ok(sources)
}

/// Walk state: the input root and the per-file byte cap.
struct Walk {
    root: std::path::PathBuf,
    per_file_cap: u64,
}

impl Walk {
    /// Read `dir` recursively, collecting readable text files into `sources`.
    fn dir(&self, dir: &std::path::Path, sources: &mut Sources) -> anyhow::Result<()> {
        let entries =
            std::fs::read_dir(dir).with_context(|| format!("input root not readable at {}", dir.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("input root entry read failed at {}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                self.dir(&path, sources)?;
                continue;
            }
            self.file(&path, sources)?;
        }
        Ok(())
    }

    /// Read one file, truncate to the cap, and insert it into `sources`.
    fn file(&self, path: &std::path::Path, sources: &mut Sources) -> anyhow::Result<()> {
        // A read failure means the file is not valid text. Skip it.
        let content = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => return Ok(()),
        };
        let rel = path
            .strip_prefix(&self.root)
            .with_context(|| format!("input path {} outside root {}", path.display(), self.root.display()))?;
        let truncated = truncate(&content, self.per_file_cap);
        if truncated.len() < content.len() {
            tracing::warn!(file = %rel.display(), per_file_cap = self.per_file_cap, "input file truncated to context cap");
        }
        sources.insert(rel.to_string_lossy().into_owned(), truncated);
        Ok(())
    }
}

/// Cut `text` to at most `max_bytes` bytes at a char boundary.
fn truncate(text: &str, max_bytes: u64) -> String {
    if text.len() as u64 <= max_bytes {
        return String::from(text);
    }
    let mut end = max_bytes as usize;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    String::from(&text[..end])
}
