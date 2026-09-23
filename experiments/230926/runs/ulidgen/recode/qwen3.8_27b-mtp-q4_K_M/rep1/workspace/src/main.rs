//! `ulidgen` — generate or tag lines with ULID.
//!
//! Port of `src/ulidgen.c`.
//!
//! Usage: `ulidgen [-n N | -t]`
//!   -n N   generate N ULIDs (default: 1), one per line
//!   -t     print each line of standard input prefixed with a ULID
//!
//! Exits 0 on success, >0 if a write to stdout fails (mirrors
//! `exit(!!ferror(stdout))`).

fn main() {
    todo!("port of C main: manual parse of std::env::args for -n N (i64, default 1) and -t; -t mode: BufRead::read_line loop, reuse one [0u8; 27] buffer across ulidgen_r calls, write '<ulid> <line>'; -n mode: loop n times printing each ULID; final flush, exit(1) on any write/flush error")
}
