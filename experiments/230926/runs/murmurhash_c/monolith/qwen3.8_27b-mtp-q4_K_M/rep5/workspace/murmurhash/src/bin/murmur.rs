//! `murmur` - murmurhash command line utility
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use std::env;
use std::io::{self, BufRead, Write};
use std::process;

fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

fn help() {
    eprintln!("\noptions:");
    eprintln!("\n  --seed=[seed]  hash seed (optional)");
    eprintln!("\n");
}

fn main() {
    let mut seed: u32 = 0;

    // parse opts
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let opt = &args[i];
        if opt == "-h" || opt == "--help" {
            usage();
            help();
            process::exit(0);
        } else if opt == "-V" || opt == "--version" {
            eprintln!("{}", murmurhash::VERSION);
            process::exit(0);
        } else if let Some(value) = opt.strip_prefix("--seed=") {
            match value.parse::<u32>() {
                Ok(v) => seed = v,
                Err(_) => {
                    eprintln!("invalid seed: `{}`", value);
                    usage();
                    process::exit(1);
                }
            }
        } else if opt == "--seed" {
            i += 1;
            if i >= args.len() {
                eprintln!("missing value for `--seed'");
                usage();
                process::exit(1);
            }
            match args[i].parse::<u32>() {
                Ok(v) => seed = v,
                Err(_) => {
                    eprintln!("invalid seed: `{}`", args[i]);
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
        i += 1;
    }

    // read lines from stdin and hash each one
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut out = io::stdout();
    while let Some(Ok(line)) = lines.next() {
        let h = murmurhash::murmurhash(line.as_bytes(), seed);
        if writeln!(out, "{}", h).is_err() {
            process::exit(1);
        }
    }

    process::exit(0);
}
