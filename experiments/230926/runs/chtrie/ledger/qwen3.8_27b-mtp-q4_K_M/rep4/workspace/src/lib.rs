//! CH-Trie (coordinate hash trie) — a faithful Rust translation of the C
//! `chtrie` library (`chtrie_alloc` / `chtrie_walk` / `chtrie_del` /
//! `chtrie_free`).
//!
//! The C API signals errors through `errno`; here the two failure classes are
//! modeled as [`ChTrieError::Range`] (C `ERANGE`) and
//! [`ChTrieError::Capacity`] (C `ENOMEM` on node-pool exhaustion).
//! `chtrie_free` is replaced by Rust's drop semantics.

pub mod chtrie;

pub use crate::chtrie::{ChTrie, ChTrieError};
