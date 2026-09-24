//! CH-Trie: a compact hash trie, translated from the C implementation
//! (`src/chtrie.c` / `src/chtrie.h`).
//!
//! The trie is a hash table of edge lists (separate chaining). Each edge
//! records `(from, sym, to)`. Node indices are allocated from a LIFO free
//! pool (`idxpool`) plus a monotonically increasing counter (`next_idx`),
//! bounded by `maxn`.

use std::fmt;

/// Errors that can arise from CH-Trie operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// `n` or `m` (or the derived edge capacity) exceeds the supported range.
    Range,
    /// The node index space (`maxn`) is exhausted and no index is recycled.
    Capacity,
    /// Memory allocation failed.
    Alloc,
    /// The requested edge does not exist and creation was not requested.
    NotFound,
}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChTrieError::Range => write!(f, "n or m out of supported range"),
            ChTrieError::Capacity => write!(f, "node index space exhausted"),
            ChTrieError::Alloc => write!(f, "memory allocation failed"),
            ChTrieError::NotFound => write!(f, "edge not found"),
        }
    }
}

impl std::error::Error for ChTrieError {}

/// A single edge in the hash table's bucket lists.
///
/// `next` links the edge into the bucket's singly linked list; the list head
/// lives in `ChTrie::etab`.
#[derive(Debug)]
pub struct Edge {
    pub next: Option<Box<Edge>>,
    pub from: usize,
    pub sym: usize,
    pub to: usize,
}

/// A CH-Trie with at most `n` nodes and alphabet size `m`.
///
/// Node index 0 is the root and is never recycled into the free pool.
#[derive(Debug)]
pub struct ChTrie {
    /// Edge table: `ecap` buckets, each the head of a linked edge list.
    pub etab: Vec<Option<Box<Edge>>>,
    /// LIFO pool of recycled node indices.
    pub idxpool: Vec<usize>,
    /// Next fresh node index to allocate (root is 0).
    pub next_idx: usize,
    /// Maximum number of nodes (inclusive bound on indices).
    pub maxn: usize,
    /// Alphabet size.
    pub alphsz: usize,
    /// Number of edge-table buckets.
    pub ecap: usize,
}

