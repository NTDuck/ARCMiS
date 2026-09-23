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

use std::io::{self, BufRead, LineWriter, Write};

use ulidgen::ulidgen_r;

fn main() {
    let mut ulid = [0u8; 27];

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut n: i64 = 1;
    let mut tflag = false;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(rest) = a.strip_prefix('-') {
            if rest == "n" {
                i += 1;
                if i < args.len() {
                    n = args[i].parse().unwrap_or(0);
                }
            } else if let Some(v) = rest.strip_prefix('n') {
                n = v.parse().unwrap_or(0);
            } else if rest == "t" {
                tflag = true;
            }
        }
        i += 1;
    }

    // line-buffered output, like setvbuf(stdout, 0, _IOLBF, 0)
    let mut out = LineWriter::new(io::stdout());

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    ulidgen_r(&mut ulid);
                    let u = std::str::from_utf8(&ulid[..26]).unwrap();
                    if out.write_fmt(format_args!("{} {}\n", u, line)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n.max(0) {
            ulidgen_r(&mut ulid);
            let u = std::str::from_utf8(&ulid[..26]).unwrap();
            if out.write_fmt(format_args!("{}\n", u)).is_err() {
                break;
            }
        }
    }

    // exit(!!ferror(stdout))
    let ok = out.flush().is_ok();
    std::process::exit(if ok { 0 } else { 1 });
}
