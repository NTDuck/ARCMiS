//! Integration test: Rust translation of the C test (`tests/test.c`).
//!
//! Mirrors the C logic: a `term[]` termination flag array and an
//! `nchild[]` child-count array of size N, with `add`, `del`, and
//! `query` helpers built on the `ChTrie` API.

use chtrie::{ChTrie, ChTrieError};

const N: usize = 65536;
const M: usize = 256;

/// Add a word to the trie, counting children on first creation and
/// marking the final node as a termination.
fn add(trie: &mut ChTrie, term: &mut [bool], nchild: &mut [usize], s: &str) {
    let mut from = 0usize;
    for &b in s.as_bytes() {
        let to = match trie.walk(from, b as usize, false) {
            Ok(to) => to,
            Err(ChTrieError::NotFound) => {
                let to = trie.walk(from, b as usize, true).expect("walk creat");
                nchild[from] += 1;
                to
            }
            Err(e) => panic!("unexpected walk error: {e}"),
        };
        from = to;
    }
    term[from] = true;
}

/// Delete a word: trace the path; if found and terminated, clear the
/// termination flag and prune the leaf chain (nodes with no other
/// children) using `chtrie` del and `nchild`.
fn del(trie: &mut ChTrie, term: &mut [bool], nchild: &mut [usize], s: &str) {
    let bytes = s.as_bytes();
    // Trace the path, recording (from, sym) pairs.
    let mut from = 0usize;
    let mut path: Vec<(usize, usize)> = Vec::with_capacity(bytes.len());
    for &b in bytes {
        match trie.walk(from, b as usize, false) {
            Ok(to) => {
                path.push((from, b as usize));
                from = to;
            }
            Err(_) => return, // not found
        }
    }
    if !term[from] {
        return;
    }
    term[from] = false;
    // Prune the leaf chain backwards while nodes have no other children.
    let mut node = from;
    for (parent, sym) in path.iter().rev() {
        if nchild[node] != 0 {
            break;
        }
        trie.del(*parent, *sym);
        nchild[*parent] -= 1;
        node = *parent;
    }
}

/// Query a word: walk without creating; true iff the path exists and
/// the final node is a termination.
fn query(trie: &mut ChTrie, term: &[bool], s: &str) -> bool {
    let mut from = 0usize;
    for &b in s.as_bytes() {
        match trie.walk(from, b as usize, false) {
            Ok(to) => from = to,
            Err(_) => return false,
        }
    }
    term[from]
}

#[test]
fn chtrie_test() {
    let mut trie = ChTrie::new(N, M).expect("chtrie_alloc");
    let mut term = vec![false; N];
    let mut nchild = vec![0usize; N];

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let stop = ["the", "an", "a"];
    let dict3 = ["this", "that"];

    for w in dict1 {
        add(&mut trie, &mut term, &mut nchild, w);
    }
    for w in dict2 {
        add(&mut trie, &mut term, &mut nchild, w);
    }
    for w in stop {
        del(&mut trie, &mut term, &mut nchild, w);
    }
    for w in dict3 {
        add(&mut trie, &mut term, &mut nchild, w);
    }

    let test_cases = [
        "hello", "the", "his", "he", "his", "go", "he", "a", "an", "this", "that", "hey", "she",
        "hers",
    ];
    let expected = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (w, e) in test_cases.iter().zip(expected.iter()) {
        let got = query(&mut trie, &term, w) as i32;
        assert_eq!(got, *e, "query({w}) = {got}, expected {e}");
    }
}

#[test]
fn new_zero_args_works() {
    let mut trie = ChTrie::new(0, 0).expect("new(0, 0) should work");
    // Root exists; walking a missing edge without creat fails.
    assert!(matches!(
        trie.walk(0, 0, false),
        Err(ChTrieError::NotFound)
    ));
    // With n clamped to 1 there is no room for new nodes.
    assert_eq!(trie.walk(0, 0, true), Err(ChTrieError::Capacity));
}

#[test]
fn walk_not_found_error() {
    let mut trie = ChTrie::new(16, 8).expect("alloc");
    assert_eq!(trie.walk(0, 5, false), Err(ChTrieError::NotFound));
    // A symbol outside the alphabet is rejected.
    assert!(trie.walk(0, 8, false).is_err());
}
