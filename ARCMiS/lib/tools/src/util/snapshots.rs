//! Snapshot store for `¶PATH#TAG` hashline tags.
//!
//! Tools mint a tag after each write. The tag pins the file content at that
//! moment. Later edits quote the tag so the tool can detect a stale view.
//!
//! The store keeps two maps: path to (tag, lines) and tag to (path, lines).
//! It has no internal lock. Each stateful tool wraps it in an
//! `std::sync::Arc<...Mutex...>` of its own.
//!
//! Tag minting hashes the content with sha2 and mixes a uuid nonce. Equal
//! content at different paths or moments still gets distinct tags.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Store of content snapshots keyed by path and by minted tag.
#[derive(Debug, Default)]
pub struct SnapshotStore {
    /// Path to (tag, lines) for the latest snapshot of that path.
    by_path: Mutex<BTreeMap<String, (String, Vec<String>)>>,
    /// Tag to (path, lines) for every minted snapshot.
    by_tag: Mutex<BTreeMap<String, (String, Vec<String>)>>,
}

impl SnapshotStore {
    /// Create an empty shared store.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            by_path: Mutex::new(BTreeMap::new()),
            by_tag: Mutex::new(BTreeMap::new()),
        })
    }

    /// Mint a 4-hex uppercase tag for `text` at `path` and record the lines.
    pub fn mint(&self, path: &str, text: &str) -> String {
        let tag = self.next_tag(path, text);
        let lines = split_lines(text);
        self.by_path.lock().expect("snapshot store poisoned").insert(String::from(path), (tag.clone(), lines.clone()));
        self.by_tag.lock().expect("snapshot store poisoned").insert(tag.clone(), (String::from(path), lines));
        tag
    }

    /// Return (path, lines) for a minted tag.
    #[must_use]
    pub fn lookup(&self, tag: &str) -> Option<(String, Vec<String>)> {
        self.by_tag.lock().expect("snapshot store poisoned").get(tag).map(|(path, lines)| (path.clone(), lines.clone()))
    }

    /// Return (tag, lines) for the latest snapshot of `path`.
    #[must_use]
    pub fn lookup_by_path(&self, path: &str) -> Option<(String, Vec<String>)> {
        self.by_path.lock().expect("snapshot store poisoned").get(path).map(|(tag, lines)| (tag.clone(), lines.clone()))
    }

    /// Derive the next 4-hex uppercase tag from content hash plus a uuid nonce.
    fn next_tag(&self, path: &str, text: &str) -> String {
        std::iter::repeat(())
            .map(|()| mint_once(path, text))
            // The nonce makes collisions practically impossible. Loop anyway
            // so a repeated tag cannot overwrite an earlier snapshot.
            .find(|tag| !self.by_tag.lock().expect("snapshot store poisoned").contains_key(tag))
            .unwrap_or_else(|| mint_once(path, text))
    }
}

/// Hash path, text, and one uuid nonce into a 4-hex uppercase tag.
fn mint_once(path: &str, text: &str) -> String {
    use sha2::Digest;
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let mut hasher = sha2::Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(b"\0");
    hasher.update(text.as_bytes());
    hasher.update(b"\0");
    hasher.update(nonce.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(2).map(|byte| format!("{byte:02X}")).collect()
}

/// Split text into lines without line terminators.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned()).collect()
}
