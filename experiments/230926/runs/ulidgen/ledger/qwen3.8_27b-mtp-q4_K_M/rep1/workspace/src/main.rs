//! ulidgen - generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to the
//! original C work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut n: i64 = 1;
    let mut tflag = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("ulidgen: option -n requires an argument");
                    std::process::exit(2);
                }
                match args[i].parse::<i64>() {
                    Ok(v) => n = v,
                    Err(_) => {
                        eprintln!("ulidgen: invalid number for -n: {}", args[i]);
                        std::process::exit(2);
                    }
                }
            }
            "-t" => tflag = true,
            other => {
                eprintln!("ulidgen: unknown option {other}");
                eprintln!("usage: ulidgen [-n N | -t]");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let mut gen = ulidgen::UlidGen::new();
    let mut stdout = io::stdout();
    let mut err = false;

    if tflag {
        // C uses getdelim: each line is printed *including* its trailing
        // newline (if present). read_line preserves that exactly.
        let mut reader = io::stdin().lock();
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let ulid = gen.next();
                    if write!(stdout, "{ulid} {line}").is_err() {
                        err = true;
                        break;
                    }
                }
                Err(_) => {
                    err = true;
                    break;
                }
            }
        }
    } else {
        for _ in 0..n {
            let ulid = gen.next();
            if writeln!(stdout, "{ulid}").is_err() {
                err = true;
                break;
            }
        }
    }

    if stdout.flush().is_err() {
        err = true;
    }
    // C: exit(!!ferror(stdout));
    std::process::exit(if err { 1 } else { 0 });
}
