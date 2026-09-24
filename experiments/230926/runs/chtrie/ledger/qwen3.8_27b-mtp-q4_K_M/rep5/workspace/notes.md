# Notes

## Session 1 — crate setup + library port
- Created `plan.md` (strategy + task list) and `tasks.json` (4 tasks).
- Created `Cargo.toml`: package `chtrie`, edition 2021, lib at `src/lib.rs`.
  - Added empty `[workspace]` table because the workspace dir is nested inside
    an outer Cargo workspace (`/home/ayin/projs/ARCMiS/Cargo.toml`); without
    it `cargo build` refuses to run.
- Created `src/lib.rs` — full port of `chtrie.c`:
  - `pub struct ChTrie { etab: Vec<Vec<Edge>>, idxpool: Vec<i32>, idxptr: usize, idxmax: i32, maxn: i32, alphsz: i32, ecap: usize }`
  - `pub struct Edge { from: i32, sym: i32, to: i32 }` (C linked lists modeled
    as per-slot `Vec<Edge>`; C prepend order preserved via `insert(0, ..)`).
  - `pub enum ChTrieError { Erange, Enomem }` with `Display` + `std::error::Error`.
  - `ChTrie::new(n, m) -> Result<ChTrie, ChTrieError>` mirrors `chtrie_alloc`
    (regulate to >=1, ERANGE checks incl. `MIN(INT_MAX, SIZE_MAX) - (n-1) < (n-1)/3`,
    `ecap = (n-1) + (n-1)/3`, pool `vec![0; n]`, `idxmax = 1`, `idxptr = 0`).
  - `walk(&mut self, from, sym, creat) -> i32` mirrors `chtrie_walk`
    (search, ENOMEM -> -1, prepend, pool-stack pop / fresh index).
  - `del(&mut self, from, sym)` mirrors `chtrie_del` (remove edge, push `to`
    onto the pool stack at `idxptr`).
  - `Drop` impl (no-op) for API parity with `chtrie_free`.
  - Hash: `(from as usize).wrapping_mul(alphsz as usize).wrapping_add(sym as usize) % ecap`
    — reproduces C `unsigned long` wrap for negative inputs; no `unsafe`.
- `cargo build`: **success** (`Finished dev profile`), no warnings.
- Next: port tests, then build+test verification.

## Session 2 — brainstorm (this task)
- Discrepancy: workspace has NO C sources (src/chtrie.c, src/chtrie.h,
  tests/test.c, Makefile, README.md) despite the task description; it
  already contains a completed Rust port (src/lib.rs) from session 1.
  C semantics reconstructed from lib.rs docs + standard CH-Trie code.
- Wrote `brainstorm.md` (~140 lines): core difficulties (manual memory
  mgmt, errno signaling, raw-pointer struct, idxptr/idxmax pool stack,
  C89/unsigned-wrap hash, test harness), candidate approaches (1:1 port
  vs idiomatic rewrite vs unsafe), recommended approach with API mapping
  table (new/walk/del/Drop), 12-step test port with expected values,
  Cargo.toml notes (no deps, edition 2021, [workspace] guard).
- No code files written or modified, per task instructions.

## Verification round 1
- **File inventory** (top level): `brainstorm.md` (5645 B), `Cargo.lock` (150 B),
  `Cargo.toml` (167 B), `notes.md`, `plan.md` (890 B), `tasks.json` (377 B),
  `src/lib.rs` (7045 B). **No `tests/` directory exists.**
- **Public API** (from `src/lib.rs`):
  - `pub enum ChTrieError { Erange, Enomem }` — `Debug/Clone/Copy/PartialEq/Eq`,
    `Display`, `std::error::Error`. Maps C `errno` ERANGE/ENOMEM.
  - `pub struct Edge { from: i32, sym: i32, to: i32 }` — `Debug/Clone/Copy/PartialEq/Eq`.
  - `pub struct ChTrie` (private fields: `etab: Vec<Vec<Edge>>`, `idxpool: Vec<i32>`,
    `idxptr: usize`, `idxmax: i32`, `maxn: i32`, `alphsz: i32`, `ecap: usize`).
  - `ChTrie::new(n: i32, m: i32) -> Result<ChTrie, ChTrieError>`
  - `ChTrie::walk(&mut self, from: i32, sym: i32, creat: bool) -> i32`
  - `ChTrie::del(&mut self, from: i32, sym: i32)`
  - `impl Drop for ChTrie` (no-op, parity with `chtrie_free`).
- **Semantics review vs C**:
  - `new`: regulates n,m to >=1; ERANGE check `MIN(INT_MAX, SIZE_MAX) - (n-1) < (n-1)/3`
    done in i64 (correct); `ecap = (n-1) + (n-1)/3`; pool `vec![0; n]`, `idxmax = 1`,
    `idxptr = 0`. Matches `chtrie_alloc`. Note: `Enomem` is declared but never
    returned by `new` (allocation failures abort instead of returning ENOMEM —
    acceptable Rust idiom, but a semantic difference from C).
  - `walk`: search slot list; miss + `!creat` -> -1; miss + `creat` with
    `idxptr == 0 && idxmax >= maxn` -> -1 (C ENOMEM); pool pop (`idxptr -= 1`)
    or fresh `idxmax++`; prepend via `insert(0, ..)`. Matches `chtrie_walk`.
  - `del`: removes matching edge, pushes `to` at `idxpool[idxptr++]`. Matches
    `chtrie_del` LIFO pool reuse.
  - `hash`: `(from as usize).wrapping_mul(alphsz).wrapping_add(sym) % ecap` —
    reproduces C `unsigned long` wrap for negative inputs. Correct.