impl ChTrie {
    /// Allocate a CH-Trie with at most `n` nodes and alphabet size `m`.
    ///
    /// `n` and `m` are clamped to at least 1. Returns `Err(Range)` if `n` or
    /// `m` exceeds `i32::MAX`, or if the derived edge capacity
    /// `(n-1) + (n-1)/3` exceeds `i32::MAX`.
    pub fn new(n: usize, m: usize) -> Result<Self, ChTrieError> {
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        let ecap = (n - 1) + (n - 1) / 3;
        if ecap > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        // Guard against a zero-sized table (n == 1): the hash function
        // takes a modulo ecap, so keep at least one bucket.
        let ecap = ecap.max(1);
        Ok(ChTrie {
            etab: (0..ecap).map(|_| None).collect(),
            idxpool: Vec::new(),
            next_idx: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    fn bucket(&self, from: usize, sym: usize) -> usize {
        ((from as u64 * self.alphsz as u64 + sym as u64) % self.ecap as u64) as usize
    }

    /// Look up the edge `(from, sym)`.
    ///
    /// If it exists, returns its `to` index. If it does not exist and
    /// `creat` is false, returns `Err(NotFound)`. If it does not exist and
    /// `creat` is true, allocates a new node index (recycling from the free
    /// pool first, then from `next_idx`) and inserts the edge at the head of
    /// its bucket; returns `Err(Capacity)` if the index space is exhausted.
    pub fn walk(&mut self, from: usize, sym: usize, creat: bool) -> Result<usize, ChTrieError> {
        let h = self.bucket(from, sym);
        let mut cur = &mut self.etab[h];
        while let Some(e) = cur {
            if e.from == from && e.sym == sym {
                return Ok(e.to);
            }
            cur = &mut e.next;
        }
        if !creat {
            return Err(ChTrieError::NotFound);
        }
        if self.idxpool.is_empty() && self.next_idx >= self.maxn {
            return Err(ChTrieError::Capacity);
        }
        let to = match self.idxpool.pop() {
            Some(t) => t,
            None => {
                let t = self.next_idx;
                self.next_idx += 1;
                t
            }
        };
        let edge = Edge {
            next: self.etab[h].take(),
            from,
            sym,
            to,
        };
        self.etab[h] = Some(Box::new(edge));
        Ok(to)
    }

    /// Delete the edge `(from, sym)`, recycling its `to` index onto the free
    /// pool. A no-op if the edge does not exist.
    pub fn del(&mut self, from: usize, sym: usize) {
        let h = self.bucket(from, sym);
        // Locate the edge's position in the bucket list (immutable scan).
        let mut pos = None;
        {
            let mut i = 0usize;
            let mut cur = &self.etab[h];
            while let Some(e) = cur {
                if e.from == from && e.sym == sym {
                    pos = Some(i);
                    break;
                }
                cur = &e.next;
                i += 1;
            }
        }
        let Some(pos) = pos else { return }; // no-op if the edge is missing
        // Walk to the slot holding the edge and unlink it.
        let mut slot = &mut self.etab[h];
        for _ in 0..pos {
            slot = &mut slot.as_mut().expect("edge present in bucket list").next;
        }
        let edge = slot.take().expect("edge present in bucket list");
        self.idxpool.push(edge.to);
    }
}

impl Drop for ChTrie {
    fn drop(&mut self) {
        // The Vecs (and the Boxed edge lists they own) free themselves.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 65536;
    const M: usize = 256;

    /// A set of strings stored in a CH-Trie, ported from `tests/test.c`.
    struct StringSet {
        tr: ChTrie,
        term: Vec<bool>,
        nchild: Vec<usize>,
    }

    impl StringSet {
        fn new() -> Self {
            StringSet {
                tr: ChTrie::new(N, M).expect("StringSet::new"),
                term: vec![false; N],
                nchild: vec![0; N],
            }
        }

        /// Insert `s`. Probes each edge with `creat=false` first so that
        /// `nchild` is only bumped for genuinely new edges.
        fn add(&mut self, s: &str) {
            let mut from = 0usize;
            for c in s.bytes() {
                let sym = c as usize;
                match self.tr.walk(from, sym, false) {
                    Ok(to) => from = to,
                    Err(ChTrieError::NotFound) => {
                        let to = self
                            .tr
                            .walk(from, sym, true)
                            .expect("StringSet::add capacity");
                        self.nchild[from] += 1;
                        from = to;
                    }
                    Err(e) => panic!("StringSet::add: {e}"),
                }
            }
            self.term[from] = true;
        }

        /// Remove `s`. Traces the path, unsets the terminal flag, then prunes
        /// the chain of non-terminal nodes with zero children.
        fn del(&mut self, s: &str) {
            let mut path: Vec<(usize, usize)> = Vec::new();
            let mut from = 0usize;
            for c in s.bytes() {
                let sym = c as usize;
                match self.tr.walk(from, sym, false) {
                    Ok(to) => {
                        path.push((from, sym));
                        from = to;
                    }
                    Err(_) => return, // path missing: nothing to delete
                }
            }
            if !self.term[from] {
                return;
            }
            self.term[from] = false;
            // Prune non-terminal zero-child chain from the leaf upward.
            let mut cur = from;
            while cur != 0 && self.nchild[cur] == 0 {
                let (parent, sym) = path.pop().expect("prune path");
                self.nchild[parent] -= 1;
                self.tr.del(parent, sym);
                cur = parent;
            }
        }

        /// Query whether `s` is in the set.
        fn query(&mut self, s: &str) -> bool {
            let mut from = 0usize;
            for c in s.bytes() {
                match self.tr.walk(from, c as usize, false) {
                    Ok(to) => from = to,
                    Err(_) => return false,
                }
            }
            self.term[from]
        }
    }

    #[test]
    fn test_queries() {
        let mut ss = StringSet::new();

        let dict1 = ["", "the", "a", "an"];
        for w in dict1 {
            ss.add(w);
        }
        let dict2 = ["he", "she", "his", "hers"];
        for w in dict2 {
            ss.add(w);
        }
        let stop = ["the", "an", "a"];
        for w in stop {
            ss.del(w);
        }
        let dict3 = ["this", "that"];
        for w in dict3 {
            ss.add(w);
        }

        let expected: &[(&str, bool)] = &[
            ("hello", false),
            ("the", false),
            ("his", true),
            ("he", true),
            ("his", true),
            ("go", false),
            ("he", true),
            ("a", false),
            ("an", false),
            ("this", true),
            ("that", true),
            ("hey", false),
            ("she", true),
            ("hers", true),
        ];
        for (word, want) in expected {
            let got = ss.query(word);
            assert_eq!(got, *want, "query({word:?}) = {got}, want {want}");
        }
    }

    #[test]
    fn test_new_clamping() {
        // C-faithful: chtrie_new clamps n and m to at least 1, so new(0,0)
        // succeeds with maxn=1 (root node only, no allocatable nodes).
        let mut tr = ChTrie::new(0, 0).expect("new(0,0) clamps to (1,1)");
        // With n clamped to 1 the index space is exhausted after the root,
        // so chtrie_walk(tr, 0, sym, 1) hits idxptr==idxpool && idxmax>=maxn
        // and returns -1 with errno=ENOMEM. The Rust analog is Err(Capacity).
        assert_eq!(tr.walk(0, 0, true), Err(ChTrieError::Capacity));

        // Clamping of a single zero argument still succeeds.
        assert!(ChTrie::new(0, 5).is_ok(), "new(0,5) clamps n to 1");
        assert!(ChTrie::new(5, 0).is_ok(), "new(5,0) clamps m to 1");
    }

    #[test]
    fn test_new_overflow() {
        assert!(matches!(
            ChTrie::new(i32::MAX as usize + 1, 1),
            Err(ChTrieError::Range)
        ));
        assert!(matches!(
            ChTrie::new(1, i32::MAX as usize + 1),
            Err(ChTrieError::Range)
        ));
    }

    #[test]
    fn test_walk_miss() {
        let mut tr = ChTrie::new(10, 10).expect("new(10,10)");
        assert_eq!(tr.walk(0, 3, false), Err(ChTrieError::NotFound));
    }

    #[test]
    fn test_del_noop() {
        let mut tr = ChTrie::new(10, 10).expect("new(10,10)");
        tr.del(0, 3); // must not panic
        assert_eq!(tr.walk(0, 3, false), Err(ChTrieError::NotFound));
    }

    #[test]
    fn test_index_reuse_lifo() {
        let mut tr = ChTrie::new(10, 10).expect("new(10,10)");
        let a = tr.walk(0, 0, true).expect("create a");
        let b = tr.walk(0, 1, true).expect("create b");
        tr.del(0, 0); // pushes a
        tr.del(0, 1); // pushes b; pool is now [a, b]
        let c = tr.walk(0, 2, true).expect("create c");
        assert_eq!(c, b, "LIFO: b was pushed last, so b is popped first");
        let d = tr.walk(0, 3, true).expect("create d");
        assert_eq!(d, a, "LIFO: a was pushed first, so a is popped second");
    }

    #[test]
    fn test_capacity() {
        let mut tr = ChTrie::new(2, 10).expect("new(2,10)");
        assert!(tr.walk(0, 0, true).is_ok(), "first node fits");
        assert_eq!(tr.walk(0, 1, true), Err(ChTrieError::Capacity));
    }
}
