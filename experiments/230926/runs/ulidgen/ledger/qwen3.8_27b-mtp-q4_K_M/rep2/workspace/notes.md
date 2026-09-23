# notes.md — ledger worker log

## 2026-09-23 — planning/brainstorm task (C → Rust, ulidgen)

- Workspace was initially empty; located the C sources via the run manifest
  (`assets/ReCodeAgent/data/tool_projects/crust/ulidgen/c`) and copied them
  into the workspace root (Makefile, README, coverage_report.json, src/, tests/).
- Read all sources: `src/ulid.h`, `src/ulid.c`, `src/ulidgen.c`, `tests/test.c`.
- Wrote `plan.md`: translation strategy (std-only Cargo package, caller-owned
  buffer state model, lib + bin + integration test) and 8 core difficulties
  with chosen approaches (getentropy → /dev/urandom + abort; clock_gettime →
  SystemTime ms; nanosleep → thread::sleep 1_234_567 ns; in-place increment
  with usize underflow guards; `same` flag kept buffer-based; getopt → manual
  parsing incl. `-nN` attached form; commented-out structure test ported as an
  active passing test; CLI I/O: read_line to keep newlines, exit(1) on
  ferror-equivalent).
- Wrote `tasks.json`: 5 pending seed tasks (scaffold, core port, CLI port,
  test port, verify parity).
- No code implemented (planning task only).

## Ledger worker — scaffold + core port (this round)

- Reported initial workspace state (files, tasks.json, plan.md) to curator.
- Created `Cargo.toml` (package `ulidgen`, edition 2021, zero dependencies).
  Note: added an empty `[workspace]` table because the workspace root is
  nested inside a parent cargo workspace at /home/ayin/projs/ARCMiS/Cargo.toml;
  without it `cargo build` fails with "current package believes it's in a
  workspace when it's not".
- Created `src/lib.rs` with `pub fn ulidgen_r(ulid: &mut [u8; 27])`, a faithful
  port of src/ulid.c: base32 alphabet, NUL at ulid[26], ms-since-epoch via
  SystemTime (truncated, `as u64`), big-endian 10-digit encode with `same`
  flag, in-place increment of ulid[10..26] (trailing 'Z'→'0', usize underflow
  guarded; all-Z ⇒ sleep 1_234_567 ns + recurse), fall-through re-randomize
  on invalid char, 16 bytes from /dev/urandom via OpenOptions + read_exact
  with abort() on failure, `alphabet[byte % 32]` encoding.
- `cargo build`: clean (one unused-import warning fixed).
- Ran a temporary sanity test (NUL terminator, 26 alphabet-valid chars,
  same-ms increment, all-Z rollover) — passed; temp test removed (official
  port is t4).
- tasks.json: t2-core → done (verified). t1-scaffold left pending: Cargo.toml
  and src/lib.rs exist, but src/main.rs (t3) and tests/test.rs (t4) are not
  yet created.
- Ported CLI src/ulidgen.c to src/main.rs (t3-cli):
  - getopt("n:t")-style manual parsing: `-n N`, attached `-nN`, `-t`, combined
    clusters (e.g. `-tn 3`); `atol`-like N parse (whitespace, sign, digits,
    trailing garbage ignored, 0 if none); unknown options print a brief
    stderr message and are skipped (C has no default case; exit behavior
    unchanged).
  - `-t` mode: read stdin with BufRead::read_line (keeps trailing newline,
    like getdelim), one reused [u8; 27] buffer (shared state like C
    `char ulid[27]`), prints "{ulid} {line}" (26 chars, no NUL).
  - `-n` mode: loop n times (negative n → 0 iterations, like C long loop),
    puts-style "ulid\n".
  - End: flush stdout; exit(1) if any write/flush error (C: exit(!!ferror(stdout))), else 0.
- `cargo build`: clean (Compiling ulidgen v0.1.0 ... Finished dev profile).
- Smoke tests:
  - `cargo run -- -n 3` → 3 lines, all 26 chars, all match
    ^[0-9A-HJ-NP-TV-Z]{26}$, all unique, consecutive ones differ
    (…MC/…MD/…ME same-ms increment); exit 0.
  - `printf 'a\nb\n' | cargo run -- -t` → both lines tagged with unique
    ULIDs; exit 0.
  - `-n3` attached form → 3 lines; `-x` → stderr "invalid option -- 'x'",
    still runs default (-n 1) and exits 0; no args → 1 ULID.
- tasks.json: t1-scaffold → done (Cargo.toml + src/lib.rs + src/main.rs +
  tests/ dir exist), t3-cli → done (verified).

## Ledger worker — test port (t4-tests, this round)

- Created `tests/test.rs` porting `tests/test.c`:
  - `is_valid_ulid(ulid: &str) -> bool`: length 26 + every char in
    "0123456789ABCDEFGHJKMNPQRSTVWXYZ".
  - `test_ulid_length`: one ULID, assert 26 chars.
  - `test_ulid_structure`: active (commented out in C main, but plan says
    port as active test); asserts is_valid_ulid, prints generated ULID.
  - `test_ulid_uniqueness`: two separate `[u8; 27]` buffers (like C),
    assert they differ.
  - `test_ulid_sortability`: ulid1, `thread::sleep(Duration::from_nanos(1_500_000))`,
    ulid2, assert `ulid1 < ulid2` lexicographically.
  - Uses `ulidgen::ulidgen_r`; 27-byte buffer converted to 26-char string
    via `String::from_utf8_lossy(&buf[0..26])`.
- `cargo test` (first run): all green, no fixes needed.
  - lib unittests: 0 tests, ok.
  - main unittests: 0 tests, ok.
  - tests/test.rs: 4 passed; 0 failed (test_ulid_length, test_ulid_structure,
    test_ulid_uniqueness, test_ulid_sortability — all ok).
  - Doc-tests: 0 tests, ok.
- tasks.json: t4-tests → done. t5-verify still pending.

## t5-verify (verification worker)
- `cargo build` (fresh, after touch): `Compiling ulidgen v0.1.0 ... Finished dev profile ... in 0.51s`, exit 0, **no warnings**.
- `cargo test`: 4 passed; 0 failed (test_ulid_length, test_ulid_structure, test_ulid_uniqueness, test_ulid_sortability); lib/main/doc suites 0 tests; exit 0.
- C reference built: `gcc -O2 -o /tmp/ulidgen_c src/ulidgen.c src/ulid.c` OK.
- Parity:
  - `-n 3`: both 3 lines, all 26 chars, all valid Crockford base32 (grep -cE '^[0-9A-HJKMNP-TV-Z]{26}$' = 3/3), no duplicates, consecutive lines increment (e.g. C ...B8B/B8C/B8D, Rust ...RWD/RWE/RWF).
  - `printf 'x\ny\n' | -t`: both 2 lines matching `^[0-9A-HJKMNP-TV-Z]{26} [xy]$`; `od -c` shows trailing `\n` preserved in both.
  - default (no args): exactly 1 ULID each.
  - exit codes: 0 for all invocations (C and Rust).
- Cargo.toml: `[dependencies]` empty — no external deps.
- tasks.json: t5-verify marked done with evidence note.
