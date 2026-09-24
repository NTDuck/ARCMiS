# CH-Trie C → Rust translation — worker notes

## Brainstorm

Source under translation (C, from `assets/ReCodeAgent/data/tool_projects/crust/chtrie/c`):
- `src/chtrie.h` / `src/chtrie.c` — the coordinate hash trie library.
- `tests/test.c` — the 14-case integration test.
- `Makefile`, `README.md` — build + docs.

Public C API:
- `chtrie *chtrie_alloc(size_t n, size_t m)` — returns `NULL` + sets `errno` (`ERANGE` on
  out-of-range/overflow, `ENOMEM` on allocation failure).
- `int chtrie_walk(chtrie *tr, int from, int sym, int creat)` — returns child index, or `-1`
  if not found; if `creat` and the node pool is exhausted, sets `errno = ENOMEM` and returns `-1`.
- `void chtrie_del(chtrie *tr, int from, int sym)` — delete a leaf edge; no-op if absent.
- `void chtrie_free(chtrie *tr)`.

Internal struct fields: `etab` (array of edge-list heads), `idxpool` (pool of free node
indexes), `idxptr` (pointer into the pool), `idxmax` (next fresh index), `maxn`, `alphsz`,
`ecap` (hash-table capacity = `(n-1) + (n-1)/3`).

### 1. Core difficulties

**a) `errno`-based error signaling.**
C signals two distinct failure classes through `errno`:
- `chtrie_alloc`: `ERANGE` when `n`/`m > INT_MAX` or when `ecap = (n-1)+(n-1)/3` would
  overflow `INT_MAX`; `ENOMEM` when `calloc` fails.
- `chtrie_walk`: `ENOMEM` when `creat` is set but the node pool is exhausted
  (`idxptr == idxpool && idxmax >= maxn`).

Rust idiom: a single `Result<T, E>` per call. I will define a small error enum
(`ChTrieError`) with variants `Range` and `Capacity` (and possibly `Alloc`), implementing
`std::error::Error` + `Display`. This preserves the two-class distinction the C code makes
without leaking `errno`. `alloc` → `Result<ChTrie, ChTrieError>`; `walk` needs to express
three outcomes (found, not-found, pool-exhausted) — see (b).

**b) `walk` has three outcomes, not two.**
`-1` means both "not found" (normal) and "allocation failed" (error). In Rust the clean
model is `Result<Option<usize>, ChTrieError>`:
- `Ok(Some(idx))` — child found or created;
- `Ok(None)` — child not found (the C `-1` in the non-error case);
- `Err(ChTrieError::Capacity)` — `creat` requested but pool exhausted (the C `ENOMEM` case).

This keeps the test's `add()` logic natural: probe with `creat=false` to detect a new child
(`Ok(None)` → bump `nchild`), then `creat=true` and require `Ok(Some(..))`.

**c) Exposed internal fields.**
C exposes `etab`, `idxpool`, `idxptr`, `idxmax`, `maxn`, `alphsz`, `ecap` as a public
struct. In Rust I will make all fields **private** and expose only the four methods.
Modeling:
- `etab: Vec<Option<Box<Edge>>>` — one slot per hash bucket; each slot is the head of a
  singly-linked list of edges. `Edge { next: Option<Box<Edge>>, from: i32, sym: i32, to: i32 }`.
  (Alternative: `Vec<Edge>` with `next: Option<usize>` index — avoids per-edge heap boxes,
  but the `Box` form is the most faithful 1:1 mapping of the C linked list and is simplest
  to reason about. I'll go with `Box`.)
- `idxpool: Vec<i32>` — the free-index pool (size `n`).
- `idxptr: usize` — index into `idxpool` (replaces the C pointer `tr->idxptr`).
- `idxmax: i32` — next fresh node index (starts at 1; 0 is the root).
- `maxn: usize`, `alphsz: usize`, `ecap: usize`.

**d) `fatal` macro + `exit(-1)`.**
`#define fatal(s) do { perror(s); exit(-1); } while (0)` is used in the test's `add()` when
`chtrie_walk(..., creat=1)` fails. In the Rust test this becomes a hard failure: use
`.expect("chtrie_walk")` / `.unwrap()` on the `Result`, or `match` and `panic!`. Since the
test is a `#[test]`, a panic is the correct "fatal" equivalent (it fails the test rather than
silently exiting). No global `exit` needed.

