use chtrie::{chtrie_alloc, chtrie_del, chtrie_free, chtrie_walk, ChTrie};

const N: usize = 65536;
const M: usize = 256;

static DICT1: [&str; 4] = ["", "the", "a", "an"];
static DICT2: [&str; 4] = ["he", "she", "his", "hers"];
static DICT3: [&str; 2] = ["this", "that"];
static STOP: [&str; 3] = ["the", "an", "a"];

/// A string set built on top of the coordinate hash trie.
struct TrieSet {
    tr: ChTrie,
    term: Vec<i32>, // is `i` a termination node
    nchild: Vec<i32>, // number of children of `i`
}

impl TrieSet {
    fn new() -> Self {
        let tr = chtrie_alloc(N, M).expect("chtrie_alloc");
        TrieSet {
            tr,
            term: vec![0; N],
            nchild: vec![0; N],
        }
    }

    fn add(&mut self, s: &str) {
        let mut it: i32 = 0;
        for c in s.bytes() {
            if chtrie_walk(&mut self.tr, it, c as i32, false) == -1 {
                self.nchild[it as usize] += 1;
            }
            let nxt = chtrie_walk(&mut self.tr, it, c as i32, true);
            assert!(nxt >= 0, "chtrie_walk failed");
            it = nxt;
        }
        self.term[it as usize] = 1;
    }

    fn del(&mut self, s: &str) {
        let mut nodes: Vec<i32> = Vec::new();
        let mut symbs: Vec<i32> = Vec::new();
        let mut it: i32 = 0;

        while it >= 0 {
            let b = s.as_bytes();
            let pos = nodes.len();
            if pos >= b.len() {
                break;
            }
            nodes.push(it);
            symbs.push(b[pos] as i32);
            it = chtrie_walk(&mut self.tr, it, b[pos] as i32, false);
        }
        if it < 0 || self.term[it as usize] == 0 {
            return;
        }
        self.term[it as usize] = 0;
        while it > 0 && self.term[it as usize] == 0 && self.nchild[it as usize] == 0 {
            let n = nodes.len() - 1;
            chtrie_del(&mut self.tr, nodes[n], symbs[n]);
            it = nodes[n];
            self.nchild[it as usize] -= 1;
        }
    }

    fn query(&mut self, s: &str) -> i32 {
        let mut it: i32 = 0;
        for c in s.bytes() {
            if it < 0 {
                break;
            }
            it = chtrie_walk(&mut self.tr, it, c as i32, false);
        }
        if it >= 0 && self.term[it as usize] != 0 {
            1
        } else {
            0
        }
    }
}

#[test]
fn test_string_set() {
    let mut set = TrieSet::new();

    // Add words to the trie
    for w in DICT1.iter() {
        set.add(w);
    }
    for w in DICT2.iter() {
        set.add(w);
    }
    for w in STOP.iter() {
        set.del(w);
    }
    for w in DICT3.iter() {
        set.add(w);
    }

    // Test cases and expected results
    let test_cases = [
        "hello", "the", "his", "he", "his", "go",
        "he", "a", "an", "this", "that", "hey", "she", "hers",
    ];
    let expected_results = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (w, expected) in test_cases.iter().zip(expected_results.iter()) {
        let result = set.query(w);
        println!("Query: {}, Result: {}, Expected: {}", w, result, expected);
        assert_eq!(result, *expected);
    }

    chtrie_free(set.tr);
    println!("All tests passed!");
}
