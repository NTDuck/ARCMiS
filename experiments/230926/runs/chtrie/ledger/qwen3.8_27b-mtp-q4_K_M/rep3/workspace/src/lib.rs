//! CH-Trie (compressed hash trie) — idiomatic Rust port of the C library
//! (`src/chtrie.c` / `src/chtrie.h`).
//!
//! The trie is a global hash table of edges keyed by the pair
//! `(from, sym)`, hashed by `h = (from * alphsz + sym) % ecap` with
//! `ecap = (n-1) + (n-1)/3`. Nodes are bare non-negative indices; the root
//! is index 0. A free list of recycled node indices (the C `idxpool` /
//! `idxptr` / `idxmax` machinery) is preserved exactly: recycled indices
//! are popped before new ones are allocated, and exhaustion is reported as
//! [`ChTrieError::Capacity`] (C: `errno = ENOMEM`).
//!
//! # Error mapping
//!
//! | C | Rust |
//! |---|------|
//! | `NULL` + `errno = ERANGE` (alloc size/overflow guard) | `Err(ChTrieError::Range)` |
//! | `-1` + `errno = ENOMEM` (node-index pool exhausted) | `Err(ChTrieError::Capacity)` |
//! | `-1` without errno (child not found) | `Ok(None)` |

use std::error::Error;
use std::fmt;

/// The root node index (C: node 0).
pub const ROOT: i32 = 0;

/// Errors returned by the CH-Trie API, replacing the C `errno` convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// `chtrie_alloc` size/overflow guard (C: `errno = ERANGE`): `n` or `m`
    /// exceeds `i32::MAX`, or `ecap = (n-1) + (n-1)/3` would overflow.
    Range,
    /// `chtrie_walk` with `creat` when the node-index pool is exhausted
    /// (C: `errno = ENOMEM`).
    Capacity,
}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChTrieError::Range => write!(f, "size out of range (ERANGE)"),
            ChTrieError::Capacity => write!(f, "node-index pool exhausted (ENOMEM)"),
        }
    }
}

impl Error for ChTrieError {}

/// One edge in the global edge table: `from --sym--> to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Edge {
    from: i32,
    sym: i32,
    to: i32,
}

