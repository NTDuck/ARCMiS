//! `murmur` - command line MurmurHash3 utility
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use std::io::{self, Read, Write};
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
    let mut seed: u32 = 0;
    let mut seed_set = false;

    // parse opts
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            usage();
            help();
            return;
        } else if arg == "-V" || arg == "--version" {
            eprintln!("{}", VERSION);
            return;
        } else if let Some(rest) = arg.strip_prefix("--seed=") {
            match rest.parse::<u32>() {
                Ok(v) => {
                    seed = v;
                    seed_set = true;
                }
                Err(_) => {
                    eprintln!("invalid seed: `{}`", rest);
                    usage();
                    process::exit(1);
                }
            }
        } else if arg == "--seed" {
            match args.next() {
                Some(v) => match v.parse::<u32>() {
                    Ok(n) => {
                        seed = n;
                        seed_set = true;
                    }
                    Err(_) => {
                        eprintln!("invalid seed: `{}`", v);
                        usage();
                        process::exit(1);
                    }
                },
                None => {
                    eprintln!("missing seed value");
                    usage();
                    process::exit(1);
                }
            }
        } else {
            // error
            eprintln!("unknown option: `{}`", arg);
            usage();
            process::exit(1);
        }
    }

    if !seed_set {
        seed = 0;
    }

    // read all of stdin
    let mut buf: Vec<u8> = Vec::new();
    if io::stdin().lock().read_to_end(&mut buf).is_err() {
        process::exit(1);
    }

    let h = murmurhash(&buf, seed);
    let mut out = io::stdout();
    let _ = writeln!(out, "{}", h);
}
