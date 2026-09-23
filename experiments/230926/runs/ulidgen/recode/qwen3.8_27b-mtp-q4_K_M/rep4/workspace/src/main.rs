//! ulidgen CLI — generate or tag lines with ULIDs.
//!
//! Mirrors C `src/ulidgen.c`:
//! - `-n N`  print N consecutive ULIDs (default 1);
//! - `-t`    prefix each stdin line with a ULID (line-buffered);
//! - exit 0 on success, non-zero on stdout error.

use std::env;
use std::io::{self, BufRead, Write};

fn usage_exit() -> ! {
    eprintln!("usage: ulidgen [-n N] [-t]");
    std::process::exit(2);
}

fn main() -> std::io::Result<()> {
    let mut n: i64 = 1;
    let mut tflag = false;

    // Manual getopt("n:t") equivalent.
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-n" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => usage_exit(),
                };
                n = match v.parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => usage_exit(),
                };
            }
            "-t" => tflag = true,
            _ => usage_exit(),
        }
    }

    let mut ulid = [0u8; 27];
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        // C: setvbuf(stdout, 0, _IOLBF, 0); getdelim loop printing "%s %s".
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            ulidgen::ulidgen_r(&mut ulid);
            out.write_all(&ulid[..26])?;
            out.write_all(b" ")?;
            out.write_all(line.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?; // line-buffered behavior
        }
    } else {
        // C: for (i = 0; i < n; i++) puts(ulid);
        for _ in 0..n {
            ulidgen::ulidgen_r(&mut ulid);
            out.write_all(&ulid[..26])?;
            out.write_all(b"\n")?;
        }
    }

    // C: fflush(0); exit(!!ferror(stdout));
    out.flush()?;
    Ok(())
}
