//! `murmur` - murmurhash command line utility
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

use std::env;
use std::io::{self, Read, Write};

use murmurhash::{murmurhash, VERSION};

fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

fn help() {
    eprintln!("\noptions:");
    eprintln!("\n  --seed=[seed]  hash seed (optional)");
    eprintln!("\n");
}

/// Read one line from stdin (up to 1024 bytes, as in the C `read_stdin`).
/// Returns `None` on EOF or allocation failure.
fn read_stdin() -> Option<String> {
    let mut buf = [0u8; 1024];
    let n = io::stdin().lock().read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    let mut res = String::with_capacity(n + 1);
    res.push_str(std::str::from_utf8(&buf[..n]).ok()?);
    Some(res)
}

fn main() {
    let mut seed: Option<u32> = None;

    // parse opts
    for opt in env::args().skip(1) {
        if opt == "-h" {
            usage();
            help();
            return;
        } else if opt == "-V" {
            eprintln!("{}", VERSION);
            return;
        } else if let Some(value) = opt.strip_prefix("--seed=") {
            match value.parse::<u32>() {
                Ok(v) => seed = Some(v),
                Err(_) => {
                    eprintln!("unknown option: `{}`", opt);
                    usage();
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("unknown option: `{}`", opt);
            usage();
            std::process::exit(1);
        }
    }

    let seed = seed.unwrap_or(0);

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // hash each line read from stdin
    while let Some(line) = read_stdin() {
        let h = murmurhash(line.as_bytes(), seed);
        let _ = writeln!(out, "{}", h);
    }
}
