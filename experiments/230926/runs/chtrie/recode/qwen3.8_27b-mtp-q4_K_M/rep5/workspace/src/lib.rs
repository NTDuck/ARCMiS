//! CH-Trie: a coordinate hash trie.
//!
//! All edges live in one global hash table indexed by
//! `h(from, sym) = (from * alphsz + sym) % ecap`. The table is fixed-size
//! (no rehashing) and a stack-based pool recycles freed node indices.

/// Error type for `ChTrie` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChTrieError {
    /// The requested size exceeds the maximum representable value.
    Range,
    /// The trie has no more available node indices.
    Capacity,
}

impl std::fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChTrieError::Range => write!(f, "requested size out of range"),
            ChTrieError::Capacity => write!(f, "no more available node indices"),
        }
    }
}

impl std::error::Error for ChTrieError {}

/// A single edge in the trie: `from --sym--> to`.
///
/// Collision chains are stored as short vectors in `ChTrie::etab`.
#[derive(Clone, Debug)]
struct Edge {
    from: usize,
    sym: usize,
    to: usize,
}

/// A coordinate hash trie.
///
/// Nodes are indexed by non-negative integers less than `maxn`; the root is
/// node 0. Symbols are non-negative integers less than `alphsz`.
#[derive(Debug)]
pub struct ChTrie {
    /// Hash table: each slot is a short collision chain of edges.
    etab: Vec<Vec<Edge>>,
    /// Pool of recycled node indices (stack).
    idxpool: Vec<usize>,
    /// Number of indices currently pushed onto the pool (top pointer).
    idxptr: usize,
    /// Next fresh index to allocate.
    idxmax: usize,
    /// Maximum number of nodes.
    maxn: usize,
    /// Alphabet size.
    alphsz: usize,
    /// Hash table capacity.
    ecap: usize,
}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    ///
    /// If `n` or `m` is 0, they are regulated to 1.
    pub fn new(mut n: usize, mut m: usize) -> Result<Self, ChTrieError> {
        if n < 1 {
            n = 1;
        }
        if m < 1 {
            m = 1;
        }
        // Mirror the C `n > INT_MAX || m > INT_MAX` range check.
        let int_max = isize::MAX as usize;
        if n > int_max || m > int_max {
            return Err(ChTrieError::Range);
        }
        // Overflow-check `ecap = (n-1) + (n-1)/3`.
        let ecap = (n - 1)
            .checked_add((n - 1) / 3)
            .ok_or(ChTrieError::Range)?;

        Ok(ChTrie {
            etab: vec![Vec::new(); ecap],
            idxpool: vec![0; n],
            idxptr: 0,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Hash slot for edge `(from, sym)`.
    ///
    /// Mirrors the C `unsigned long` wrap-then-mod semantics.
    fn slot(&self, from: usize, sym: usize) -> usize {
        (from.wrapping_mul(self.alphsz).wrapping_add(sym)) % self.ecap
    }

    /// Walk from node `from` via symbol `sym`.
    ///
    /// If `create` is true and the edge does not exist, a new node is created.
    /// Returns the child node index, or `None` if not found and `create` is
    /// false (or if creation fails due to capacity).
    pub fn walk(&mut self, from: usize, sym: usize, create: bool) -> Option<usize> {
        let h = self.slot(from, sym);
        if let Some(edge) = self.etab[h].iter().find(|e| e.from == from && e.sym == sym) {
            return Some(edge.to);
        }
        if !create {
            return None;
        }
        // Capacity check: no recycled index available and the fresh pool is
        // exhausted.
        if self.idxptr == 0 && self.idxmax >= self.maxn {
            return None;
        }
        let to = if self.idxptr > 0 {
            let to = self.idxpool[self.idxptr - 1];
            self.idxptr -= 1;
            to
        } else {
            let to = self.idxmax;
            self.idxmax += 1;
            to
        };
        self.etab[h].insert(0, Edge { from, sym, to });
        Some(to)
    }

    /// Delete the edge from `from` via `sym`.
    ///
    /// The child must be a leaf (caller's responsibility). If the edge does
    /// not exist, the trie is left unchanged.
    pub fn delete(&mut self, from: usize, sym: usize) {
        let h = self.slot(from, sym);
        let pos = self
            .etab[h]
            .iter()
            .position(|e| e.from == from && e.sym == sym);
        let Some(pos) = pos else {
            return;
        };
        let edge = self.etab[h].remove(pos);
        self.idxpool[self.idxptr] = edge.to;
        self.idxptr += 1;
    }
}

impl Drop for ChTrie {
    fn drop(&mut self) {
        // `Vec` handles cleanup automatically; no manual free needed.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_regulates_zero() {
        // Zero sizes are regulated to 1.
        let tr = ChTrie::new(0, 0).expect("new(0, 0) should succeed");
        assert_eq!(tr.maxn, 1);
        assert_eq!(tr.alphsz, 1);
        // A huge `n` overflows the ecap computation and is rejected.
        assert!(matches!(
            ChTrie::new(usize::MAX, 1),
            Err(ChTrieError::Range)
        ));
    }

    #[test]
    fn walk_create_and_lookup() {
        let mut tr = ChTrie::new(16, 4).unwrap();
        // No edge yet: lookup without create fails.
        assert_eq!(tr.walk(0, 1, false), None);
        // Create the edge.
        let to = tr.walk(0, 1, true).expect("create should succeed");
        assert_eq!(to, 1);
        // Now the lookup succeeds and returns the same child.
        assert_eq!(tr.walk(0, 1, false), Some(to));
        // A different symbol has no edge.
        assert_eq!(tr.walk(0, 2, false), None);
    }

    #[test]
    fn delete_recycles_index() {
        let mut tr = ChTrie::new(16, 4).unwrap();
        let to = tr.walk(0, 1, true).unwrap();
        // Deleting a non-existent edge is a no-op.
        tr.delete(0, 2);
        assert_eq!(tr.walk(0, 1, false), Some(to));
        // Delete the edge; the child index goes back onto the pool.
        tr.delete(0, 1);
        assert_eq!(tr.walk(0, 1, false), None);
        // Creating again recycles the same index.
        let to2 = tr.walk(0, 1, true).unwrap();
        assert_eq!(to2, to);
    }
}
