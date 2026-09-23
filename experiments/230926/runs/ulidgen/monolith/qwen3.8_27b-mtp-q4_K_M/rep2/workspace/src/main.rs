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

use std::io::{BufRead, Write};

use ulidgen::ulidgen_r;

fn main() {
    let mut ulid = [0u8; 27];

    let mut n: i64 = 1;
    let mut tflag = false;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if let Some(rest) = a.strip_prefix('-') {
            if !rest.is_empty() {
                match rest.as_bytes()[0] {
                    b'n' => {
                        let val = if rest.len() > 1 {
                            rest[1..].to_string()
                        } else {
                            i += 1;
                            if i >= args.len() {
                                eprintln!("ulidgen: option requires an argument -- 'n'");
                                std::process::exit(1);
                            }
                            args[i].clone()
                        };
                        n = val.parse::<i64>().unwrap_or(0);
                    }
                    b't' => tflag = true,
                    _ => {}
                }
            }
        }
        i += 1;
    }

    let mut err = false;
    if tflag {
        let mut r = std::io::stdin().lock();
        let mut out = std::io::stdout();
        let mut line = Vec::new();

        loop {
            line.clear();
            match r.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    let ulid_str = std::str::from_utf8(&ulid[..26]).unwrap();
                    if out.write_all(ulid_str.as_bytes()).is_err()
                        || out.write_all(b" ").is_err()
                        || out.write_all(&line).is_err()
                        || out.flush().is_err()
                    {
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
        let mut out = std::io::stdout();
        for _ in 0..n {
            ulidgen_r(&mut ulid);
            let ulid_str = std::str::from_utf8(&ulid[..26]).unwrap();
            if out.write_all(ulid_str.as_bytes()).is_err()
                || out.write_all(b"\n").is_err()
                || out.flush().is_err()
            {
                err = true;
                break;
            }
        }
    }

    std::process::exit(if err { 1 } else { 0 });
}
