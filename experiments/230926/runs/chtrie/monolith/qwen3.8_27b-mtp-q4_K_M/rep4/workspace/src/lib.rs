//! CH-Trie: the coordinate hash trie.
//!
//! The coordinate hash trie is a trie variant balancing between time, space,
//! and simplicity.
//!
//! The basic idea is very simple. We use a global hash table to store all
//! edges in a trie. Each edge is stored as a dictionary item
//! `(from, sym) -> to`.
//!
//! We use a special hash function:
//!
//! ```text
//! h(from, sym) = (from * m + sym) mod H
//! ```
//!
//! where `m` is the size of the alphabet, and `H` is the number of slots in
//! the hash table. For a trie with `n` nodes, we take `H = (n-1) + (n-1)/3`.
//! **No rehashing, resizing, or reallocation is required.**
//!
//! The time complexity of transition from one node to another is O(1) for
//! the average case and O(m) for the worst case. The space complexity is
//! O(n), unrelated to `m`.
//!
//! See <https://arxiv.org/abs/2302.03690> for the analysis and proof.

use std::io;

/// A single edge in the trie, stored as `(from, sym) -> to`.
struct Edge {
    from: i32,
    sym: i32,
    to: i32,
}

/// A `ChTrie` instance represents a coordinate hash trie.
///
/// Nodes in the trie are indexed by non-negative integers less than `n`.
/// The root node is indexed by 0. Symbols are non-negative integers less
/// than `m`.
pub struct ChTrie {
    /// Global hash table of edges, bucketed by `h(from, sym)`.
    etab: Vec<Vec<Edge>>,
    /// Pool of available node indexes (a stack of freed indexes).
    idxpool: Vec<i32>,
    /// Number of free indexes currently in the pool.
    idxptr: usize,
    /// Next fresh node index.
    idxmax: i32,
    /// Maximum number of nodes.
    maxn: usize,
    /// Size of the alphabet.
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
    /// The root node is indexed by 0. Symbols are non-negative integers less
    /// than `m`.
    ///
    /// Upon success, return a `ChTrie`. Otherwise, return an error with kind
    /// [`io::ErrorKind::InvalidInput`] (corresponding to `ERANGE` in C).
    pub fn alloc(n: usize, m: usize) -> io::Result<ChTrie> {
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "n or m is too large",
            ));
        }
        // Equivalent to the C check:
        //   MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
        if i32::MAX as usize - (n - 1) < (n - 1) / 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "edge table capacity overflows",
            ));
        }
        let ecap = (n - 1) + (n - 1) / 3;
        Ok(ChTrie {
            etab: vec![Vec::new(); ecap],
            idxpool: vec![0i32; n],
            idxptr: 0,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Walk from one node to its child.
    ///
    /// If the child didn't exist and `creat` is non-zero, a new node will be
    /// created.
    ///
    /// Upon the child is found or created, return `Ok(Some(child))`.
    /// Otherwise, return `Ok(None)`.
    /// If `creat` is non-zero and this routine fails, return an error with
    /// kind [`io::ErrorKind::OutOfMemory`] (corresponding to `ENOMEM` in C).
    pub fn walk(&mut self, from: i32, sym: i32, creat: bool) -> io::Result<Option<i32>> {
        let h = ((from as u64) * (self.alphsz as u64) + sym as u64)
            % (self.ecap as u64) as usize;
        if let Some(e) = self.etab[h].iter().find(|e| e.from == from && e.sym == sym) {
            return Ok(Some(e.to));
        }
        if creat {
            if self.idxptr == 0 && self.idxmax >= self.maxn as i32 {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "node index pool exhausted",
                ));
            }
            let to = if self.idxptr > 0 {
                // Reuse a freed node index (pop from the pool).
                self.idxptr -= 1;
                self.idxpool[self.idxptr]
            } else {
                // Allocate a fresh node index.
                let idx = self.idxmax;
                self.idxmax += 1;
                idx
            };
            self.etab[h].push(Edge { from, sym, to });
            return Ok(Some(to));
        }
        Ok(None)
    }

    /// Delete a child node.
    ///
    /// The child node must be a leaf if it exists, or the behavior is
    /// undefined.
    ///
    /// If the child doesn't exist, the trie shall be left unchanged.
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = ((from as u64) * (self.alphsz as u64) + sym as u64)
            % (self.ecap as u64) as usize;
        if let Some(pos) = self.etab[h].iter().position(|e| e.from == from && e.sym == sym) {
            let e = self.etab[h].remove(pos);
            // Push the freed node index back onto the pool.
            self.idxpool[self.idxptr] = e.to;
            self.idxptr += 1;
        }
    }
}
