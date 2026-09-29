//! `edit` applies one hashline patch to one or more files inside the sandbox
//! root through `oxi_hashline::Patcher`.

use std::path::PathBuf;
use std::sync::Arc;

use rig::tool::Tool;
use rig::tool::ToolContext;
use rig::tool::ToolExecutionError;
use rig::tool::ToolOutput;
use serde::Deserialize;

/// Consecutive byte-identical no-op failures tolerated on one file/payload
/// before a hard error (oh-my-pi noop-loop-guard).
const NOOP_GUARD_LIMIT: usize = 3;

/// `edit` routes hashline patches through `oxi_hashline::apply_edits` via the
/// shared `Patcher`. Stale tags and no-ops are typed errors the model recovers
/// from by re-reading.
pub struct Edit {
    /// Root directory. Tool paths resolve inside it.
    pub root: PathBuf,
    /// Shared patcher: owns the `HashlineFs` impl over `root` and the shared
    /// snapshot store.
    pub patcher: Arc<oxi_hashline::Patcher>,
    /// No-op loop guard: (path, payload-hash) -> consecutive count.
    noop_counts: Arc<std::sync::Mutex<std::collections::HashMap<u64, usize>>>,
}

impl Edit {
    /// Construct the edit tool over `root` with the given snapshot store.
    pub fn new(root: PathBuf, snapshots: Arc<dyn oxi_hashline::SnapshotStore>) -> Arc<Self> {
        let fs = Arc::new(RootFs {
            root: root.clone(),
        });
        let patcher = Arc::new(oxi_hashline::Patcher::new(fs, snapshots));
        Arc::new(Self {
            root,
            patcher,
            noop_counts: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        })
    }
}

impl Edit {
    /// Envelope metadata for `edit`.
    pub(crate) const METADATA: crate::envelope::ToolMetadata = crate::envelope::base_metadata(
        "edit",
        "Applies one hashline patch to one or more files.",
        crate::envelope::ToolCategory::FileSystem,
        crate::envelope::SideEffectClass::WriteLocal,
        crate::envelope::CostClass::Medium,
        false,
    );
}

impl Tool for Edit {
    type Args = EditArgs;
    type Error = ToolExecutionError;
    type Output = ToolOutput;

    const NAME: &'static str = "edit";

    fn description(&self) -> String {
        "Apply one hashline patch to one or more files inside the sandbox root.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "One patch: one '[PATH#TAG]' header per file, then ops 'SWAP start.=end:' with '+TEXT' rows, 'DEL start.=end', 'INS.PRE N:' / 'INS.POST N:' / 'INS.HEAD:' / 'INS.TAIL:' with '+TEXT' rows."
                }
            },
            "required": ["patch"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let patch = oxi_hashline::split_patch_input(&args.patch, None)
            .map_err(|error| ToolExecutionError::other(error.to_string()))?;
        let payload_key = payload_fingerprint(&args.patch);
        let result = self.patcher.apply(&patch).await;
        match result {
            Ok(applied) => {
                self.noop_counts.lock().expect("noop guard poisoned").clear();
                let mut output = String::new();
                for section in &applied.sections {
                    output.push_str(&format!(
                        "[{}#{}]\nApplied to '{}' (first changed line: {}).\n",
                        section.path,
                        section.new_hash,
                        section.path,
                        section.first_changed_line.map(|l| l.to_string()).unwrap_or_else(|| "none".to_owned())
                    ));
                }
                Ok(ToolOutput::text(output))
            },
            Err(oxi_hashline::mismatch::HashlineError::NoOp {
                path,
            }) => {
                let mut counts = self.noop_counts.lock().expect("noop guard poisoned");
                let count = {
                    let slot = counts.entry(payload_key).or_insert(0);
                    *slot += 1;
                    *slot
                };
                let trip = count >= NOOP_GUARD_LIMIT;
                if trip {
                    counts.clear();
                }
                let message = if trip {
                    format!(
                        "Edit to {path} is a no-op and has repeated {NOOP_GUARD_LIMIT} times. Stop editing this file \
                         with the same patch; the change you want is already present. Re-read the file and verify."
                    )
                } else {
                    format!(
                        "Edit to {path} resulted in no changes ({count}/{NOOP_GUARD_LIMIT} of the no-op guard). The \
                         file already matches your patch. Re-read the file."
                    )
                };
                Err(ToolExecutionError::other(message))
            },
            Err(error) => Err(ToolExecutionError::other(error.to_string())),
        }
    }
}

/// Arguments for `edit`.
#[derive(Debug, Deserialize)]
pub struct EditArgs {
    pub patch: String,
}

/// Stable 64-bit fingerprint of the patch text for the no-op guard.
fn payload_fingerprint(patch: &str) -> u64 {
    // FNV-1a: stable, dependency-free, and collision-resistant enough to
    // distinguish repeated payloads of the same file.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in patch.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// `HashlineFs` implementation rooted at the tool's root directory.
struct RootFs {
    root: PathBuf,
}

impl RootFs {
    /// Resolve one root-relative path, refusing traversal outside the root.
    fn resolve(&self, path: &str) -> Result<PathBuf, oxi_hashline::mismatch::HashlineError> {
        crate::util::path::path_sanitize(&self.root, path).map_err(|error| {
            oxi_hashline::mismatch::HashlineError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, error))
        })
    }
}

#[async_trait::async_trait]
impl oxi_hashline::HashlineFs for RootFs {
    async fn read_text(&self, path: &str) -> Result<String, oxi_hashline::mismatch::HashlineError> {
        let resolved = self.resolve(path)?;
        match tokio::fs::read_to_string(&resolved).await {
            Ok(text) => Ok(oxi_hashline::normalize_to_lf(oxi_hashline::strip_bom(&text).text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
                Err(oxi_hashline::mismatch::HashlineError::NotFound {
                    path: path.to_owned(),
                }),
            Err(error) => Err(oxi_hashline::mismatch::HashlineError::Io(error)),
        }
    }

    async fn write_text(&self, path: &str, text: &str) -> Result<String, oxi_hashline::mismatch::HashlineError> {
        let resolved = self.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(oxi_hashline::mismatch::HashlineError::Io)?;
        }
        // Atomic write: temp file in the target directory, then rename.
        let temp = resolved.with_extension("arcmis.tmp");
        tokio::fs::write(&temp, text.as_bytes()).await.map_err(oxi_hashline::mismatch::HashlineError::Io)?;
        tokio::fs::rename(&temp, &resolved).await.map_err(oxi_hashline::mismatch::HashlineError::Io)?;
        Ok(path.to_owned())
    }

    fn canonical_path(&self, path: &str) -> String {
        crate::util::path::path_sanitize(&self.root, path)
            .map(|resolved| resolved.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.to_owned())
    }
}

crate::impl_envelope!(Edit);
