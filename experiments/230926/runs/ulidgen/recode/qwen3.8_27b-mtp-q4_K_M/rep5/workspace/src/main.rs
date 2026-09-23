//! ulidgen — generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! Rust port of `src/ulidgen.c` from the public-domain C project
//! by Leah Neukirchen <leah@vuxu.org>.
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::{self, BufRead, Write};
use std::process;

const USAGE: &str = "usage: ulidgen [-n N | -t]\n\
                     -n N   generate N ULID (default: 1)\n\
                     -t     print each line of standard input prefixed with an ULID\n";

fn main() {
    // 1. Manual arg parsing (getopt "n:t" equivalent).
    let mut n: i64 = 1;
    let mut tag = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i >= args.len() {
                    eprint!("{}", USAGE);
                    process::exit(2);
                }
                match args[i].parse::<i64>() {
                    Ok(v) => n = v,
                    Err(_) => {
                        eprint!("{}", USAGE);
                        process::exit(2);
                    }
                }
            }
            "-t" => tag = true,
            other => {
                eprintln!("ulidgen: unknown option '{}'", other);
                eprint!("{}", USAGE);
                process::exit(2);
            }
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tag {
        // 2. -t mode: tag each stdin line with a ULID.
        let stdin = io::stdin();
        let mut last: Option<String> = None;
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    let ulid = ulidgen::ulidgen(last.as_deref());
                    if writeln!(out, "{} {}", ulid, line).is_err() {
                        process::exit(1);
                    }
                    // Line-buffering equivalent of setvbuf(_IOLBF).
                    if out.flush().is_err() {
                        process::exit(1);
                    }
                    last = Some(ulid);
                }
                Err(e) => {
                    eprintln!("ulidgen: {}", e);
                    process::exit(1);
                }
            }
        }
    } else {
        // 3. -n mode: generate n ULIDs.
        let mut last: Option<String> = None;
        for _ in 0..n {
            let ulid = ulidgen::ulidgen(last.as_deref());
            if writeln!(out, "{}", ulid).is_err() {
                process::exit(1);
            }
            last = Some(ulid);
        }
    }

    // 4. Final flush; exit(1) on error (equivalent of exit(!!ferror(stdout))).
    if out.flush().is_err() {
        process::exit(1);
    }
}
