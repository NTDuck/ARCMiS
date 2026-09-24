//! Rust port of the C test suite `tests/test.c` for the CH-Trie library.
//!
//! The C test allocates a trie with N=65536 nodes and M=256 alphabet,
//! adds dict1 = {"", "the", "a", "an"}, dict2 = {"he", "she", "his", "hers"},
//! deletes stop = {"the", "an", "a"}, adds dict3 = {"this", "that"}, and
//! checks 14 query results.
//!
//! The C helpers `add`/`del`/`query` rely on the C struct's `term[]` and
//! `nchild[]` arrays, which are not part of the Rust public API
//! (`new` / `walk` / `del` only). They are therefore emulated here with
//! equivalent bookkeeping:
//! - `term`   : set of node indices marked as terminal words;
//! - `nchild` : per-node count of outgoing edges (C `nchild[]`);
//! - `edges`  : set of (from, sym) edges currently present, used to detect
//!              whether a `walk(creat=true)` actually created a new edge
//!              (C `chtrie_walk` increments `nchild[from]` only on creation).

use chtrie::ChTrie;
use std::collections::{HashMap, HashSet};

/// Test-side state mirroring the C struct's `term` / `nchild` bookkeeping.
struct State {
    tr: ChTrie,
    term: HashSet<i32>,
    nchild: HashMap<i32, i32>,
    edges: HashSet<(i32, i32)>,
}

/// Port of the C `add(tr, from, w)`:
/// walk with creat=1 per char, then mark the final node as a term.
fn add(st: &mut State, from: i32, w: &str) -> i32 {
    let mut it = from;
    for c in w.chars() {
        let sym = c as u8 as i32;
        let to = st.tr.walk(it, sym, true);
        if to < 0 {
            return -1;
        }
        // C `chtrie_walk` increments `nchild[from]` only when it creates
        // the edge; track creation via the edge set.
        if st.edges.insert((it, sym)) {
            *st.nchild.entry(it).or_insert(0) += 1;
        }
        it = to;
    }
    st.term.insert(it);
    0
}

/// Port of the C `del(tr, from, w)`:
/// trace the path (creat=0), unmark term at each node, and prune the edge
/// via `chtrie_del` whenever the child has no children (nchild == 0).
fn del(st: &mut State, from: i32, w: &str) -> i32 {
    let mut it = from;
    for c in w.chars() {
        let sym = c as u8 as i32;
        let to = st.tr.walk(it, sym, false);
        if to < 0 {
            return -1;
        }
        st.term.remove(&to);
        if st.nchild.get(&to).copied().unwrap_or(0) == 0 {
            st.tr.del(it, sym);
            st.edges.remove(&(it, sym));
            if let Some(v) = st.nchild.get_mut(&it) {
                *v -= 1;
            }
        }
        it = to;
    }
    0
}

/// Port of the C `query(tr, from, w)`:
/// walk with creat=0; return 1 iff the path exists and the final node is
/// marked as a term.
fn query(st: &mut State, from: i32, w: &str) -> i32 {
    let mut it = from;
    for c in w.chars() {
        let sym = c as u8 as i32;
        it = st.tr.walk(it, sym, false);
        if it < 0 {
            return 0;
        }
    }
    if st.term.contains(&it) {
        1
    } else {
        0
    }
}

#[test]
fn test_chtrie() {
    let mut st = State {
        tr: ChTrie::new(65536, 256).expect("chtrie_alloc(65536, 256)"),
        term: HashSet::new(),
        nchild: HashMap::new(),
        edges: HashSet::new(),
    };

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let stop = ["the", "an", "a"];
    let dict3 = ["this", "that"];

    for w in dict1 {
        assert_eq!(add(&mut st, 0, w), 0, "add({:?})", w);
    }
    for w in dict2 {
        assert_eq!(add(&mut st, 0, w), 0, "add({:?})", w);
    }
    for w in stop {
        assert_eq!(del(&mut st, 0, w), 0, "del({:?})", w);
    }
    for w in dict3 {
        assert_eq!(add(&mut st, 0, w), 0, "add({:?})", w);
    }

    let test_cases = [
        "hello", "the", "his", "he", "his", "go", "he", "a", "an", "this",
        "that", "hey", "she", "hers",
    ];
    let expected_results = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (w, exp) in test_cases.iter().zip(expected_results.iter()) {
        assert_eq!(query(&mut st, 0, w), *exp, "query({:?})", w);
    }
}
