//! CLI — translation of `src/ulidgen.c`.
//!
//! Usage: `ulidgen [-n N] [-t]`
//!   -n N   generate N ULIDs (default: 1)
//!   -t     prefix each stdin line with a ULID
//!
//! Exits 0 on success, non-zero on stdout error (C: `exit(!!ferror(stdout))`).

use std::io::{self, BufRead, Write};
use ulidgen::UlidGen;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse options (C: `getopt(argc, argv, "n:t")`).
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-n" => {
                // C: `atol` (lenient); fall back to 1 on bad input.
                n = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            }
            "-t" => {
                tflag = true;
            }
            _ => {
                eprintln!("ulidgen: unknown option {a}");
                std::process::exit(2);
            }
        }
    }

    let mut gen = UlidGen::new();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        // For each line of stdin (C: `getdelim`), print `"<ULID> <line>"`
        // (C: `printf("%s %s", ...)`; C uses `setvbuf(_IOLBF)` for line
        // buffering — the locked stdout + per-line `writeln!` is equivalent).
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    let _ = writeln!(out, "{} {}", gen.next(), l);
                }
                Err(_) => break,
            }
        }
    } else {
        // Print `n` ULIDs, one per line (C: `puts(ulid)`).
        for _ in 0..n.max(0) {
            let _ = writeln!(out, "{}", gen.next());
        }
    }

    // C: `fflush(0); exit(!!ferror(stdout));`
    let ok = out.flush().is_ok();
    std::process::exit(if ok { 0 } else { 1 });
}
