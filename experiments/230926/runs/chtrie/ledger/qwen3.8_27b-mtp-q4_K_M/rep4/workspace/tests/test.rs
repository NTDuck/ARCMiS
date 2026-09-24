//! Port of `tests/test.c` — the 14-case CH-Trie integration test.

use chtrie::ChTrie;

const N: usize = 65536;
const M: usize = 256;

/// Shared state mirroring the C test's globals: the trie plus the
/// `term[N]` (termination flag) and `nchild[N]` (child count) arrays.
struct State {
    tr: ChTrie,
    term: Vec<bool>,
    nchild: Vec<i32>,
}

impl State {
    fn new() -> Self {
        let tr = ChTrie::alloc(N, M).expect("chtrie_alloc");
        State {
            tr,
            term: vec![false; N],
            nchild: vec![0; N],
        }
    }

    /// C `add(char *s)`.
    fn add(&mut self, s: &str) {
        let mut it = 0;
        for c in s.bytes() {
            if self.tr.walk(it, c as i32, false).expect("chtrie_walk").is_none() {
                self.nchild[it as usize] += 1;
            }
            it = self
                .tr
                .walk(it, c as i32, true)
                .expect("chtrie_walk")
                .expect("chtrie_walk");
        }
        self.term[it as usize] = true;
    }

    /// C `del(char *s)`.
    fn del(&mut self, s: &str) {
        let mut nodes: Vec<i32> = Vec::new();
        let mut symbs: Vec<i32> = Vec::new();
        let mut it = 0;
        let mut found = true;
        for c in s.bytes() {
            nodes.push(it);
            symbs.push(c as i32);
            match self.tr.walk(it, c as i32, false).expect("chtrie_walk") {
                Some(x) => it = x,
                None => {
                    found = false;
                    break;
                }
            }
        }
        if !found || !self.term[it as usize] {
            return;
        }
        self.term[it as usize] = false;
        while it > 0 && !self.term[it as usize] && self.nchild[it as usize] == 0 {
            let n = nodes.len();
            self.tr.del(nodes[n - 1], symbs[n - 1]);
            it = nodes[n - 1];
            self.nchild[it as usize] -= 1;
        }
    }

    /// C `query(char *s)`.
    fn query(&mut self, s: &str) -> bool {
        let mut it = 0;
        for c in s.bytes() {
            match self.tr.walk(it, c as i32, false).expect("chtrie_walk") {
                Some(x) => it = x,
                None => return false,
            }
        }
        self.term[it as usize]
    }
}

#[test]
fn chtrie_test() {
    let mut st = State::new();

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let stop = ["the", "an", "a"];
    let dict3 = ["this", "that"];

    for w in dict1 {
        st.add(w);
    }
    for w in dict2 {
        st.add(w);
    }
    for w in stop {
        st.del(w);
    }
    for w in dict3 {
        st.add(w);
    }

    let test_cases = [
        "hello", "the", "his", "he", "his", "go", "he", "a", "an", "this",
        "that", "hey", "she", "hers",
    ];
    let expected_results = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    assert_eq!(test_cases.len(), expected_results.len());
    for (w, expected) in test_cases.iter().zip(expected_results.iter()) {
        assert_eq!(st.query(w) as i32, *expected, "query({w:?})");
    }
}
