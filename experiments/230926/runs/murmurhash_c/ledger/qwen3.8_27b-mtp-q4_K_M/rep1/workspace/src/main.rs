//! `murmurhash` CLI - reads lines from stdin, hashes each line (trailing
//! newline stripped) with an optional `--seed=N` (default 0), and prints the
//! hash in decimal, one per line.
//!
//! This is a working CLI; it intentionally does NOT reproduce the bugs in the
//! C `main.c` (malformed `printf("%d" PRIu32, ...)` format string and the
//! loop re-hashing `buf` instead of `key`).

use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

const VERSION: &str = "0.2.0";

fn print_help() {
    let out = io::stdout();
    let mut lock = out.lock();
    let _ = writeln!(
        lock,
        "murmurhash {VERSION}\n\
         \n\
         USAGE:\n\
         \x20   murmurhash [OPTIONS]\n\
         \n\
         Reads lines from stdin and prints the murmurhash of each line in\n\
         decimal, one per line.\n\
         \n\
         OPTIONS:\n\
         \x20   --seed=N    hash seed (default 0)\n\
         \x20   -h, --help  print this help text\n\
         \x20   -V, --version  print version information"
    );
}

fn parse_seed(args: &[String]) -> Result<u32, String> {
    let mut seed: u32 = 0;
    for arg in args {
        if let Some(value) = arg.strip_prefix("--seed=") {
            seed = value
                .parse::<u32>()
                .map_err(|_| format!("invalid seed '{}': expected an unsigned 32-bit integer", value))?;
        } else if arg == "--seed" {
            return Err("--seed requires a value: use --seed=N".to_string());
        } else if arg == "-h" || arg == "--help" {
            print_help();
            std::process::exit(0);
        } else if arg == "-V" || arg == "--version" {
            println!("murmurhash {VERSION}");
            std::process::exit(0);
        } else {
            return Err(format!("unrecognized argument '{}'", arg));
        }
    }
    Ok(seed)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let seed = match parse_seed(&args) {
        Ok(seed) => seed,
        Err(message) => {
            let err = io::stderr();
            let mut lock = err.lock();
            let _ = writeln!(lock, "murmurhash: {}", message);
            let _ = writeln!(lock, "try 'murmurhash --help'");
            return ExitCode::from(2);
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                // hash the line bytes, trimmed of the trailing newline
                let key = line.as_bytes();
                let hash = murmurhash::murmurhash(key, seed);
                if writeln!(out, "{}", hash).is_err() {
                    return ExitCode::FAILURE;
                }
            }
            Err(e) => {
                let err = io::stderr();
                let mut lock = err.lock();
                let _ = writeln!(lock, "murmurhash: error reading stdin: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}
