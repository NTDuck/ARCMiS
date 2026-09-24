//! Integration test — port of the C `tests/test.c`.
//!
//! Builds a string set via `add`/`del`/`query` helpers over `u8` symbols
//! (N = 65536 nodes, M = 256 symbols) and asserts the 14 query results.

use chtrie::ChTrie;

const N: u32 = 65536;
const M: u32 = 256;

/// Insert `s` into the trie, marking its terminal node.
/// C: `add(char *s)`.
fn add(tr: &mut ChTrie, term: &mut [bool], nchild: &mut [u32], s: &str) {
    // TODO: implement (port of C `add`)
    unimplemented!()
}

/// Delete `s` from the trie, pruning unused nodes.
/// C: `del(char *s)`.
fn del(tr: &mut ChTrie, term: &mut [bool], nchild: &mut [u32], s: &str) {
    // TODO: implement (port of C `del`)
    unimplemented!()
}

/// Query whether `s` is a terminal word in the trie.
/// C: `query(char *s)`.
fn query(tr: &ChTrie, term: &[bool], s: &str) -> bool {
    // TODO: implement (port of C `query`)
    unimplemented!()
}

#[test]
fn test_suite() {
    // TODO: port of `tests/test.c` `main`.
    //
    //   let mut tr = ChTrie::new(N, M).expect("chtrie_alloc");
    //   let mut term = vec![false; N as usize];
    //   let mut nchild = vec![0u32; N as usize];
    //
    //   for w in ["", "the", "a", "an"] { add(&mut tr, &mut term, &mut nchild, w); }
    //   for w in ["he", "she", "his", "hers"] { add(&mut tr, &mut term, &mut nchild, w); }
    //   for w in ["the", "an", "a"] { del(&mut tr, &mut term, &mut nchild, w); }
    //   for w in ["this", "that"] { add(&mut tr, &mut term, &mut nchild, w); }
    //
    //   let test_cases = [
    //       "hello", "the", "his", "he", "his", "go",
    //       "he", "a", "an", "this", "that", "hey", "she", "hers",
    //   ];
    //   let expected = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];
    //   for (case, exp) in test_cases.iter().zip(expected.iter()) {
    //       let result = if query(&tr, &term, case) { 1 } else { 0 };
    //       assert_eq!(result, *exp, "query({case})");
    //   }
    unimplemented!()
}