**e) Memory: `calloc`/`malloc`/`free` vs `Vec`/`Box`.**
- `calloc(1, sizeof *tr)` → constructing the `ChTrie` struct.
- `calloc(ecap, sizeof *etab[0])` → `vec![None; ecap]`.
- `calloc(n, sizeof *idxpool[0])` → `vec![0i32; n]`.
- `malloc(sizeof *p)` / `free(p)` → `Box::new(Edge{..})` / drop.
- `chtrie_free` (manual free of every edge + arrays) → replaced by Rust's `Drop`/automatic
  deallocation. I will implement `Drop for ChTrie` (or rely on default drop) so no manual
  `free` is needed; the C `chtrie_free` maps to the value going out of scope. I may still
  provide a `free`/`drop`-style method for API symmetry, but it is not required.

**f) Index-pool pointer arithmetic.**
C uses `*--tr->idxptr` (pop a free index) and `*tr->idxptr++ = p->to` (push a free index),
with `tr->idxptr == tr->idxpool` meaning "pool empty". Translated to a `usize` cursor:
- pop: `let idx = self.idxpool[self.idxptr]; self.idxptr -= 1;` (guard `idxptr > 0`).
- push: `self.idxpool[self.idxptr] = to; self.idxptr += 1;` (guard `idxptr < idxpool.len()`).
- "pool empty" test: `self.idxptr == 0`.
- "pool full / no fresh index" test: `self.idxptr == 0 && self.idxmax >= self.maxn`.
All index math must use checked/`debug_assert` bounds to avoid out-of-range panics.

**g) Hash function & types.**
`h = (from * alphsz + sym) % ecap` uses `unsigned long`. In Rust use `usize`/`u64` with
`wrapping_mul`/`wrapping_add` to mirror the C unsigned wraparound, then `% ecap`. `from`,
`sym`, `to` are `int` in C → `i32` in Rust (they are node/symbol indexes, non-negative in
practice).

### 2. Candidate Rust API approaches

**Option A — mirror the C API closely.**
```
impl ChTrie {
    fn alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError>;
    fn walk(&mut self, from: i32, sym: i32, creat: bool) -> Result<Option<i32>, ChTrieError>;
    fn del(&mut self, from: i32, sym: i32);
    // chtrie_free → Drop
}
```
Pros: 1:1 with the C header, easiest to verify against the C test; `creat` flag preserved.
Cons: `creat: bool` and `Result<Option<..>>` are slightly less idiomatic.

**Option B — more Rust-idiomatic.**
```
impl ChTrie {
    fn new(n: usize, m: usize) -> Result<ChTrie, ChTrieError>;
    fn get(&self, from: i32, sym: i32) -> Option<i32>;
    fn insert(&mut self, from: i32, sym: i32) -> Result<i32, ChTrieError>;
    fn remove(&mut self, from: i32, sym: i32);
}
```
Pros: idiomatic names, `get`/`insert` split removes the `creat` flag.
Cons: diverges from the C API; the C test's `add()`/`del()`/`query()` helpers rely on the
single `walk(from, sym, creat)` primitive, so a faithful port is cleaner with Option A.

**Recommendation: Option A** (faithful mirror) with Rust error handling (`Result`/`Option`,
`Drop`). Rationale: the task is a *translation* whose correctness is judged by the C test's
14 cases; keeping `alloc`/`walk`/`del` names and the `creat` flag makes the port a direct,
auditable mapping and minimizes the risk of semantic drift. I will still use idiomatic Rust
internals (private fields, `Vec`/`Box`, `Result`/`Option`, `Drop`) and a proper `Error`
type. I may add thin idiomatic aliases (`new` → `alloc`) for convenience, but the primary
surface stays Option A.

