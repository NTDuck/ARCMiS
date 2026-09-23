//! CH-Trie: the official Rust library of the coordinate hash trie.
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
//! the average case and O(m) for the worst case. The space complexity is
//! O(n), unrelated to `m`.

use std::mem;

/// A single edge in the global edge hash table.
#[derive(Debug, Clone)]
pub struct ChTrieEdge {
    /// Index of the next edge in the same bucket, or `None`.
    pub next: Option<usize>,
    pub from: i32,
    pub sym: i32,
    pub to: i32,
}

/// A `chtrie` instance represents a trie.
#[derive(Debug)]
pub struct ChTrie {
    /// Global hash table of edges; each slot holds the index of the head
    /// edge of its bucket, or `None`.
    etab: Vec<Option<usize>>,
    /// All edges, indexed by the values stored in `etab`.
    edges: Vec<ChTrieEdge>,
    /// Pool of available node indexes.
    idxpool: Vec<i32>,
    /// Number of indexes already taken back into the pool.
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

/// Allocate a trie with at most `n` nodes, and the alphabet size `m`.
///
/// If `n` or `m` is less than 1, they will be regulated to 1.
///
/// Nodes in the trie are indexed by non-negative integers less than `n`.
/// The root node is indexed by 0.
/// Symbols are non-negative integers less than `m`.
///
/// Upon success, return a `ChTrie`.
/// Otherwise, return `None`.
pub fn chtrie_alloc(n: usize, m: usize) -> Option<ChTrie> {
    let n = n.max(1);
    let m = m.max(1);
    if n > i32::MAX as usize || m > i32::MAX as usize {
        return None;
    }
    let nm1 = n - 1;
    if nm1 / 3 > i32::MAX as usize - nm1 {
        return None;
    }
    let ecap = nm1 + nm1 / 3;
    let etab = vec![None; ecap];
    let idxpool = vec![0; n];
    Some(ChTrie {
        etab,
        edges: Vec::new(),
        idxpool,
        idxptr: 0,
        idxmax: 1,
        maxn: n,
        alphsz: m,
        ecap,
    })
}

/// Walk from one node to its child.
///
/// If the child didn't exist and `creat` is non-zero,
/// a new node will be created.
///
/// Upon the child is found or created, return the index of the child.
/// Otherwise, return -1.
pub fn chtrie_walk(tr: &mut ChTrie, from: i32, sym: i32, creat: bool) -> i32 {
    let h = ((from as u64) * (tr.alphsz as u64) + sym as u64) % tr.ecap as u64;
    let h = h as usize;
    let mut p = tr.etab[h];
    while let Some(idx) = p {
        let e = &tr.edges[idx];
        if e.from == from && e.sym == sym {
            return e.to;
        }
        p = e.next;
    }
    if creat {
        if tr.idxptr == 0 && tr.idxmax >= tr.maxn as i32 {
            return -1;
        }
        let next = tr.etab[h];
        let idx = tr.edges.len();
        tr.edges.push(ChTrieEdge {
            next,
            from,
            sym,
            to: 0,
        });
        tr.etab[h] = Some(idx);
        if tr.idxptr > 0 {
            tr.idxptr -= 1;
            tr.edges[idx].to = tr.idxpool[tr.idxptr];
        } else {
            tr.edges[idx].to = tr.idxmax;
            tr.idxmax += 1;
        }
        return tr.edges[idx].to;
    }
    -1
}

/// Delete a child node.
///
/// The child node must be a leaf if it exists,
/// or the behavior is undefined.
///
/// If the child doesn't exist, the trie shall be left unchanged.
pub fn chtrie_del(tr: &mut ChTrie, from: i32, sym: i32) {
    let h = ((from as u64) * (tr.alphsz as u64) + sym as u64) % tr.ecap as u64;
    let h = h as usize;
    let mut prev: Option<usize> = None;
    let mut p = tr.etab[h];
    while let Some(idx) = p {
        if tr.edges[idx].from == from && tr.edges[idx].sym == sym {
            break;
        }
        prev = Some(idx);
        p = tr.edges[idx].next;
    }
    let idx = match p {
        Some(idx) => idx,
        None => return,
    };
    if let Some(q) = prev {
        tr.edges[q].next = tr.edges[idx].next;
    } else {
        tr.etab[h] = None;
    }
    tr.idxpool[tr.idxptr] = tr.edges[idx].to;
    tr.idxptr += 1;
}

/// Free the trie.
///
/// In Rust, memory is also released automatically when the `ChTrie` value
/// is dropped; this function is provided for API parity with the C library.
pub fn chtrie_free(tr: ChTrie) {
    mem::drop(tr)
}
