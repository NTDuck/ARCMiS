//! Translation of `src/chtrie.c` / `src/chtrie.h`.
//!
//! Mapping of the C API:
//! - `chtrie_alloc(n, m)`            → [`ChTrie::alloc`]
//! - `chtrie_walk(tr, from, sym, cr)`→ [`ChTrie::walk`]
//! - `chtrie_del(tr, from, sym)`     → [`ChTrie::del`]
//! - `chtrie_free(tr)`               → `Drop` (automatic)

use std::error::Error;
use std::fmt;

/// Errors corresponding to the C library's `errno` signaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    /// `chtrie_alloc` was given `n`/`m` beyond `i32::MAX`, or
    /// `ecap = (n-1) + (n-1)/3` would overflow `i32` (C `ERANGE`).
    Range,
    /// `walk` with `creat` was requested but the node pool is exhausted
    /// (C `ENOMEM`).
    Capacity,
}

impl Error for ChTrieError {}

impl fmt::Display for ChTrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChTrieError::Range => write!(f, "chtrie: argument out of range"),
            ChTrieError::Capacity => write!(f, "chtrie: node pool exhausted"),
        }
    }
}

/// A single edge in a hash-bucket chain (C `struct chtrie_edge`).
struct Edge {
    next: Option<Box<Edge>>,
    from: i32,
    sym: i32,
    to: i32,
}

/// A coordinate hash trie (C `chtrie`).
///
/// All fields are private; the C struct's public internals are replaced by
/// the `alloc` / `walk` / `del` methods. Memory is released automatically
/// when the value is dropped (C `chtrie_free`).
pub struct ChTrie {
    /// Hash table: one bucket slot per `ecap`; each slot is the head of a
    /// singly-linked list of edges (C `struct chtrie_edge **etab`).
    etab: Vec<Option<Box<Edge>>>,
    /// Pool of available node indexes (C `int *idxpool`, size `n`).
    idxpool: Vec<i32>,
    /// Cursor into `idxpool`; `0` means the pool is empty (C `int *idxptr`).
    idxptr: usize,
    /// Next fresh node index; starts at 1 (0 is the root).
    idxmax: i32,
    /// Maximum number of nodes (C `maxn`).
    maxn: usize,
    /// Alphabet size (C `alphsz`).
    alphsz: usize,
    /// Hash-table capacity, `(n-1) + (n-1)/3` (C `ecap`).
    ecap: usize,
}

impl ChTrie {
    /// Translate of `chtrie_alloc(n, m)`.
    ///
    /// `n` and `m` are clamped to at least 1. Returns
    /// [`ChTrieError::Range`] where the C code would set `errno = ERANGE`.
    pub fn alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError> {
        let n = if n < 1 { 1 } else { n };
        let m = if m < 1 { 1 } else { m };
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return Err(ChTrieError::Range);
        }
        // C: if (MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3) errno = ERANGE;
        // i.e. (n-1) + (n-1)/3 must fit in i32.
        let nm1 = n - 1;
        if nm1 > i32::MAX as usize - nm1 / 3 {
            return Err(ChTrieError::Range);
        }
        // C computes ecap = (n-1) + (n-1)/3, which is 0 when n == 1 and
        // would make the bucket hash a division by zero; clamp to 1.
        let ecap = (nm1 + nm1 / 3).max(1);
        Ok(ChTrie {
            etab: (0..ecap).map(|_| None).collect(),
            idxpool: vec![0; n],
            idxptr: 0,
            idxmax: 1,
            maxn: n,
            alphsz: m,
            ecap,
        })
    }

    /// Bucket index: `h = (from * alphsz + sym) % ecap`, using wrapping
    /// unsigned arithmetic to mirror the C `unsigned long` computation.
    fn bucket(&self, from: i32, sym: i32) -> usize {
        let h = (from as usize)
            .wrapping_mul(self.alphsz)
            .wrapping_add(sym as usize);
        h % self.ecap
    }

    /// Translate of `chtrie_walk(tr, from, sym, creat)`.
    ///
    /// - `Ok(Some(idx))` — the child was found, or created when `creat` is
    ///   set (C returns the child index);
    /// - `Ok(None)` — the child was not found and `creat` is false
    ///   (C returns `-1` without setting `errno`);
    /// - `Err(ChTrieError::Capacity)` — `creat` is set but the node pool is
    ///   exhausted (C returns `-1` with `errno = ENOMEM`).
    pub fn walk(
        &mut self,
        from: i32,
        sym: i32,
        creat: bool,
    ) -> Result<Option<i32>, ChTrieError> {
        let h = self.bucket(from, sym);
        // Linear scan of the bucket chain.
        let mut p = self.etab[h].as_mut();
        while let Some(e) = p {
            if e.from == from && e.sym == sym {
                return Ok(Some(e.to));
            }
            p = e.next.as_mut();
        }
        if !creat {
            return Ok(None);
        }
        // C: if (tr->idxptr == tr->idxpool && tr->idxmax >= tr->maxn)
        //     { errno = ENOMEM; return -1; }
        if self.idxptr == 0 && self.idxmax >= self.maxn as i32 {
            return Err(ChTrieError::Capacity);
        }
        // C: if pool non-empty p->to = *--tr->idxptr; else p->to = idxmax++;
        let to = if self.idxptr > 0 {
            self.idxptr -= 1;
            self.idxpool[self.idxptr]
        } else {
            let t = self.idxmax;
            self.idxmax += 1;
            t
        };
        // Push the new edge at the head of the bucket chain.
        let edge = Box::new(Edge {
            next: self.etab[h].take(),
            from,
            sym,
            to,
        });
        self.etab[h] = Some(edge);
        Ok(Some(to))
    }

    /// Translate of `chtrie_del(tr, from, sym)`.
    ///
    /// Unlinks the matching edge from its bucket chain and recycles its
    /// `to` index back into the pool. No-op if the edge is not present.
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = self.bucket(from, sym);
        // Detach the whole chain, drop the matching edge, re-link the rest.
        let mut chain: Vec<Box<Edge>> = Vec::new();
        let mut p = self.etab[h].take();
        while let Some(mut e) = p {
            let next = e.next.take();
            chain.push(e);
            p = next;
        }
        let mut removed: Option<i32> = None;
        let mut new_chain: Option<Box<Edge>> = None;
        for mut e in chain.into_iter().rev() {
            if removed.is_none() && e.from == from && e.sym == sym {
                removed = Some(e.to);
                continue;
            }
            e.next = new_chain;
            new_chain = Some(e);
        }
        self.etab[h] = new_chain;
        if let Some(to) = removed {
            // C: *tr->idxptr++ = p->to;
            self.idxpool[self.idxptr] = to;
            self.idxptr += 1;
        }
    }
}

