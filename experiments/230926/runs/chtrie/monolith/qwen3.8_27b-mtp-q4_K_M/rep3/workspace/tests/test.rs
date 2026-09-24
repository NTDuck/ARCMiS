//! Translation of the C test suite (`tests/test.c`).

use chtrie::ChTrie;

const N: usize = 65536;
const M: usize = 256;

/// State shared by `add`, `del`, and `query`
/// (the C version used file-scope statics `tr`, `term`, and `nchild`).
struct State {
    tr: ChTrie,
    term: Vec<i32>,
    nchild: Vec<i32>,
}

fn add(st: &mut State, s: &str) {
    let mut it: i32 = 0;
    for c in s.bytes() {
        if st.tr.walk(it, c as i32, false).is_err() {
            st.nchild[it as usize] += 1;
        }
        match st.tr.walk(it, c as i32, true) {
            Ok(n) => it = n,
            Err(e) => panic!("chtrie_walk: {e}"),
        }
    }
    st.term[it as usize] = 1;
}

fn del(st: &mut State, s: &str) {
    let mut nodes: Vec<i32> = Vec::new();
    let mut symbs: Vec<i32> = Vec::new();
    let mut it: i32 = 0;
    for c in s.bytes() {
        nodes.push(it);
        symbs.push(c as i32);
        match st.tr.walk(it, c as i32, false) {
            Ok(n) => it = n,
            Err(_) => {
                it = -1;
                break;
            }
        }
    }
    if it < 0 || st.term[it as usize] == 0 {
        return;
    }
    st.term[it as usize] = 0;
    while it > 0 && st.term[it as usize] == 0 && st.nchild[it as usize] == 0 {
        let n = nodes.len() - 1;
        st.tr.del(nodes[n], symbs[n]);
        it = nodes[n];
        st.nchild[it as usize] -= 1;
    }
}

fn query(st: &mut State, s: &str) -> bool {
    let mut it: i32 = 0;
    for c in s.bytes() {
        match st.tr.walk(it, c as i32, false) {
            Ok(n) => it = n,
            Err(_) => {
                it = -1;
                break;
            }
        }
    }
    it >= 0 && st.term[it as usize] != 0
}

#[test]
fn test_dict() {
    let mut st = State {
        tr: ChTrie::alloc(N, M).expect("chtrie_alloc"),
        term: vec![0; N],
        nchild: vec![0; N],
    };

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let dict3 = ["this", "that"];
    let stop = ["the", "an", "a"];

    for w in dict1 {
        add(&mut st, w);
    }
    for w in dict2 {
        add(&mut st, w);
    }
    for w in stop {
        del(&mut st, w);
    }
    for w in dict3 {
        add(&mut st, w);
    }

    let test_cases = [
        "hello", "the", "his", "he", "his", "go", "he", "a", "an", "this", "that", "hey", "she",
        "hers",
    ];
    let expected = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (w, &exp) in test_cases.iter().zip(expected.iter()) {
        let result = query(&mut st, w) as i32;
        println!("Query: {}, Result: {}, Expected: {}", w, result, exp);
        assert_eq!(result, exp);
    }
}

#[test]
fn alloc_regulates_small_sizes() {
    // n or m < 1 are regulated to 1.
    let tr = ChTrie::alloc(0, 0).expect("chtrie_alloc");
    assert_eq!(tr.walk(0, 0, true), Ok(1));
}

#[test]
fn del_of_missing_edge_is_noop() {
    let mut tr = ChTrie::alloc(8, 256).expect("chtrie_alloc");
    // Deleting a child that was never created leaves the trie unchanged.
    tr.del(0, 5);
    assert_eq!(tr.walk(0, 5, false), Err(chtrie::ChTrieError::NotFound));
    // And creation still works afterwards.
    assert_eq!(tr.walk(0, 5, true), Ok(1));
}

#[test]
fn capacity_is_enforced() {
    // At most 2 nodes: root (0) and one child.
    let mut tr = ChTrie::alloc(2, 256).expect("chtrie_alloc");
    assert_eq!(tr.walk(0, 'a' as i32, true), Ok(1));
    assert_eq!(tr.walk(0, 'b' as i32, true), Err(chtrie::ChTrieError::Capacity));
}

#[test]
fn deleted_node_index_is_reused() {
    let mut tr = ChTrie::alloc(8, 256).expect("chtrie_alloc");
    let a = tr.walk(0, 'a' as i32, true).expect("chtrie_walk");
    tr.del(0, 'a' as i32);
    // The freed index must be recycled from the pool.
    let b = tr.walk(0, 'b' as i32, true).expect("chtrie_walk");
    assert_eq!(a, b);
}

#[test]
fn bucket_chains_do_not_leak_across_slots() {
    // Force several edges into the same bucket and verify that walking a
    // missing edge does not run off the end of a bucket chain.
    let mut tr = ChTrie::alloc(16, 256).expect("chtrie_alloc");
    for c in b"abcdefgh" {
        tr.walk(0, *c as i32, true).expect("chtrie_walk");
    }
    // 'i' was never created; lookup must terminate with NotFound.
    assert_eq!(tr.walk(0, b'i' as i32, false), Err(chtrie::ChTrieError::NotFound));
    // And every created edge is still reachable.
    for c in b"abcdefgh" {
        assert!(tr.walk(0, *c as i32, false).is_ok());
    }
}
