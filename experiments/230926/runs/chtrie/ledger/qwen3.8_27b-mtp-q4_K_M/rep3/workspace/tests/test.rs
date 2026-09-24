//! 1:1 port of `tests/test.c` — same dictionaries, same helpers
//! (`add` / `del` / `query` over `term` / `nchild` state), and the same
//! 14 query assertions with the exact expected results.

use chtrie::{ChTrie, ROOT};

const N: usize = 65536;
const M: usize = 256;

const DICT1: &[&str] = &["", "the", "a", "an"];
const DICT2: &[&str] = &["he", "she", "his", "hers"];
const DICT3: &[&str] = &["this", "that"];
const STOP: &[&str] = &["the", "an", "a"];

/// The trie plus the test's bookkeeping arrays
/// (`term[N]` / `nchild[N]` in the C source).
struct State {
    tr: ChTrie,
    term: Vec<i32>,   /* is `i` a termination node */
    nchild: Vec<i32>, /* number of children of `i` */
}

impl State {
    fn new() -> State {
        State {
            tr: ChTrie::alloc(N, M).expect("chtrie_alloc"),
            term: vec![0; N],
            nchild: vec![0; N],
        }
    }

    fn add(&mut self, s: &str) {
        let mut it = ROOT;
        for &c in s.as_bytes() {
            if self.tr.walk(it, c as i32, false).unwrap().is_none() {
                self.nchild[it as usize] += 1;
            }
            it = self
                .tr
                .walk(it, c as i32, true)
                .expect("chtrie_walk")
                .expect("chtrie_walk");
        }
        self.term[it as usize] = 1;
    }

    fn del(&mut self, s: &str) {
        let mut nodes: Vec<i32> = Vec::new(); /* trace the path */
        let mut symbs: Vec<i32> = Vec::new();
        let mut it = ROOT;

        for &c in s.as_bytes() {
            nodes.push(it);
            symbs.push(c as i32);
            match self.tr.walk(it, c as i32, false).unwrap() {
                Some(next) => it = next,
                None => return,
            }
        }
        if self.term[it as usize] == 0 {
            return;
        }
        self.term[it as usize] = 0;
        while it > 0 && self.term[it as usize] == 0 && self.nchild[it as usize] == 0 {
            let n = nodes.len() - 1;
            self.tr.del(nodes[n], symbs[n]);
            it = nodes[n];
            self.nchild[it as usize] -= 1;
        }
    }

    fn query(&mut self, s: &str) -> i32 {
        let mut it = ROOT;
        for &c in s.as_bytes() {
            match self.tr.walk(it, c as i32, false).unwrap() {
                Some(next) => it = next,
                None => return 0,
            }
        }
        self.term[it as usize]
    }
}

#[test]
fn chtrie_test() {
    let mut st = State::new();

    // Add words to the trie
    for w in DICT1 {
        st.add(w);
    }
    for w in DICT2 {
        st.add(w);
    }
    for w in STOP {
        st.del(w);
    }
    for w in DICT3 {
        st.add(w);
    }

    // Test cases and expected results
    let test_cases = [
        "hello", "the", "his", "he", "his", "go",
        "he", "a", "an", "this", "that", "hey", "she", "hers",
    ];
    let expected_results = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    assert_eq!(test_cases.len(), expected_results.len());
    for (case, expected) in test_cases.iter().zip(expected_results.iter()) {
        let result = st.query(case);
        println!("Query: {}, Result: {}, Expected: {}", case, result, expected);
        assert_eq!(result, *expected);
    }

    // All tests passed!
}
