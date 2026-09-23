//! ulidgen - generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::{self, BufRead, Write};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut n: i64 = 1;
    let mut tflag = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        eprintln!("ulidgen: option -n requires an argument");
                        std::process::exit(1);
                    }
                };
                n = v.parse().unwrap_or(0);
            }
            "-t" => tflag = true,
            other => {
                eprintln!("ulidgen: unknown option: {other}");
                eprintln!("usage: ulidgen [-n N | -t]");
                std::process::exit(1);
            }
        }
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut prev = String::new();
    let mut err = false;

    if tflag {
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    let ulid = ulidgen::ulidgen_r(&prev);
                    prev = ulid;
                    if writeln!(out, "{} {}", prev, line).is_err() {
                        err = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n {
            let ulid = ulidgen::ulidgen_r(&prev);
            prev = ulid;
            if writeln!(out, "{}", prev).is_err() {
                err = true;
                break;
            }
        }
    }

    if out.flush().is_err() {
        err = true;
    }

    std::process::exit(if err { 1 } else { 0 });
}