- **Build**: `cargo build` -> exit 0, `Finished dev profile`, no warnings.
- **Tests**: `cargo test` -> exit 0, but **0 tests ran** (unit tests: 0 passed /
  0 failed; doc-tests: 0 passed / 0 failed). No test files exist anywhere;
  `tasks.json` still lists `tests-port` as pending. The C test suite has NOT
  been ported, so "tests pass" is vacuous.
- **Conclusion**: compiles cleanly; API and semantics match the C library per
  code review; however the test port is missing, so behavioral verification
  is incomplete. No fixes applied (per task instructions).

## Test port round
- **Files**: `tests/test.rs` (integration test, port of C `tests/test.c`),
  `src/lib.rs` (unit tests `#[cfg(test)] mod tests` + one bug fix).
- **Integration test** (`test_chtrie`): N=65536, M=256; add dict1
  {"", "the", "a", "an"}, add dict2 {"he", "she", "his", "hers"}, del stop
  {"the", "an", "a"}, add dict3 {"this", "that"}; 14 query assertions over
  test_cases/expected_results exactly as in the C test. C `term[]`/`nchild[]`
  bookkeeping (not in the Rust public API) emulated with `HashSet<i32>` term,
  `HashMap<i32,i32>` nchild, and an edge set to detect creation.
- **Unit tests** (5): (a) `new(0,0)` regulates to 1/1 and works;
  (b) `new(i32::MAX, 256)` -> `Err(ChTrieError::Erange)`;
  (c) `del` of nonexistent edge is a no-op;
  (d) pool exhaustion with `new(4,256)`: 3 children (1,2,3), 4th create -> -1,
      del child 2, next create reuses index 2 (LIFO);
  (e) `walk` hit/miss semantics.
- **Bug found & fixed**: `new(0,0)` -> n regulated to 1 -> `ecap = 0` ->
  `hash` divided by zero (panic). C has the same latent UB (`% 0`). Fixed by
  clamping `ecap` to >= 1 in `ChTrie::new` (no behavior change for n >= 2).
- **`cargo test` output** (exit status 0):
  ```
  running 5 tests
  test tests::del_nonexistent_edge_is_noop ... ok
  test tests::new_huge_n_returns_erange ... ok
  test tests::pool_exhaustion_and_lifo_reuse ... ok
  test tests::new_regulates_zero_to_one ... ok
  test tests::walk_hit_miss_semantics ... ok
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  running 1 test
  test test_chtrie ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

  Doc-tests chtrie
  running 0 tests
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- **Totals**: 6 tests, 6 passed, 0 failed. Exit status 0.
- **Semantic mismatches C vs Rust**:
  1. `n == 1` (incl. `new(0,0)`): C `chtrie_walk` divides by zero (UB);
     Rust clamps `ecap` to 1 so the trie is usable.
  2. C signals ENOMEM/ERANGE via `errno`; Rust returns `Result`/`-1`
     (`Enomem` is declared but `new` cannot fail allocation — Rust `Vec`
     aborts on OOM instead of returning ENOMEM).
  3. C `term[]`/`nchild[]` live in the struct; Rust API exposes only
     new/walk/del, so the test emulates that bookkeeping externally.

## Final verification

- `cargo build`: exit 0, no warnings (fresh recompile after touching sources).
- `cargo test`: exit 0.
  - Unit tests (src/lib.rs): 5 passed, 0 failed —
    `del_nonexistent_edge_is_noop`, `new_huge_n_returns_erange`,
    `new_regulates_zero_to_one`, `pool_exhaustion_and_lifo_reuse`,
    `walk_hit_miss_semantics`.
  - Integration test (tests/test.rs): `test_chtrie` passed (1 passed, 0 failed).
  - Doc-tests: 0.
- tests/test.rs confirmed: exactly the 14 C test cases
  (hello,the,his,he,his,go,he,a,an,this,that,hey,she,hers) with expected
  results 0,0,1,1,1,0,1,0,0,1,1,0,1,1; `add` walks creat=true per char and
  marks the final node as a term; `del` walks creat=0, unmarks the term, and
  prunes the edge via `del()` while nchild==0; `query` walks creat=0 and
  returns 1 iff the final node is a term.
- src/lib.rs public API confirmed: `ChTrie::new`, `ChTrie::walk`,
  `ChTrie::del`, `ChTrieError`, `Edge`, and the `Drop` impl (hash is
  private). The ecap clamp for n==1 (`.max(1)`) is present and documented
  in the `new` doc comment and inline comment.
- Verdict: PASS.
