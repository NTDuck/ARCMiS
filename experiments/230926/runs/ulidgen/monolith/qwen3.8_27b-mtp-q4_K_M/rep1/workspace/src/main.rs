/*
 * ulidgen - generate or tag lines with ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * Usage: ulidgen [-n N | -t]
 *    -n N   generate N ULID (default: 1)
 *    -t     print each line of standard input prefixed with an ULID
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

use std::io::{self, BufRead, Write};

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
                if i < args.len() {
                    n = args[i].parse().unwrap_or(0);
                }
            }
            "-t" => tflag = true,
            _ => {}
        }
        i += 1;
    }

    let mut err = false;

    if tflag {
        let stdin = io::stdin();
        let mut out = io::BufWriter::new(io::stdout().lock());

        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    ulidgen::ulidgen_r(&mut ulid);
                    let s = String::from_utf8(ulid[..26].to_vec()).unwrap();
                    if writeln!(out, "{} {}", s, line).is_err() {
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
        if out.flush().is_err() {
            err = true;
        }
    } else {
        let mut out = io::stdout().lock();
        for _ in 0..n {
            ulidgen::ulidgen_r(&mut ulid);
            let s = String::from_utf8(ulid[..26].to_vec()).unwrap();
            if writeln!(out, "{}", s).is_err() {
                err = true;
                break;
            }
        }
        if out.flush().is_err() {
            err = true;
        }
    }

    std::process::exit(if err { 1 } else { 0 });
}
