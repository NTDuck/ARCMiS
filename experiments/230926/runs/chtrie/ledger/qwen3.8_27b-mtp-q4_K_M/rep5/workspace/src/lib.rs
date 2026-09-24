//! Rust port of the C CH-Trie library (`chtrie.c` / `chtrie.h`).
//!
//! A CH-Trie is a compact hash-based trie used to store a set of
//! (from, sym) -> to transitions. This port keeps the exact C semantics:
//! same hash formula, same prepend-on-insert edge lists, same index-pool
//! stack behavior, and the same `ERANGE` / `ENOMEM` failure modes (returned
//! as `Result` errors instead of `errno`).
//!
//! No `unsafe` code is used.

use std::fmt;

/// Errors that can be returned by [`ChTrie::new`].
///
/// Mirrors the C library's `errno` values:
/// - [`ChTrieError::Erange`]  — `errno = ERANGE` (size parameters too large)
/// - [`ChTrieError::Enomem`]  — `errno = ENOMEM` (allocation failure)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// The requested size parameters are out of range (C `ERANGE`).
    Erange,
    /// Out of memory (C `ENOMEM`).
    Enomem,
}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChTrieError::Erange => write!(f, "argument out of domain (ERANGE)"),
            ChTrieError::Enomem => write!(f, "out of memory (ENOMEM)"),
        }
    }
}

impl std::error::Error for ChTrieError {}

/// A single transition edge in the hash table.
///
/// In C, edges are linked-list nodes (`struct chtrie_edge`) chained per
/// hash slot. In Rust each hash slot is a `Vec<Edge>`; the list order is
/// preserved (new edges are prepended, exactly as in C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// Source node index.
    pub from: i32,
    /// Symbol (alphabet index).
    pub sym: i32,
    /// Destination node index.
    pub to: i32,
}

/// A CH-Trie: a hash table of (from, sym) -> to transitions with a pool of
/// reusable destination indices.
#[derive(Debug)]
pub struct ChTrie {
    /// Hash table: `etab[h]` is the (prepended) list of edges hashing to `h`.
    etab: Vec<Vec<Edge>>,
    /// Pool of reusable destination indices, used as a stack at `idxptr`.
    idxpool: Vec<i32>,
    /// Stack pointer into `idxpool` (number of free indices currently pooled).
    idxptr: usize,
    /// Next fresh destination index to allocate.
    idxmax: i32,
    /// Maximum number of nodes (`n` passed to [`ChTrie::new`]).
    maxn: i32,
    /// Alphabet size (`m` passed to [`ChTrie::new`]).
    alphsz: i32,
    /// Number of hash table slots.
    ecap: usize,
}

