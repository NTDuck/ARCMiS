//! CLI mirroring the C `src/ulidgen.c`:
//!
//! ```text
//! usage: ulidgen [-n N | -t]
//!   -n N  print N consecutive ULIDs (default 1)
//!   -t    read lines from stdin, prefix each line with a ULID + space
//! ```
//!
//! Exits 0 on success, 1 on error.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!("usage: ulidgen [-n N | -t]");
    std::process::exit(1);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut count: u64 = 1;
    let mut tail_mode = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-n" => match it.next().and_then(|s| s.parse::<u64>().ok()) {
                Some(n) => count = n,
                None => {
                    eprintln!("ulidgen: -n requires a numeric argument");
                    return ExitCode::from(1);
                }
            },
            "-t" => tail_mode = true,
            _ => {
                eprintln!("ulidgen: unknown argument: {arg}");
                usage();
            }
        }
    }

    if tail_mode {
        // Read lines from stdin, prefix each with a ULID + space,
        // line-buffered (flush after each line).
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut ulid = [b'0'; 27];
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    ulidgen::ulidgen_r(&mut ulid);
                    let ulid_str =
                        std::str::from_utf8(&ulid[..26]).expect("ulid is ascii");
                    if out
                        .write_all(ulid_str.as_bytes())
                        .and_then(|_| out.write_all(b" "))
                        .and_then(|_| out.write_all(line.as_bytes()))
                        .and_then(|_| out.write_all(b"\n"))
                        .and_then(|_| out.flush())
                        .is_err()
                    {
                        return ExitCode::from(1);
                    }
                }
                Err(e) => {
                    eprintln!("ulidgen: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        return ExitCode::SUCCESS;
    }

    // -n N mode: print N consecutive ULIDs (reusing one buffer, as in C).
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut ulid = [b'0'; 27];
    for _ in 0..count {
        ulidgen::ulidgen_r(&mut ulid);
        let ulid_str = std::str::from_utf8(&ulid[..26]).expect("ulid is ascii");
        if out
            .write_all(ulid_str.as_bytes())
            .and_then(|_| out.write_all(b"\n"))
            .and_then(|_| out.flush())
            .is_err()
        {
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}
