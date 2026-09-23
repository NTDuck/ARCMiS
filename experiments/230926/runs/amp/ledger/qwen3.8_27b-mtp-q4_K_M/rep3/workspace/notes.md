# Notes

## Findings

- (planning) Workspace was empty at planning time: src/amp.c, src/amp.h,
  tests/test.c, Makefile, Readme.md, package.json were NOT present. The plan
  (plan.md) is written against the API surface described in the task
  (amp_t, amp_encode, amp_decode, amp_decode_arg with a mutable cursor,
  version/argc nibble header). Task t2 must verify all assumptions against
  the actual C sources before implementation.
- C sources were missing and have been restored from the authoritative spec;
  previous plan's assumed enum API was wrong — real API is the simple argv
  protocol.
- (ledger) Restored src/amp.h, src/amp.c, tests/test.c, Makefile, Readme.md,
  package.json exactly per the authoritative spec. Verified: C compiles
  clean with `gcc -Wall -Wextra -O2 -g` and the test binary prints `ok`.
  Rewrote plan.md to describe the real argv protocol (header byte
  version/argc nibbles + per-arg u32be length-prefixed bytes) and the
  corresponding Rust API (AMP_VERSION, Amp struct with buf/pos cursor,
  amp_encode, amp_decode, amp_decode_arg). No Rust code written yet.
  Updated tasks.json: planning item marked done, translate/tests/verify
  remain todo.
- (ledger) Rust translation complete. Created Cargo.toml (package `amp`
  v0.0.1, edition 2021, MIT, no deps), src/lib.rs (AMP_VERSION, Amp struct
  with version/argc/buf/pos + Default/new, amp_encode, amp_decode,
  amp_decode_arg — u32be framing, version/argc nibble header, cursor
  advancement mirroring the C code), and tests/test.rs (1:1 port of
  tests/test.c). `cargo build` finishes with no warnings; `cargo test`
  output summary:
    - unittests src/lib.rs: 0 tests, ok
    - tests/test.rs: 1 test, `test ... ok` — "test result: ok. 1 passed;
      0 failed; 0 ignored; 0 measured; 0 filtered out"
    - Doc-tests amp: 0 tests, ok
  Deviation: added an empty `[workspace]` table to Cargo.toml because the
  workspace root sits inside a parent Cargo workspace
  (/home/ayin/projs/ARCMiS/Cargo.toml); without it cargo refuses to build.
  tasks.json: t1–t5 marked done.
- (verification) Independent verification pass, all green:
  - File inventory confirmed: Cargo.toml, src/lib.rs, tests/test.rs,
    src/amp.c, src/amp.h, tests/test.c, plan.md, tasks.json, notes.md
    (plus Makefile, Readme.md, package.json, Cargo.lock) all present.
  - Fresh `cargo clean && cargo build`: exit 0, no warnings.
  - Fresh `cargo test` (before parity test): all suites ok —
    unittests 0 passed; tests/test.rs "test ... ok" (1 passed);
    Doc-tests 0 passed.
  - Byte-level parity: added tests/parity.rs (kept, strengthens suite)
    asserting amp_encode(["some","stuff","here"]) == exactly
    0x13, 00 00 00 04 "some", 00 00 00 05 "stuff", 00 00 00 04 "here",
    and that amp_decode yields version==1, argc==3, three amp_decode_arg
    calls return "some","stuff","here" in order with cursor fully
    consumed (pos == buf.len()). Both parity tests pass.
  - Final `cargo test` summary lines:
      tests/parity.rs: "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
      tests/test.rs:   "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
      unittests:       "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
      Doc-tests amp:   "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
  - Cross-check: C reference (tests/test.c + src/amp.c) compiles clean
    with gcc -Wall -Wextra -O2 -g and prints "ok".
  - No fixes were required; no code was modified. tasks.json is
    consistent: t0–t5 all "done", matching the actual workspace state.
