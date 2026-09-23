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

use ulidgen::ulidgen_r;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut n: i64 = 1;
    let mut tflag = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i < args.len() {
                    /* atol-style: default to 1 on parse failure */
                    n = args[i].parse::<i64>().unwrap_or(1);
                }
            }
            "-t" => tflag = true,
            _ => {}
        }
        i += 1;
    }

    let mut ulid = [0u8; 27];
    let mut out = io::stdout().lock();
    let mut err = false;

    if tflag {
        let mut input = io::stdin().lock();
        let mut line = String::new();
        loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    let ulid_str = String::from_utf8_lossy(&ulid[..26]);
                    /* sequential writes, each checked; flush after each line */
                    if out.write_all(ulid_str.as_bytes()).is_err()
                        || out.write_all(b" ").is_err()
                        || out.write_all(line.as_bytes()).is_err()
                        || out.flush().is_err()
                    {
                        err = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n {
            ulidgen_r(&mut ulid);
            let ulid_str = String::from_utf8_lossy(&ulid[..26]);
            if out.write_all(ulid_str.as_bytes()).is_err()
                || out.write_all(b"\n").is_err()
            {
                err = true;
                break;
            }
        }
    }

    if out.flush().is_err() {
        err = true;
    }

    /* mirror exit(!!ferror(stdout)) */
    std::process::exit(if err { 1 } else { 0 });
}
