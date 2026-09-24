//! ulidgen — generate or tag lines with ULID.
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! To the extent possible under law, the creator of this work has waived all
//! copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use clap::Parser;
use std::io::{self, BufRead, LineWriter, Write};

/// Generate or tag lines with ULID
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Print N consecutive ULID (default: 1)
    #[arg(short = 'n', default_value_t = 1, value_parser = parse_long)]
    n: i64,

    /// Read lines from standard input, and prefix each line with a ULID
    #[arg(short = 't')]
    t: bool,
}

/// C used `atol`; accept the same (i64).
fn parse_long(s: &str) -> Result<i64, String> {
    s.parse::<i64>().map_err(|_| format!("invalid number: {s}"))
}

fn main() {
    let args = Args::parse();

    // One shared buffer: the same-millisecond increment logic in ulidgen_r
    // depends on the caller reusing the buffer (C semantics).
    let mut ulid = [0u8; 27];

    if args.t {
        // Line-buffered stdout, like setvbuf(stdout, 0, _IOLBF, 0).
        let stdout = io::stdout();
        let mut out = LineWriter::new(stdout.lock());
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("ulidgen: {e}");
                    std::process::exit(1);
                }
            };
            ulidgen::ulidgen_r(&mut ulid);
            let s = String::from_utf8(ulid[..26].to_vec()).unwrap();
            // C prints "%s %s" where the line keeps its trailing '\n';
            // lines() strips it, so writeln! restores it.
            if out.write_all(format!("{s} {line}\n").as_bytes()).is_err() {
                std::process::exit(1);
            }
        }
        if out.flush().is_err() {
            std::process::exit(1);
        }
    } else {
        for _ in 0..args.n {
            ulidgen::ulidgen_r(&mut ulid);
            let s = String::from_utf8(ulid[..26].to_vec()).unwrap();
            if writeln!(io::stdout(), "{s}").is_err() {
                std::process::exit(1);
            }
        }
    }

    // Mirrors `fflush(0); exit(!!ferror(stdout));` — 0 on success, 1 on error.
}
