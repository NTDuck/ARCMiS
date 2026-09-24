//! CH-Trie — coordinate hash trie.
//!
//! Rust port of the C `chtrie` library (BSD-3-Clause, DONG Yuxuan).
//!
//! C → Rust symbol mapping:
//!   `chtrie_alloc(n, m)`            -> `ChTrie::new(n, m)`
//!   `chtrie_walk(tr, f, s, 0)`     -> `ChTrie::walk(f, s)`          (Option<u32>)
//!   `chtrie_walk(tr, f, s, 1)`     -> `ChTrie::walk_or_create(f, s)` (Result<u32, ChTrieError>)
//!   `chtrie_del(tr, f, s)`         -> `ChTrie::del(f, s)`
//!   `chtrie_free(tr)`              -> `Drop` (automatic via `Vec`)
//!   `struct chtrie_edge`           -> `Edge`
//!   `struct chtrie`                -> `ChTrie`
//!   `errno` (ERANGE/ENOMEM)        -> `ChTrieError::{Range, Capacity}`

use std::fmt;

/// One edge in the global hash table.
///
/// Replaces the C `struct chtrie_edge`. The C linked-list pointer `next`
/// becomes an index (`Option<usize>`) into the crate-internal `edges` pool.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    /// Next edge in the bucket chain, or `None` for the tail.
    pub next: Option<usize>,
    /// Source node index.
    pub from: u32,
    /// Symbol (edge label).
    pub sym: u32,
    /// Destination node index.
    pub to: u32,
}

/// Errors returned by the creating/allocating operations.
///
/// Replaces the C `errno` side effects (`ERANGE`, `ENOMEM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// `n`/`m` too large, or `ecap` would overflow. (C: `ERANGE`)
    Range,
    /// Node pool exhausted while creating a node. (C: `ENOMEM`)
    Capacity,
}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: implement
        unimplemented!()
    }
}

impl std::error::Error for ChTrieError {}

/// The coordinate hash trie.
///
/// Replaces the C `struct chtrie`.
pub struct ChTrie {
    /// Bucket heads: index into `edges`, or `None` for an empty bucket.
    /// C: `struct chtrie_edge **etab`.
    pub etab: Vec<Option<usize>>,
    /// Edge pool (replaces the per-node `malloc`'d edge nodes).
    pub edges: Vec<Edge>,
    /// Recycled node indices (stack). C: `int *idxpool`.
    pub idxpool: Vec<u32>,
    /// Top of the recycled-index pool. C: `int *idxptr`.
    pub idxptr: usize,
    /// Next fresh node index. C: `int idxmax`.
    pub idxmax: u32,
    /// Maximum number of nodes. C: `int maxn`.
    pub maxn: u32,
    /// Alphabet size. C: `int alphsz`.
    pub alphsz: u32,
    /// Hash-table capacity. C: `int ecap`.
    pub ecap: usize,
}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    ///
    /// C: `chtrie_alloc(n, m)`. `n`/`m` are clamped to at least 1.
    /// Returns `Err(ChTrieError::Range)` where C would set `errno = ERANGE`.
    pub fn new(n: u32, m: u32) -> Result<Self, ChTrieError> {
        // TODO: implement
        unimplemented!()
    }

    /// Look up the child of `from` reached by `sym` without creating it.
    ///
    /// C: `chtrie_walk(tr, from, sym, 0)`. Returns `None` on a miss
    /// (C returned `-1` with no error side effect).
    pub fn walk(&self, from: u32, sym: u32) -> Option<u32> {
        // TODO: implement
        unimplemented!()
    }

    /// Look up the child of `from` reached by `sym`, creating it if absent.
    ///
    /// C: `chtrie_walk(tr, from, sym, 1)`. Returns
    /// `Err(ChTrieError::Capacity)` where C would set `errno = ENOMEM`.
    pub fn walk_or_create(&mut self, from: u32, sym: u32) -> Result<u32, ChTrieError> {
        // TODO: implement
        unimplemented!()
    }

    /// Delete the edge `(from, sym)`, recycling the child index.
    ///
    /// C: `chtrie_del(tr, from, sym)`. No-op if the edge is absent.
    pub fn del(&mut self, from: u32, sym: u32) {
        // TODO: implement
        unimplemented!()
    }
}

// C: `chtrie_free` — no manual `Drop` needed; the `Vec`s free themselves.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_clamps_small_values() {
        // TODO: `ChTrie::new(0, 0)` succeeds with maxn == 1, alphsz == 1.
        unimplemented!()
    }

    #[test]
    fn alloc_rejects_huge_n() {
        // TODO: `ChTrie::new(u32::MAX, 1)` returns `Err(ChTrieError::Range)`.
        unimplemented!()
    }

    #[test]
    fn walk_miss_and_create() {
        // TODO: `walk` misses, `walk_or_create` creates, `walk` then hits.
        unimplemented!()
    }

    #[test]
    fn del_absent_is_noop() {
        // TODO: `del` on a missing edge leaves the trie unchanged.
        unimplemented!()
    }

    #[test]
    fn index_recycled_after_del() {
        // TODO: after `del`, a subsequent `walk_or_create` reuses the index.
        unimplemented!()
    }
}
