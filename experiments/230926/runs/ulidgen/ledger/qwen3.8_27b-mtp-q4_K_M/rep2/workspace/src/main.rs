//! ulidgen - generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Faithful port of `src/ulidgen.c`.
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with an ULID

use std::io::{BufRead, Write};
use std::process::ExitCode;

use ulidgen::ulidgen_r;

/// `atol`-like parse: skip leading whitespace, optional sign, then digits;
/// trailing garbage is ignored; no digits means 0.
fn atol(s: &str) -> i64 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut sign = 1i64;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if b[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((b[i] - b'0') as i64);
        i += 1;
    }
    sign * v
}

fn main() -> ExitCode {
    let mut n: i64 = 1;
    let mut tflag = false;

    // getopt(argc, argv, "n:t")-style parsing:
    //   -n N, -nN (attached), -t, combined clusters like -tn 3.
    // Unknown options: getopt prints its own error to stderr and the C
    // switch has no default case, so they are silently skipped.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.len() > 1 && arg.starts_with('-') {
            let chars: Vec<char> = arg[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                match chars[j] {
                    'n' => {
                        if j + 1 < chars.len() {
                            // attached form: -nN
                            n = atol(chars[j + 1..].iter().collect::<String>().as_str());
                            j = chars.len();
                        } else if i + 1 < args.len() {
                            i += 1;
                            n = atol(&args[i]);
                        } else {
                            eprintln!("ulidgen: option requires an argument -- 'n'");
                        }
                    }
                    't' => {
                        tflag = true;
                    }
                    c => {
                        eprintln!("ulidgen: invalid option -- '{}'", c);
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }

    let mut out = std::io::stdout().lock();
    let mut err = false;

    if tflag {
        // C: setvbuf(stdout, 0, _IOLBF, 0) — Rust stdout is already
        // line-buffered on a tty; we flush at the end below.
        let mut reader = std::io::stdin().lock();
        let mut line = String::new();
        // One reused buffer, like the C `char ulid[27]` (shared state).
        let mut ulid = [0u8; 27];
        loop {
            line.clear();
            // read_line keeps the trailing newline, like getdelim.
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulidgen_r(&mut ulid);
                    let ulid_str = std::str::from_utf8(&ulid[0..26]).unwrap();
                    // printf("%s %s", ulid, line)
                    if out.write_all(ulid_str.as_bytes()).is_err()
                        || out.write_all(b" ").is_err()
                        || out.write_all(line.as_bytes()).is_err()
                    {
                        err = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        let mut ulid = [0u8; 27];
        for _ in 0..n.max(0) {
            ulidgen_r(&mut ulid);
            // puts(ulid)
            if out.write_all(std::str::from_utf8(&ulid[0..26]).unwrap().as_bytes()).is_err()
                || out.write_all(b"\n").is_err()
            {
                err = true;
                break;
            }
        }
    }

    // fflush(0); exit(!!ferror(stdout));
    if out.flush().is_err() {
        err = true;
    }
    if err {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