// `chtrie_free` is subsumed by Rust's drop semantics: the `Vec`s and `Box`es
// release all edges and arrays when the `ChTrie` value goes out of scope.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_clamps_small_values() {
        // n, m < 1 are clamped to 1 (C behavior).
        let mut tr = ChTrie::alloc(0, 0).expect("alloc(0, 0)");
        // maxn = 1: root only; any creat walk must fail with Capacity.
        assert_eq!(
            tr.walk(0, 0, true),
            Err(ChTrieError::Capacity)
        );
    }

    #[test]
    fn alloc_range_errors() {
        // n > INT_MAX → ERANGE.
        assert!(matches!(
            ChTrie::alloc(i32::MAX as usize + 1, 1),
            Err(ChTrieError::Range)
        ));
        // m > INT_MAX → ERANGE.
        assert!(matches!(
            ChTrie::alloc(1, i32::MAX as usize + 1),
            Err(ChTrieError::Range)
        ));
        // (n-1) + (n-1)/3 overflows INT_MAX → ERANGE.
        assert!(matches!(
            ChTrie::alloc(i32::MAX as usize, i32::MAX as usize),
            Err(ChTrieError::Range)
        ));
    }

    #[test]
    fn walk_create_lookup_delete_recycle() {
        let mut tr = ChTrie::alloc(4, 256).expect("alloc");
        // Create edge 0 --'a'--> 1 (fresh index, pool empty).
        assert_eq!(tr.walk(0, b'a' as i32, true), Ok(Some(1)));
        // Lookup without creat.
        assert_eq!(tr.walk(0, b'a' as i32, false), Ok(Some(1)));
        // Missing edge, no creat.
        assert_eq!(tr.walk(0, b'b' as i32, false), Ok(None));
        // Delete and recreate: the recycled index must be 1 again.
        tr.del(0, b'a' as i32);
        assert_eq!(tr.walk(0, b'a' as i32, false), Ok(None));
        assert_eq!(tr.walk(0, b'a' as i32, true), Ok(Some(1)));
        // Deleting a non-existent edge is a no-op.
        tr.del(0, b'z' as i32);
        assert_eq!(tr.walk(0, b'a' as i32, false), Ok(Some(1)));
    }

    #[test]
    fn walk_capacity_error() {
        // maxn = 2: only index 1 can be handed out.
        let mut tr = ChTrie::alloc(2, 256).expect("alloc");
        assert_eq!(tr.walk(0, b'a' as i32, true), Ok(Some(1)));
        assert_eq!(tr.walk(0, b'b' as i32, true), Err(ChTrieError::Capacity));
        // After deleting the edge, the recycled index is available again.
        tr.del(0, b'a' as i32);
        assert_eq!(tr.walk(0, b'b' as i32, true), Ok(Some(1)));
    }
}