/// A CH-Trie with at most `n` nodes and alphabet size `m`.
///
/// Owns the edge table and the node-index free list; everything is freed
/// automatically on drop (C: `chtrie_free`).
#[derive(Debug, PartialEq)]
pub struct ChTrie {
    /// Global edge table: `ecap` buckets, each a list of edges in
    /// head-insertion order (C: `etab` array of linked lists).
    etab: Vec<Vec<Edge>>,
    /// Free list of recycled node indices (C: `idxpool` / `idxptr`).
    /// `idxpool.len()` plays the role of the C `idxptr - idxpool` offset.
    idxpool: Vec<i32>,
    /// Next fresh node index (C: `idxmax`); starts at 1 (root is 0).
    idxmax: i32,
    /// Maximum number of nodes (C: `maxn`).
    maxn: i32,
    /// Alphabet size (C: `alphsz`).
    alphsz: i32,
    /// Edge-table capacity (C: `ecap`); 0 when `n == 1`.
    ecap: i32,
}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    ///
    /// `n` and `m` are clamped to at least 1 (C: `if (n < 1) n = 1;`).
    /// Returns [`ChTrieError::Range`] when `n` or `m` exceeds `i32::MAX`,
    /// or when the C overflow guard `INT_MAX - (n-1) < (n-1)/3` fires.
    /// Size validation, separated from allocation so the guard boundary
    /// can be tested without actually allocating (a passing `n` near the
    /// guard would require a ~51 GB edge table).
    ///
    /// Mirrors the C checks in `chtrie_alloc` exactly:
    /// 1. clamp `n`/`m` to at least 1,
    /// 2. `n > INT_MAX || m > INT_MAX` -> `Range`,
    /// 3. `MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3` -> `Range`.
    ///
    /// Returns the validated `(n, m, ecap)` with `ecap = (n-1) + (n-1)/3`.
    fn validate(n: usize, m: usize) -> Result<(i32, i32, i32), ChTrieError> {
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        let n = n as i32;
        let m = m as i32;
        // C: if (MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3) errno = ERANGE;
        // (SZ_MAX is SIZE_MAX, which is > INT_MAX on every supported
        // platform, so MIN(INT_MAX, SZ_MAX) == INT_MAX == i32::MAX.)
        if i32::MAX - (n - 1) < (n - 1) / 3 {
            return Err(ChTrieError::Range);
        }
        let ecap = (n - 1) + (n - 1) / 3;
        Ok((n, m, ecap))
    }

    pub fn alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError> {
        let (n, m, ecap) = Self::validate(n, m)?;
        // C allocates `ecap` buckets; when n == 1, ecap == 0 and the C
        // `h %= ecap` is undefined. We keep one (empty) bucket so the
        // degenerate trie is usable instead of panicking.
        let etab = vec![Vec::new(); ecap.max(1) as usize];
        Ok(ChTrie {
            etab,
            idxpool: Vec::with_capacity(n as usize),
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Bucket index for `(from, sym)`, mirroring the C unsigned-long
    /// arithmetic `h = (unsigned long)from * alphsz + sym; h %= ecap;`.
    fn bucket(&self, from: i32, sym: i32) -> usize {
        if self.ecap == 0 {
            return 0;
        }
        let h = (from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64)
            % (self.ecap as u64);
        h as usize
    }

    /// Walk from node `from` to its child on symbol `sym`.
    ///
    /// Returns `Ok(Some(child))` when the child is found or (with
    /// `creat == true`) created, `Ok(None)` when it does not exist and
    /// `creat` is false (C: `-1` without errno), and
    /// `Err(ChTrieError::Capacity)` when creation is requested but the
    /// node-index pool is exhausted (C: `-1` with `errno = ENOMEM`).
    // Faithful port of the C signature `chtrie_walk(tr, from, sym, creat)`.
    #[allow(clippy::too_many_arguments)]
    pub fn walk(
        &mut self,
        from: i32,
        sym: i32,
        creat: bool,
    ) -> Result<Option<i32>, ChTrieError> {
        let h = self.bucket(from, sym);
        if let Some(pos) = self.etab[h].iter().position(|e| e.from == from && e.sym == sym) {
            return Ok(Some(self.etab[h][pos].to));
        }
        if !creat {
            return Ok(None);
        }
        // C: if (idxptr == idxpool && idxmax >= maxn) { errno = ENOMEM; ... }
        if self.idxpool.is_empty() && self.idxmax >= self.maxn {
            return Err(ChTrieError::Capacity);
        }
        // C: p->to = *--idxptr (recycled) or idxmax++ (fresh).
        let to = if let Some(recycled) = self.idxpool.pop() {
            recycled
        } else {
            let t = self.idxmax;
            self.idxmax += 1;
            t
        };
        // C: head insertion (p->next = etab[h]; etab[h] = p).
        self.etab[h].insert(0, Edge { from, sym, to });
        Ok(Some(to))
    }

    /// Delete the child edge `(from, sym)` and recycle the child index
    /// into the free list (C: `*idxptr++ = p->to`). No-op if absent.
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = self.bucket(from, sym);
        if let Some(pos) = self.etab[h].iter().position(|e| e.from == from && e.sym == sym) {
            let edge = self.etab[h].remove(pos);
            self.idxpool.push(edge.to);
        }
    }
}

// C: chtrie_free — the owned Vecs are released automatically.
impl Drop for ChTrie {
    fn drop(&mut self) {
        // Everything is owned by Vecs; nothing extra to do.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_clamps_to_one() {
        let a = ChTrie::alloc(0, 0).unwrap();
        let b = ChTrie::alloc(1, 1).unwrap();
        assert_eq!(a.maxn, b.maxn);
        assert_eq!(a.alphsz, b.alphsz);
        assert_eq!(a.ecap, b.ecap);
        assert_eq!(a.idxmax, 1);
        assert!(a.idxpool.is_empty());
    }

    #[test]
    fn alloc_range_errors() {
        // n > INT_MAX
        assert_eq!(
            ChTrie::alloc(i32::MAX as usize + 1, 1),
            Err(ChTrieError::Range)
        );
        // m > INT_MAX
        assert_eq!(
            ChTrie::alloc(1, i32::MAX as usize + 1),
            Err(ChTrieError::Range)
        );
        // Overflow guard: INT_MAX - (n-1) < (n-1)/3 fires at
        // n-1 = 1610612736 (INT_MAX * 3 / 4).
        assert_eq!(
            ChTrie::alloc(1_610_612_737, 1),
            Err(ChTrieError::Range)
        );
        // Just below the guard: n-1 = 1610612735 passes validation.
        // Exercise the validation path only — actually allocating this
        // trie would need ecap = 2147483646 buckets (~51 GB).
        assert_eq!(
            ChTrie::validate(1_610_612_736, 1),
            Ok((1_610_612_736, 1, 2_147_483_646))
        );
        // And a small allocation that passes the same guard succeeds.
        let small = ChTrie::alloc(1024, 256).unwrap();
        assert_eq!(small.maxn, 1024);
        assert_eq!(small.alphsz, 256);
        assert_eq!(small.ecap, 1023 + 1023 / 3);
    }

    #[test]
    fn walk_capacity_on_pool_exhaustion() {
        // maxn = 2: root (0) plus at most one child (1).
        let mut tr = ChTrie::alloc(2, 1).unwrap();
        assert_eq!(tr.walk(ROOT, 0, true).unwrap(), Some(1));
        assert_eq!(tr.walk(ROOT, 1, true), Err(ChTrieError::Capacity));
        // Non-creating walk never errors.
        assert_eq!(tr.walk(ROOT, 1, false).unwrap(), None);
    }

    #[test]
    fn del_recycles_index() {
        let mut tr = ChTrie::alloc(2, 1).unwrap();
        assert_eq!(tr.walk(ROOT, 0, true).unwrap(), Some(1));
        tr.del(ROOT, 0);
        // Recycled index 1 is handed out again before any fresh one.
        assert_eq!(tr.walk(ROOT, 0, true).unwrap(), Some(1));
        // Deleting an absent edge is a no-op.
        tr.del(ROOT, 0);
        tr.del(ROOT, 0);
        assert_eq!(tr.walk(ROOT, 0, true).unwrap(), Some(1));
    }

    #[test]
    fn walk_finds_existing_edge() {
        let mut tr = ChTrie::alloc(4, 256).unwrap();
        let a = tr.walk(ROOT, 'a' as i32, true).unwrap().unwrap();
        let b = tr.walk(ROOT, 'a' as i32, false).unwrap().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn degenerate_single_node_trie() {
        // n == 1: ecap == 0; only the root exists, no child can be made.
        let mut tr = ChTrie::alloc(1, 1).unwrap();
        assert_eq!(tr.walk(ROOT, 0, false).unwrap(), None);
        assert_eq!(tr.walk(ROOT, 0, true), Err(ChTrieError::Capacity));
    }
}
