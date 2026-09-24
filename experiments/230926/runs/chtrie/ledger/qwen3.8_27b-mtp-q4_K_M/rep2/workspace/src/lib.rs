//! CH-Trie: a coordinate hash trie.
//!
//! Rust translation of the C CH-Trie library (`chtrie.c` / `chtrie.h`).
//! The trie maps sequences of symbols (coordinates) to node indices,
//! supporting walk (lookup/insert) and delete operations.

/// A coordinate hash trie.
pub struct ChTrie {
    /// Edge table: one inner `Vec` of edges per node.
    etab: Vec<Vec<Edge>>,
    /// Pool of free node indices.
    idxpool: Vec<usize>,
    /// Cursor into the index pool.
    idxptr: usize,
    /// Maximum node index allocated so far.
    idxmax: usize,
    /// Maximum number of nodes.
    maxn: usize,
    /// Alphabet size.
    alphsz: usize,
    /// Edge table capacity.
    ecap: usize,
}

/// A single trie edge.
struct Edge {
    /// Source node index.
    from: usize,
    /// Symbol (coordinate) on the edge.
    sym: usize,
    /// Destination node index.
    to: usize,
}

/// Errors returned by trie operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChTrieError {
    /// The requested path or node was not found.
    NotFound,
    /// A symbol or index was out of range.
    Range,
    /// The trie is at capacity.
    Capacity,
}

impl std::fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChTrieError::NotFound => write!(f, "not found"),
            ChTrieError::Range => write!(f, "out of range"),
            ChTrieError::Capacity => write!(f, "capacity exceeded"),
        }
    }
}

impl std::error::Error for ChTrieError {}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    ///
    /// Translation of `chtrie_alloc`.
    pub fn new(n: usize, m: usize) -> Result<Self, ChTrieError> {
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        // Overflow check: ecap = (n-1) + (n-1)/3 must fit in usize.
        // (C: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3)
        if (n - 1) > (usize::MAX - (n - 1) / 3) {
            return Err(ChTrieError::Range);
        }
        let ecap = ((n - 1) + (n - 1) / 3).max(1);
        Ok(ChTrie {
            etab: (0..ecap).map(|_| Vec::new()).collect(),
            idxpool: (1..n).collect(),
            idxptr: n - 1,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Walk from one node to its child.
    ///
    /// If `creat` is true and the edge does not exist, a new edge is
    /// created and a new node index is allocated (recycled from the free
    /// pool if available).
    ///
    /// Translation of `chtrie_walk`.
    pub fn walk(&mut self, from: usize, sym: usize, creat: bool) -> Result<usize, ChTrieError> {
        let h = ((from as u64 * self.alphsz as u64 + sym as u64) % self.ecap as u64) as usize;
        if let Some(pos) = self.etab[h].iter().position(|e| e.from == from && e.sym == sym) {
            return Ok(self.etab[h][pos].to);
        }
        if !creat {
            return Err(ChTrieError::NotFound);
        }
        if self.idxptr == 0 && self.idxmax >= self.maxn {
            return Err(ChTrieError::Capacity);
        }
        let to = if self.idxptr > 0 {
            let t = self.idxpool[self.idxptr - 1];
            self.idxptr -= 1;
            t
        } else {
            let t = self.idxmax;
            self.idxmax += 1;
            t
        };
        // C prepends the new edge to the bucket's linked list.
        self.etab[h].insert(0, Edge { from, sym, to });
        Ok(to)
    }

    /// Delete a child node.
    ///
    /// Removes the edge `(from, sym)` from the edge table and recycles
    /// the destination node index back into the free pool.
    ///
    /// Translation of `chtrie_del`.
    pub fn del(&mut self, from: usize, sym: usize) {
        let h = ((from as u64 * self.alphsz as u64 + sym as u64) % self.ecap as u64) as usize;
        if let Some(pos) = self.etab[h].iter().position(|e| e.from == from && e.sym == sym) {
            let edge = self.etab[h].remove(pos);
            self.idxpool[self.idxptr] = edge.to;
            self.idxptr += 1;
        }
    }
}

impl Drop for ChTrie {
    /// Translation of `chtrie_free`: nothing to do; the `Vec`s free
    /// themselves. Kept for parity with the C API.
    fn drop(&mut self) {}
}
