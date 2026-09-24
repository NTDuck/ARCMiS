//! # CH-Trie
//!
//! *CH-Trie* is the official Rust library of the *coordinate hash trie*.
//!
//! The coordinate hash trie is a trie variant balancing between time, space,
//! and simplicity.
//!
//! The basic idea is very simple.
//! We use a global hash table to store all edges in a Trie.
//! Each edge is stored as a dictionary item `(from, symb)->to`.
//!
//! We use a special hash function:
//!
//! ```text
//! h(from, symb) = (from*m + symb) mod H
//! ```
//!
//! where `m` is the size of the alphabet, and `H` is the number of slots in
//! the hash table. For a trie with `n` nodes, we take `H = (n-1)/alpha` where
//! `alpha` is the constant load factor. **No rehashing, resizing, or
//! reallocation is required.**
//!
//! The time complexity of transition from one node to another is O(1) for
//! the average case and O(m) for the worst case.
//! The space complexity is O(n), unrelated to `m`.
//!
//! See <https://arxiv.org/abs/2302.03690> for the analysis and proof.

use std::error::Error;
use std::fmt;

/// Errors that can occur when operating on a [`ChTrie`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// The requested size is out of range (analogous to `ERANGE`).
    Range,
    /// The trie has no more node capacity (analogous to `ENOMEM`).
    Capacity,
    /// The requested child does not exist (analogous to `chtrie_walk`
    /// returning `-1`).
    NotFound,
}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChTrieError::Range => write!(f, "size out of range"),
            ChTrieError::Capacity => write!(f, "no more node capacity"),
            ChTrieError::NotFound => write!(f, "child not found"),
        }
    }
}

impl Error for ChTrieError {}

/// A single edge in the global hash table.
#[derive(Clone)]
struct Edge {
    /// Index of the next edge in the same bucket, or `None` if this is the
    /// last edge in the bucket.
    next: Option<usize>,
    from: i32,
    sym: i32,
    to: i32,
}

/// A coordinate hash trie.
///
/// Nodes in the trie are indexed by non-negative integers less than `n`.
/// The root node is indexed by 0.
/// Symbols are non-negative integers less than `m`.
pub struct ChTrie {
    /// Global hash table of edges.
    etab: Vec<Option<Edge>>,
    /// Pool of available node indexes.
    idxpool: Vec<i32>,
    /// Number of indexes currently in the pool.
    idxptr: usize,
    /// Next fresh node index.
    idxmax: i32,
    /// Maximum number of nodes.
    maxn: usize,
    /// Alphabet size.
    alphsz: usize,
    /// Number of slots in the edge hash table.
    ecap: usize,
}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes, and the alphabet size `m`.
    ///
    /// If `n` or `m` is less than 1, they will be regulated to 1.
    ///
    /// Nodes in the trie are indexed by non-negative integers less than `n`.
    /// The root node is indexed by 0.
    /// Symbols are non-negative integers less than `m`.
    ///
    /// Upon success, return a trie.
    /// Otherwise, return [`ChTrieError::Range`].
    pub fn alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError> {
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        if (i32::MAX as usize).saturating_sub(n - 1) < (n - 1) / 3 {
            return Err(ChTrieError::Range);
        }
        let ecap = (n - 1) + (n - 1) / 3;
        Ok(ChTrie {
            etab: vec![None; ecap],
            idxpool: vec![0; n],
            idxptr: 0,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    fn hash(&self, from: i32, sym: i32) -> usize {
        let h = (from as u64) * (self.alphsz as u64) + sym as u64;
        (h % self.ecap as u64) as usize
    }

    /// Walk from one node to its child.
    ///
    /// If the child didn't exist and `creat` is non-zero,
    /// a new node will be created.
    ///
    /// Upon the child is found or created, return the index of the child.
    /// Otherwise, return [`ChTrieError::NotFound`].
    /// If `creat` is non-zero and this routine fails due to lack of capacity,
    /// return [`ChTrieError::Capacity`].
    pub fn walk(&mut self, from: i32, sym: i32, creat: bool) -> Result<i32, ChTrieError> {
        let h = self.hash(from, sym);
        let mut p = self.etab[h].as_ref();
        while let Some(e) = p {
            if e.from == from && e.sym == sym {
                return Ok(e.to);
            }
            p = e.next.and_then(|i| self.etab[i].as_ref());
        }
        if creat {
            if self.idxptr == 0 && self.idxmax >= self.maxn as i32 {
                return Err(ChTrieError::Capacity);
            }
            let to = if self.idxptr > 0 {
                let v = self.idxpool[self.idxptr - 1];
                self.idxptr -= 1;
                v
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };
            let next = self.etab[h].as_ref().and_then(|e| e.next);
            self.etab[h] = Some(Edge { next, from, sym, to });
            return Ok(to);
        }
        Err(ChTrieError::NotFound)
    }

    /// Delete a child node.
    ///
    /// The child node must be a leaf if it exists,
    /// or the behavior is undefined.
    ///
    /// If the child doesn't exist, the trie shall be left unchanged.
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = self.hash(from, sym);
        let mut prev: Option<usize> = None;
        let mut cur = h;
        let mut found = false;
        while let Some(e) = self.etab[cur].as_ref() {
            if e.from == from && e.sym == sym {
                found = true;
                break;
            }
            prev = Some(cur);
            match e.next {
                Some(i) => cur = i,
                None => break,
            }
        }
        if !found {
            return;
        }
        let e = self.etab[cur].clone().expect("edge found above");
        if let Some(prev) = prev {
            self.etab[prev].as_mut().expect("prev edge exists").next = e.next;
        } else {
            self.etab[h] = None;
        }
        self.idxpool[self.idxptr] = e.to;
        self.idxptr += 1;
    }
}
