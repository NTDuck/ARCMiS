//! ulidgen — generate or tag lines with ULID.
//!
//! Port of `src/ulidgen.c`.
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use clap::Parser;
use std::io::{BufRead, Write};

/// generate or tag lines with ULID
#[derive(Parser)]
#[command(name = "ulidgen", version, about)]
struct Cli {
    /// print N consecutive ULID
    #[arg(short = 'n', long = "count", default_value_t = 1)]
    count: u64,

    /// read lines from standard input, and prefix each line with a ULID
    #[arg(short = 't', long = "tag")]
    tag: bool,
}

fn main() {
    let cli = Cli::parse();

    let mut out = std::io::stdout();

    // One persistent buffer across all calls (as C's `char ulid[27]`),
    // so the same-millisecond increment path triggers.
    let mut ulid = [0u8; 27];

    if cli.tag {
        // setvbuf(stdout, NULL, _IOLBF, 0) → flush after each line.
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("ulidgen: {e}");
                    std::process::exit(1);
                }
            };
            ulidgen::ulidgen_r(&mut ulid);
            let ulid_str = String::from_utf8_lossy(&ulid[..26]);
            // lines() strips the trailing newline; re-append it to match
            // C getdelim output byte-for-byte: printf("%s %s", ulid, line).
            if writeln!(out, "{ulid_str} {line}").is_err()
                || out.flush().is_err()
            {
                eprintln!("ulidgen: write error");
                std::process::exit(1);
            }
        }
    } else {
        for _ in 0..cli.count {
            ulidgen::ulidgen_r(&mut ulid);
            let ulid_str = String::from_utf8_lossy(&ulid[..26]);
            if writeln!(out, "{ulid_str}").is_err() || out.flush().is_err() {
                eprintln!("ulidgen: write error");
                std::process::exit(1);
            }
        }
    }
}