impl ChTrie {
    /// Allocate a new CH-Trie for up to `n` nodes and an alphabet of size `m`.
    ///
    /// Mirrors `chtrie_alloc(n, m)`:
    /// - `n` and `m` are regulated to at least 1;
    /// - if `n > INT_MAX || m > INT_MAX`, or
    ///   `MIN(INT_MAX, SIZE_MAX) - (n-1) < (n-1)/3`, returns
    ///   [`ChTrieError::Erange`];
    /// - `ecap = (n-1) + (n-1)/3` hash slots are created;
    /// - the index pool is initialized with `n` slots, `idxmax = 1`,
    ///   `idxptr = 0`.
    pub fn new(n: i32, m: i32) -> Result<ChTrie, ChTrieError> {
        let n = if n < 1 { 1 } else { n };
        let m = if m < 1 { 1 } else { m };

        // C: if (n > INT_MAX || m > INT_MAX) -> ERANGE.
        // With i32 parameters this is automatic, but keep the check explicit.
        if n > i32::MAX || m > i32::MAX {
            return Err(ChTrieError::Erange);
        }

        // C: if (MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3) -> ERANGE.
        // MIN(INT_MAX, SIZE_MAX) == INT_MAX for i32 params; use i64 to avoid
        // overflow in the subtraction.
        let n1 = (n - 1) as i64;
        let min_int_max_sz_max = i32::MAX as i64; // MIN(INT_MAX, SIZE_MAX)
        if min_int_max_sz_max - n1 < n1 / 3 {
            return Err(ChTrieError::Erange);
        }

        // C: ecap = (n-1) + (n-1)/3. For n == 1 this is 0, which makes the
        // C hash (`% ecap`) divide by zero (UB). We clamp to 1 so that a
        // 1-node trie is usable; for n >= 2 the value is unchanged.
        let ecap = (((n - 1) + (n - 1) / 3).max(1)) as usize;
        let etab = vec![Vec::new(); ecap];
        let idxpool = vec![0; n as usize];

        Ok(ChTrie {
            etab,
            idxpool,
            idxptr: 0,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Look up the transition `(from, sym)`; if not found and `creat` is
    /// true, create it, allocating a fresh destination index (reusing a
    /// pooled one if available).
    ///
    /// Mirrors `chtrie_walk(tr, from, sym, creat)`:
    /// - returns the destination index `to` if the edge exists (or was
    ///   created);
    /// - returns `-1` if the edge does not exist and `creat` is false;
    /// - returns `-1` (C `errno = ENOMEM`) if creation is requested but the
    ///   index pool is exhausted (`idxptr == 0 && idxmax >= maxn`).
    pub fn walk(&mut self, from: i32, sym: i32, creat: bool) -> i32 {
        let h = self.hash(from, sym);

        // Search the slot's edge list for a matching (from, sym).
        if let Some(edge) = self.etab[h].iter().find(|e| e.from == from && e.sym == sym) {
            return edge.to;
        }

        if creat {
            // C: if (tr->idxptr == tr->idxpool && tr->idxmax >= tr->maxn)
            //      { errno = ENOMEM; return -1; }
            if self.idxptr == 0 && self.idxmax >= self.maxn {
                // (ENOMEM; reported as -1 exactly like the C function.)
                return -1;
            }

            // C prepends the new edge to the head of the slot's list.
            let to = if self.idxptr > 0 {
                // Reuse a pooled index: p->to = *--tr->idxptr;
                self.idxptr -= 1;
                self.idxpool[self.idxptr]
            } else {
                // Allocate a fresh index: p->to = tr->idxmax++;
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };

            self.etab[h].insert(0, Edge { from, sym, to });
            to
        } else {
            -1
        }
    }

    /// Delete the transition `(from, sym)` if present, returning its
    /// destination index to the pool. Does nothing if the edge is absent.
    ///
    /// Mirrors `chtrie_del(tr, from, sym)`.
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = self.hash(from, sym);
        let list = &mut self.etab[h];
        if let Some(pos) = list.iter().position(|e| e.from == from && e.sym == sym) {
            let edge = list.remove(pos);
            // C: *tr->idxptr++ = p->to;  (push onto the pool stack)
            self.idxpool[self.idxptr] = edge.to;
            self.idxptr += 1;
        }
    }

    /// Hash formula, exactly as in C:
    /// `h = ((unsigned long)from * alphsz + sym) % ecap`.
    ///
    /// C unsigned arithmetic wraps modulo 2^64; `wrapping_mul` /
    /// `wrapping_add` on `usize` reproduce that for negative `from`/`sym`.
    fn hash(&self, from: i32, sym: i32) -> usize {
        let h = (from as usize)
            .wrapping_mul(self.alphsz as usize)
            .wrapping_add(sym as usize);
        h % self.ecap
    }
}

/// API parity with `chtrie_free`; `Vec`s release their memory automatically.
impl Drop for ChTrie {
    fn drop(&mut self) {
        // No-op: owned Vecs free their backing memory on drop.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a) `new(0, 0)` regulates n and m to 1 and the trie works:
    /// with maxn = 1 only the root (index 0) exists, so lookups miss and
    /// creation fails (pool exhausted, C `ENOMEM` -> -1).
    #[test]
    fn new_regulates_zero_to_one() {
        let mut tr = ChTrie::new(0, 0).expect("new(0,0) must succeed");
        assert_eq!(tr.walk(0, 5, false), -1, "miss on empty trie");
        assert_eq!(tr.walk(0, 5, true), -1, "creation fails: maxn = 1");
        // del of a nonexistent edge is still a no-op.
        tr.del(0, 5);
        assert_eq!(tr.walk(0, 5, false), -1);
    }

    /// (b) `new` with a huge n returns `Err(ChTrieError::Erange)`.
    #[test]
    fn new_huge_n_returns_erange() {
        assert!(
            matches!(ChTrie::new(i32::MAX, 256), Err(ChTrieError::Erange)),
            "new(i32::MAX, 256) must fail with Erange"
        );
    }

    /// (c) `del` of a nonexistent edge is a no-op (no panic, no state change).
    #[test]
    fn del_nonexistent_edge_is_noop() {
        let mut tr = ChTrie::new(16, 256).expect("new(16, 256)");
        tr.del(0, 99);
        assert_eq!(tr.walk(0, 99, false), -1, "edge still absent");
        assert_eq!(tr.walk(0, 99, true), 1, "creation still allocates index 1");
    }

    /// (d) Pool exhaustion with `new(4, 256)`: root (0) plus 3 children
    /// (1, 2, 3) fill the pool; a 4th creation returns -1; after deleting
    /// one child, the next creation reuses that index (LIFO reuse).
    #[test]
    fn pool_exhaustion_and_lifo_reuse() {
        let mut tr = ChTrie::new(4, 256).expect("new(4, 256)");
        assert_eq!(tr.walk(0, 1, true), 1, "first child");
        assert_eq!(tr.walk(0, 2, true), 2, "second child");
        assert_eq!(tr.walk(0, 3, true), 3, "third child");
        assert_eq!(tr.walk(0, 4, true), -1, "pool exhausted -> -1");
        // Delete the middle child; its index goes onto the pool stack.
        tr.del(0, 2);
        assert_eq!(tr.walk(0, 4, true), 2, "LIFO reuse of deleted index");
        // Pool is empty again and idxmax is back at maxn: exhausted once more.
        assert_eq!(tr.walk(0, 5, true), -1, "pool exhausted again");
    }

    /// (e) `walk` hit/miss semantics: miss without creat -> -1; creat
    /// allocates; subsequent walks (creat or not) hit the same index;
    /// distinct (from, sym) pairs get distinct indices.
    #[test]
    fn walk_hit_miss_semantics() {
        let mut tr = ChTrie::new(16, 256).expect("new(16, 256)");
        assert_eq!(tr.walk(0, 7, false), -1, "miss, creat=false");
        assert_eq!(tr.walk(0, 7, true), 1, "create allocates index 1");
        assert_eq!(tr.walk(0, 7, false), 1, "hit after create");
        assert_eq!(tr.walk(0, 7, true), 1, "hit again, no new index");
        assert_eq!(tr.walk(1, 7, false), -1, "different from -> miss");
        assert_eq!(tr.walk(0, 8, true), 2, "different sym -> new index");
        assert_eq!(tr.walk(0, 7, false), 1, "original edge intact");
    }
}
