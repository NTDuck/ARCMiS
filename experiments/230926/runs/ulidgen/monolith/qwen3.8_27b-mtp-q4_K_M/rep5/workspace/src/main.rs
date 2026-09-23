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

use ulidgen::ulidgen_r;

fn main() {
    let mut ulid = [0u8; 27];

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut n: i64 = 1;
    let mut tflag = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("ulidgen: option requires an argument -- 'n'");
                    std::process::exit(1);
                }
                match args[i].parse::<i64>() {
                    Ok(v) => n = v,
                    Err(_) => {
                        eprintln!("ulidgen: invalid number: {}", args[i]);
                        std::process::exit(1);
                    }
                }
            }
            "-t" => tflag = true,
            other => {
                eprintln!("ulidgen: illegal option -- {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let mut err = false;

    if tflag {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    ulidgen_r(&mut ulid);
                    let u = std::str::from_utf8(&ulid[..26]).expect("valid UTF-8");
                    if out.write_fmt(format_args!("{} {}\n", u, line)).is_err() {
                        err = true;
                        break;
                    }
                    /* line-buffered output, like setvbuf(stdout, 0, _IOLBF, 0) */
                    if out.flush().is_err() {
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
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for _ in 0..n {
            ulidgen_r(&mut ulid);
            let u = std::str::from_utf8(&ulid[..26]).expect("valid UTF-8");
            if out.write_fmt(format_args!("{}\n", u)).is_err() {
                err = true;
                break;
            }
        }
        if out.flush().is_err() {
            err = true;
        }
    }

    /* exit(!!ferror(stdout)) */
    std::process::exit(if err { 1 } else { 0 });
}
