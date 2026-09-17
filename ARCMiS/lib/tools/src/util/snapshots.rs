//! Snapshot store for `¶PATH#TAG` hashline tags.
//!
//! Tools mint a tag after each write. The tag pins the file content at that
//! moment. Later edits quote the tag so the tool can detect a stale view.
//!
//! The store keeps two maps: path to (tag, lines) and tag to (path, lines).
//! It has no internal lock. Each stateful tool wraps it in an
//! `::std::sync::Arc<...Mutex...>` of its own.
//!
//! Tag minting hashes the content with sha2 and mixes a uuid nonce. Equal
//! content at different paths or moments still gets distinct tags.

use ::std::collections::BTreeMap;

/// Store of content snapshots keyed by path and by minted tag.
#[derive(::core::fmt::Debug, ::core::default::Default)]
pub struct SnapshotStore {
    /// Path to (tag, lines) for the latest snapshot of that path.
    by_path: ::std::sync::Mutex<
        BTreeMap<::std::string::String, (::std::string::String, ::std::vec::Vec<::std::string::String>)>,
    >,
    /// Tag to (path, lines) for every minted snapshot.
    by_tag: ::std::sync::Mutex<
        BTreeMap<::std::string::String, (::std::string::String, ::std::vec::Vec<::std::string::String>)>,
    >,
}

impl SnapshotStore {
    /// Create an empty shared store.
    #[must_use]
    pub fn new() -> ::std::sync::Arc<Self> {
        ::std::sync::Arc::new(Self {
            by_path: ::std::sync::Mutex::new(BTreeMap::new()),
            by_tag: ::std::sync::Mutex::new(BTreeMap::new()),
        })
    }

    /// Mint a 4-hex uppercase tag for `text` at `path` and record the lines.
    pub fn mint(&self, path: &str, text: &str) -> ::std::string::String {
        let tag = self.next_tag(path, text);
        let lines = split_lines(text);
        self.by_path.lock().expect("snapshot store poisoned").insert(
            ::std::string::String::from(path),
            (::std::clone::Clone::clone(&tag), ::std::clone::Clone::clone(&lines)),
        );
        self.by_tag
            .lock()
            .expect("snapshot store poisoned")
            .insert(::std::clone::Clone::clone(&tag), (::std::string::String::from(path), lines));
        tag
    }

    /// Return (path, lines) for a minted tag.
    #[must_use]
    pub fn lookup(
        &self,
        tag: &str,
    ) -> ::core::option::Option<(::std::string::String, ::std::vec::Vec<::std::string::String>)> {
        self.by_tag
            .lock()
            .expect("snapshot store poisoned")
            .get(tag)
            .map(|(path, lines)| (::std::clone::Clone::clone(path), ::std::clone::Clone::clone(lines)))
    }

    /// Return (tag, lines) for the latest snapshot of `path`.
    #[must_use]
    pub fn lookup_by_path(
        &self,
        path: &str,
    ) -> ::core::option::Option<(::std::string::String, ::std::vec::Vec<::std::string::String>)> {
        self.by_path
            .lock()
            .expect("snapshot store poisoned")
            .get(path)
            .map(|(tag, lines)| (::std::clone::Clone::clone(tag), ::std::clone::Clone::clone(lines)))
    }
}

/// Render a snapshot as a `¶PATH#TAG` header plus `LINE:TEXT` rows.
///
/// `skipped` holds inclusive 1-based line ranges the caller omitted. Each
/// range adds a footer note that tells the model to re-read those ranges.
#[must_use]
pub fn hashline_render(
    path: &str,
    tag: &str,
    lines: &[::std::string::String],
    skipped: &[(usize, usize)],
) -> ::std::string::String {
    let header = ::std::format!("¶{path}#{tag}");
    let mut output = ::std::vec![header];
    output.extend(lines.iter().enumerate().map(|(index, line)| ::std::format!("{}:{line}", index + 1)));
    let elided_total = skipped.iter().map(|(start, end)| end - start + 1).sum::<usize>();
    if elided_total > 0 {
        let ranges = skipped
            .iter()
            .map(|(start, end)| ::std::format!("{start}-{end}"))
            .collect::<::std::vec::Vec<_>>()
            .join(", ");
        output.push(::std::format!("[{elided_total} lines elided at {ranges}; re-read needed ranges]"));
    }
    output.join("\n")
}

/// Derive the next 4-hex uppercase tag from content hash plus a uuid nonce.
impl SnapshotStore {
    fn next_tag(&self, path: &str, text: &str) -> ::std::string::String {
        ::core::iter::repeat(())
            .map(|()| mint_once(path, text))
            // The nonce makes collisions practically impossible. Loop anyway
            // so a repeated tag cannot overwrite an earlier snapshot.
            .find(|tag| !self.by_tag.lock().expect("snapshot store poisoned").contains_key(tag))
            .unwrap_or_else(|| mint_once(path, text))
    }
}

/// Hash path, text, and one uuid nonce into a 4-hex uppercase tag.
fn mint_once(path: &str, text: &str) -> ::std::string::String {
    use ::sha2::Digest;
    let nonce = ::uuid::Uuid::new_v4().simple().to_string();
    let mut hasher = ::sha2::Sha256::new();
    ::sha2::Digest::update(&mut hasher, path.as_bytes());
    ::sha2::Digest::update(&mut hasher, b"\0");
    ::sha2::Digest::update(&mut hasher, text.as_bytes());
    ::sha2::Digest::update(&mut hasher, b"\0");
    ::sha2::Digest::update(&mut hasher, nonce.as_bytes());
    let digest = ::sha2::Digest::finalize(hasher);
    digest.iter().take(2).map(|byte| ::std::format!("{byte:02X}")).collect::<::std::string::String>()
}

/// Split text into lines without line terminators.
fn split_lines(text: &str) -> ::std::vec::Vec<::std::string::String> {
    if text.is_empty() {
        return ::std::vec::Vec::new();
    }
    text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned()).collect::<::std::vec::Vec<_>>()
}
