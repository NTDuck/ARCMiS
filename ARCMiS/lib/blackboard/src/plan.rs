//! `plan.md`: the orchestrator's capped working plan. GVS5H's cap keeps the
//! file inside one model read; the cap is a config value, not a magic number.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;

/// The plan file.
#[derive(Debug, Clone)]
pub struct PlanFile {
    path: PathBuf,
    /// Character ceiling; default 4000 per GVS5H.
    cap: usize,
}

impl PlanFile {
    /// Bind `plan.md` at `{dir}/plan.md` with the default cap.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self::with_cap(dir, 4000)
    }

    /// Bind with an explicit cap.
    #[must_use]
    pub fn with_cap(dir: &Path, cap: usize) -> Self {
        Self {
            path: dir.join("plan.md"),
            cap,
        }
    }

    /// Read the whole plan; empty string before the first write.
    pub fn read(&self) -> anyhow::Result<String> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(error.into()),
        }
    }

    /// Write the plan, truncating to the cap with an elision marker.
    pub fn write_capped(&self, text: &str) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.path.parent().context("plan dir")?)?;
        let capped = cap_text(text, self.cap);
        std::fs::write(&self.path, capped)?;
        Ok(())
    }

    /// The configured cap.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.cap
    }
}

/// Truncate `text` to `cap` chars, appending an elision marker.
pub(crate) fn cap_text(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text.to_owned();
    }
    let kept: String = text.chars().take(cap).collect();
    format!("{kept}\n... [plan truncated at {cap} chars; re-plan to stay inside the cap]")
}
