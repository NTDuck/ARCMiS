//! `murmur` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use std::io::{self, BufRead, IsTerminal};
use std::process;

use murmurhash::{murmurhash, VERSION};

fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

fn help() {
    eprintln!("\noptions:");
    eprintln!("\n  --seed=[seed]  hash seed (optional)");
    eprintln!("\n");
}

fn main() {
    let mut seed: Option<u32> = None;

    // parse opts
    for opt in std::env::args().skip(1) {
        // flags
        if let Some(rest) = opt.strip_prefix('-') {
            match rest {
                "h" => {
                    usage();
                    help();
                    return;
                }

                "V" => {
                    eprintln!("{VERSION}");
                    return;
                }

                r if r.starts_with('-') => {
                    let r = &r[1..];
                    if let Some(s) = r.strip_prefix("seed") {
                        if let Some(v) = s.strip_prefix('=') {
                            match v.parse::<u32>() {
                                Ok(n) => seed = Some(n),
                                Err(_) => {
                                    // error
                                    eprintln!("unknown option: `--seed={v}'");
                                    usage();
                                    process::exit(1);
                                }
                            }
                        }
                    }
                }

                _ => {
                    // error
                    eprintln!("unknown option: `-{rest}'");
                    usage();
                    process::exit(1);
                }
            }
        }
    }

    let seed = seed.unwrap_or(0);

    let stdin = io::stdin();

    if stdin.is_terminal() {
        process::exit(1);
    }

    let mut lines = stdin.lock().lines();

    while let Some(line) = lines.next() {
        match line {
            Ok(buf) => println!("{}", murmurhash(buf.as_bytes(), seed)),
            Err(_) => break,
        }
    }
}
