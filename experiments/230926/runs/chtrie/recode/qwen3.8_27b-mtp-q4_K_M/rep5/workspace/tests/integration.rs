//! Integration test mirroring `tests/test.c`: a string set built on ChTrie.

use chtrie::ChTrie;

const N: usize = 65536;
const M: usize = 256;

/// Shared state for the string-set test.
struct StringSet {
    tr: ChTrie,
    /// Is node `i` a termination node?
    term: Vec<bool>,
    /// Number of children of node `i`.
    nchild: Vec<usize>,
}

impl StringSet {
    fn new() -> Self {
        // TODO: implement (see plan.md, Part B step 1)
        unimplemented!("StringSet::new")
    }

    /// Add a word to the set.
    fn add(&mut self, s: &str) {
        // TODO: implement (see plan.md, Part B step 2)
        unimplemented!("StringSet::add")
    }

    /// Remove a word from the set, pruning leaf nodes.
    fn del(&mut self, s: &str) {
        // TODO: implement (see plan.md, Part B step 3)
        unimplemented!("StringSet::del")
    }

    /// Query whether a word is in the set.
    fn query(&self, s: &str) -> bool {
        // TODO: implement (see plan.md, Part B step 4)
        unimplemented!("StringSet::query")
    }
}

#[test]
fn string_set() {
    // TODO: implement (see plan.md, Part B step 5)
    unimplemented!("string_set")
}
