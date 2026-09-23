//! ulidgen CLI — port of `src/ulidgen.c`.
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID
//!
//! Public domain (http://creativecommons.org/publicdomain/zero/1.0/).

use std::io::{BufRead, LineWriter, Write};

use ulidgen::ulid::ulidgen_r;

fn main() {
    let mut opts = getopts::Options::new();
    opts.optopt("n", "", "Print N consecutive ULID", "N");
    opts.optflag("t", "", "prefix stdin lines with a ULID");

    let matches = match opts.parse(std::env::args().skip(1)) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let n: i64 = matches
        .opt_str("n")
        .map(|s| s.parse::<i64>().unwrap_or(1))
        .unwrap_or(1);
    let tflag = matches.opt_present("t");

    // One reused buffer (critical for same-millisecond uniqueness).
    let mut ulid = [0u8; 27];

    if tflag {
        // Line-buffered stdout (C setvbuf _IOLBF).
        let stdout = std::io::stdout();
        let mut out = LineWriter::new(stdout.lock());
        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            match stdin.lock().read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    let ulid_str = std::str::from_utf8(&ulid[..26]).unwrap();
                    if out
                        .write_all(ulid_str.as_bytes())
                        .and_then(|_| out.write_all(b" "))
                        .and_then(|_| out.write_all(line.as_bytes()))
                        .is_err()
                    {
                        std::process::exit(1);
                    }
                    line.clear();
                }
                Err(_) => std::process::exit(1),
            }
        }
    } else {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for _ in 0..n {
            ulidgen_r(&mut ulid);
            let ulid_str = std::str::from_utf8(&ulid[..26]).unwrap();
            if out
                .write_all(ulid_str.as_bytes())
                .and_then(|_| out.write_all(b"\n"))
                .is_err()
            {
                std::process::exit(1);
            }
        }
    }
}
