//! `main.rs` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::process;

use murmurhash::{murmurhash, MURMURHASH_VERSION};

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
    for opt in env::args().skip(1) {
        if opt == "-h" || opt == "--help" {
            usage();
            help();
            process::exit(0);
        } else if opt == "-V" || opt == "--version" {
            eprintln!("{}", MURMURHASH_VERSION);
            process::exit(0);
        } else if let Some(value) = opt.strip_prefix("--seed=") {
            match value.parse::<u32>() {
                Ok(v) => seed = Some(v),
                Err(_) => {
                    eprintln!("unknown option: `{}`", opt);
                    usage();
                    process::exit(1);
                }
            }
        } else {
            // error
            eprintln!("unknown option: `{}`", opt);
            usage();
            process::exit(1);
        }
    }

    let seed = seed.unwrap_or(0);

    if std::io::stdin().is_terminal() {
        process::exit(1);
    }

    let stdin = io::stdin();
    let lines = stdin.lock().lines();
    let mut any = false;

    for line in lines {
        match line {
            Ok(s) => {
                any = true;
                let h = murmurhash(s.as_bytes(), seed);
                println!("{}", h);
            }
            Err(_) => break,
        }
    }

    if !any {
        process::exit(1);
    }
}
