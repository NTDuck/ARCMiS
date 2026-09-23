//! ulidgen CLI — generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Rust port of `src/ulidgen.c` (public domain, Leah Neukirchen).
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::{self, BufRead, Write};

use ulidgen_r::ulidgen_r;

/// `atol` semantics: skip leading whitespace, optional sign, parse leading
/// digits, stop at the first non-digit (0 if none).
fn atol(s: &str) -> i64 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut sign = 1i64;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if b[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((b[i] - b'0') as i64);
        i += 1;
    }
    sign * v
}

fn main() {
    // getopt(argc, argv, "n:t") — the C switch has no default case, so
    // unknown options are silently ignored.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut n: i64 = 1;
    let mut tflag = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                if i + 1 < args.len() {
                    n = atol(&args[i + 1]);
                    i += 1;
                }
                // `-n` without a value: getopt would error and optarg would
                // be NULL; keep it simple and keep the default of 1.
            }
            "-t" => tflag = true,
            _ => {} // unknown option: ignored (no default case in C)
        }
        i += 1;
    }

    // char ulid[27] = { 0 }; — one buffer reused across all calls
    let mut ulid = [0u8; 27];

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut err = false;

    if tflag {
        // setvbuf(stdout, 0, _IOLBF, 0): Rust's stdout is line-buffered on a
        // tty by default; we flush at the end (and check for errors).
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut line = String::new();
        // getdelim(&line, &len, '\n', stdin): lines keep their trailing
        // newline; read_line preserves it exactly.
        loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    // printf("%s %s", ulid, line)
                    let ulid_str = String::from_utf8_lossy(&ulid[..26]);
                    if out.write_all(ulid_str.as_bytes()).is_err()
                        || out.write_all(b" ").is_err()
                        || out.write_all(line.as_bytes()).is_err()
                    {
                        err = true;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        // for (i = 0; i < n; i++) { ulidgen_r(ulid); puts(ulid); }
        for _ in 0..n {
            ulidgen_r(&mut ulid);
            let ulid_str = String::from_utf8_lossy(&ulid[..26]);
            if out.write_all(ulid_str.as_bytes()).is_err()
                || out.write_all(b"\n").is_err()
            {
                err = true;
            }
        }
    }

    // fflush(0); exit(!!ferror(stdout));
    if out.flush().is_err() {
        err = true;
    }
    std::process::exit(if err { 1 } else { 0 });
}
