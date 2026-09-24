//! ulidgen — generate or tag lines with ULID.
//!
//! Port of `src/ulidgen.c` (public domain, Leah Neukirchen).
//!
//! Usage: `ulidgen [-n N | -t]`
//!   -n N   generate N ULID (default: 1)
//!   -t     print each line of standard input prefixed with an ULID
//!
//! Exit status: 0 on success, 1 on write/flush error (mirrors `exit(!!ferror(stdout))`).

use std::io::{self, BufRead, Write};

use ulidgen::ulidgen_r;

fn main() {
    let mut ulid = [0u8; 27];

    // manual parse of: -n N  and  -t   (mirrors getopt "n:t")
    let mut n: i64 = 1;
    let mut tflag = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-n" => {
                n = args
                    .next()
                    .expect("-n requires an argument")
                    .parse()
                    .unwrap_or(1);
            }
            "-t" => tflag = true,
            _ => {
                eprintln!("usage: ulidgen [-n N] [-t]");
                std::process::exit(1);
            }
        }
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        // tag mode: prefix each stdin line with a ULID.
        // Use BufRead::read_line (like C getdelim) so the trailing newline is
        // preserved and the output matches printf("%s %s", ulid, line) byte-for-byte.
        let mut input = String::new();
        let mut reader = stdin.lock();
        loop {
            input.clear();
            match reader.read_line(&mut input) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    let _ = write!(
                        out,
                        "{} {}",
                        std::str::from_utf8(&ulid[..26]).unwrap(),
                        input
                    );
                }
                Err(_) => break,
            }
        }
    } else {
        // generate mode: print n ULIDs
        for _ in 0..n.max(0) {
            ulidgen_r(&mut ulid);
            let _ = writeln!(out, "{}", std::str::from_utf8(&ulid[..26]).unwrap());
        }
    }

    // exit(!!ferror(stdout))
    let ok = out.flush().is_ok();
    std::process::exit(if ok { 0 } else { 1 });
}
