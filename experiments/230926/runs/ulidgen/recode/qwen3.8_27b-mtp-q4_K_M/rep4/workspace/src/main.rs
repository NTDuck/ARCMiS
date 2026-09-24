//! ulidgen — generate or tag lines with ULID.
//!
//! Port of `src/ulidgen.c`.
//!
//! Usage: `ulidgen [-n N | -t]`
//!   -n N   generate N ULIDs (default: 1)
//!   -t     print each line of standard input prefixed with a ULID
//!
//! Exit status: 0 on success, 1 if a stdout write failed
//! (mirrors C `exit(!!ferror(stdout))`).

use std::io::{self, BufRead, Write};
use std::process;

use ulidgen::ulidgen_r;

fn usage() {
    eprintln!("usage: ulidgen [-n N | -t]");
}

fn main() {
    // getopt("n:t") -> manual argument parsing.
    let mut n: i64 = 1;
    let mut tag = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" => match args.next() {
                Some(value) => match value.parse::<i64>() {
                    Ok(value) => n = value,
                    Err(_) => {
                        usage();
                        process::exit(1);
                    }
                },
                None => {
                    usage();
                    process::exit(1);
                }
            },
            "-t" => tag = true,
            _ => {
                usage();
                process::exit(1);
            }
        }
    }

    let mut stdout = io::stdout();
    let mut ulid = [0u8; 27];

    if tag {
        // setvbuf(stdout, _IOLBF) -> explicit flush() after each line.
        let mut stdin = io::stdin().lock();
        let mut line = String::new();
        loop {
            line.clear();
            // getdelim keeps the trailing '\n'; read_line does too.
            match stdin.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }
            ulidgen_r(&mut ulid);
            let id = std::str::from_utf8(&ulid[..26]).expect("ULID is ASCII");
            if write!(stdout, "{} {}", id, line).is_err() {
                process::exit(1);
            }
            if stdout.flush().is_err() {
                process::exit(1);
            }
        }
        return;
    }

    // puts per ULID.
    for _ in 0..n {
        ulidgen_r(&mut ulid);
        let id = std::str::from_utf8(&ulid[..26]).expect("ULID is ASCII");
        if writeln!(stdout, "{}", id).is_err() {
            process::exit(1);
        }
    }
}
