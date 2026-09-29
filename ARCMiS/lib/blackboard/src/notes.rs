//! `notes.md`: the specialists' shared scratchpad. Capped like the plan.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;

use crate::plan::cap_text;

/// The notes file.
#[derive(Debug, Clone)]
pub struct NotesFile {
    path: PathBuf,
    /// Character ceiling; default 8000 per GVS5H.
    cap: usize,
}

impl NotesFile {
    /// Bind `notes.md` at `{dir}/notes.md` with the default cap.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self::with_cap(dir, 8000)
    }

    /// Bind with an explicit cap.
    #[must_use]
    pub fn with_cap(dir: &Path, cap: usize) -> Self {
        Self {
            path: dir.join("notes.md"),
            cap,
        }
    }

    /// Read the notes; empty string before the first write.
    pub fn read(&self) -> anyhow::Result<String> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(error.into()),
        }
    }

    /// Write the notes, truncating to the cap with an elision marker.
    pub fn write_capped(&self, text: &str) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.path.parent().context("notes dir")?)?;
        std::fs::write(&self.path, cap_text(text, self.cap))?;
        Ok(())
    }

    /// The configured cap.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.cap
    }
}
