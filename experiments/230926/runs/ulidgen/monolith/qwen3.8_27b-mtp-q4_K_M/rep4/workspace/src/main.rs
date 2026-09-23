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
                if i >= args.len() {
                    eprintln!("ulidgen: option requires an argument -- 'n'");
                    std::process::exit(1);
                }
                n = args[i].parse::<i64>().unwrap_or(0);
            }
            "-t" => tflag = true,
            _ => {
                let c = args[i].chars().nth(1).unwrap_or('?');
                eprintln!("ulidgen: illegal option -- {c}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut err = false;

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    ulidgen::ulidgen_r(&mut ulid);
                    let ulid_str = String::from_utf8_lossy(&ulid[..26]);
                    if writeln!(out, "{ulid_str} {line}").is_err() {
                        err = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n.max(0) {
            ulidgen::ulidgen_r(&mut ulid);
            let ulid_str = String::from_utf8_lossy(&ulid[..26]);
            if writeln!(out, "{ulid_str}").is_err() {
                err = true;
                break;
            }
        }
    }

    if out.flush().is_err() {
        err = true;
    }

    // exit 0 on success, 1 if an error occurs
    std::process::exit(if err { 1 } else { 0 });
}