### 3. Cargo project layout

```
Cargo.toml          # package "chtrie", edition 2021, [lib] name = "chtrie"
src/lib.rs          # re-export (or the whole impl) — I'll put the impl in src/chtrie.rs
src/chtrie.rs       # ChTrie, Edge, ChTrieError, alloc/walk/del, Drop
tests/test.rs       # port of tests/test.c (14 cases) as a #[test]
```
`Cargo.toml` will declare `[lib] path = "src/lib.rs"` and the package name `chtrie` so the
integration test does `use chtrie::ChTrie;`.

### 4. Verification

- `cargo build` must succeed with no errors (warnings acceptable but I'll aim for clean).
- `cargo test` must pass. The ported test must reproduce the C test exactly:
  - build trie with `N = 65536`, `M = 256`;
  - `add` dict1 = `["", "the", "a", "an"]`, `add` dict2 = `["he", "she", "his", "hers"]`;
  - `del` stop = `["the", "an", "a"]`;
  - `add` dict3 = `["this", "that"]`;
  - then the 14 queries with expected results:
    - cases:  `hello the his he his go he a an this that hey she hers`
    - expect: `0 0 1 1 1 0 1 0 0 1 1 0 1 1`
  - assert each `query == expected` (Rust `assert_eq!`), matching the C `assert`.
- I will also add a couple of small unit tests for the error paths (`alloc` with `n`/`m`
  beyond `i32::MAX` → `Err(Range)`, and pool exhaustion → `Err(Capacity)`) to cover the
  `errno` branches, since the C test does not exercise them.

## Report (brainstorm, no code)

Note: the C sources (src/chtrie.c, src/chtrie.h, tests/test.c, Makefile, README.md,
LICENSE) are NOT present in this workspace — only notes.md, plan.md, tasks.json exist.
The brainstorm report was therefore produced from the existing notes/plan (written with
access to the sources) plus knowledge of the CH-Trie reference implementation. Full
report delivered in the worker reply.

## Brainstorm report (ledger worker, this session)

Workspace state check (this session):
- `find . -type f` → only `notes.md`, `plan.md`, `tasks.json`. The C sources
  (src/chtrie.c, src/chtrie.h, tests/test.c, Makefile, README.md, LICENSE) are
  NOT present in the workspace, contrary to the task description. The earlier
  notes/plan were written with access to the sources (path recorded:
  `assets/ReCodeAgent/data/tool_projects/crust/chtrie/c`).
- Toolchain: cargo 1.97.1 / rustc available at ~/.cargo/bin.

Findings (brainstorm, no code written):

1. Core difficulties of the C → Rust translation:
   - errno-based error signaling: chtrie_alloc sets ERANGE (n/m > INT_MAX or
     ecap overflow) and ENOMEM (calloc failure); chtrie_walk sets ENOMEM on
     pool exhaustion. Rust: one Result per call with a ChTrieError enum
     (Range / Capacity) implementing std::error::Error + Display.
   - walk has three outcomes (found / not-found / pool-exhausted), C collapses
     the latter two to -1 + errno. Rust: Result<Option<i32>, ChTrieError>.
   - Public struct with internal fields (etab, idxpool, idxptr, idxmax, maxn,
     alphsz, ecap) and chtrie_edge linked lists → private fields,
     Vec<Option<Box<Edge>>> for etab, Vec<i32> + usize cursor for the pool.
   - malloc/calloc/free + goto-based error paths → Vec/Box, RAII, Drop
     replaces chtrie_free; no manual cleanup paths needed.
   - Tests use assert() + fatal() (perror + exit(-1)); test binary links the
     .c file directly. Rust: #[test] in tests/test.rs, .expect()/unwrap()
     replaces fatal (panic fails the test), assert_eq! replaces assert.
   - Makefile (.o + test binary) → Cargo.toml + src/lib.rs + tests/test.rs,
     `cargo test` as the target command.
   - Hash `h = (from*alphsz+sym) % ecap` uses unsigned long wraparound →
     wrapping_mul/wrapping_add in usize, then % ecap.

2. Candidate approaches:
   - A: Faithful mirror — ChTrie::alloc/walk/del with creat flag,
     Result<Option<i32>>, Drop for free. 1:1 auditable mapping to the C API.
   - B: Idiomatic — new/get/insert/remove, Option/Result split, no creat flag.
     Cleaner Rust but diverges from the C test helpers (add/del/query rely on
     the single walk(from, sym, creat) primitive).
   - C: Hybrid — Option A surface plus thin idiomatic aliases (new → alloc).
   - Layout: single src/lib.rs vs. src/lib.rs + src/chtrie.rs (impl in
     chtrie.rs, re-exported). Tests: integration tests/test.rs (chosen,
     mirrors tests/test.c) vs. #[cfg(test)] unit tests in lib.rs.

3. Recommendation: Option A (faithful mirror) with idiomatic Rust internals
   (private fields, Vec/Box, Result/Option, Drop, ChTrieError enum).
   Rationale: correctness is judged by the C test's 14 cases; keeping the
   alloc/walk/del names and the creat flag makes the port a direct, auditable
   mapping and minimizes semantic drift. Layout: Cargo.toml (package "chtrie",
   edition 2021), src/lib.rs (+ src/chtrie.rs), tests/test.rs.

4. Test expectations (must be reproduced exactly):
   - trie N = 65536, M = 256;
   - add dict1 = ["", "the", "a", "an"]; add dict2 = ["he", "she", "his", "hers"];
   - del stop = ["the", "an", "a"]; add dict3 = ["this", "that"];
   - queries: hello the his he his go he a an this that hey she hers
   - expected: 0 0 1 1 1 0 1 0 0 1 1 0 1 1
   - Plus extra unit tests for errno branches not covered by the C test:
     alloc with n/m > i32::MAX → Err(Range); pool exhaustion → Err(Capacity).

## Verification + completion (ledger worker, this session)

Workspace state found:
- Files present: `Cargo.toml`, `src/lib.rs`, `src/chtrie.rs`, `tests/test.rs`,
  `notes.md`, `plan.md`, `tasks.json`, `Cargo.lock`, `target/`.
- Despite the previous worker's report of compile errors, the sources in the
  workspace are complete and consistent with the plan (Option A: faithful
  mirror of the C API).

Verification performed:
- `cargo test` (incremental): 4 unit tests + 1 integration test, all pass.
- `cargo clean && cargo test` (from scratch): compiles cleanly, all tests pass:
  - `chtrie::tests::alloc_clamps_small_values` ... ok
  - `chtrie::tests::alloc_range_errors` ... ok
  - `chtrie::tests::walk_capacity_error` ... ok
  - `chtrie::tests::walk_create_lookup_delete_recycle` ... ok
  - `chtrie_test` (14-case port of tests/test.c) ... ok
- No code changes were required; no fixes were needed.

Semantic notes (intentional, documented in code):
- `ChTrieError::Range` models C `ERANGE`; `ChTrieError::Capacity` models C
  `ENOMEM` on node-pool exhaustion. C's `ENOMEM` on `calloc` failure in
  `chtrie_alloc` is not representable (Rust allocation aborts on OOM).
- `ecap` is clamped to >= 1 when `n == 1` (C would compute ecap = 0 and divide
  by zero in the hash); with `n == 1` the pool holds only the root, so any
  `creat` walk correctly fails with `Capacity`.
- `chtrie_free` is replaced by Rust drop semantics (no explicit `Drop` impl
  needed; `Vec`/`Box` release everything).
- Hash uses `wrapping_mul`/`wrapping_add` in `usize` to mirror C's
  `unsigned long` arithmetic.

Tasks t1–t6 marked done in tasks.json.
