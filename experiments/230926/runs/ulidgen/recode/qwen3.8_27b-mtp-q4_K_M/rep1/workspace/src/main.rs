//! ulidgen — generate or tag lines with ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Usage: ulidgen [-n N | -t]
//!    -n N   generate N ULID (default: 1)
//!    -t     print each line of standard input prefixed with a ULID
//!
//! Port of the public-domain C src/ulidgen.c.
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io;

use ulidgen::ulidgen_r;

/// Port of C `main(int argc, char *argv[])`.
///
/// - `-n N` (default 1, parsed like `atol` — `parse().unwrap_or(1)`);
/// - `-t` reads stdin lines and prints `<ULID> <line>` (newline re-appended,
///   since Rust `lines()` strips it while C `getdelim` kept it);
/// - exits nonzero on stdout write errors (mirrors `exit(!!ferror(stdout))`).
fn main() -> io::Result<()> {
    // TODO: implement (see design.md section 3, src/main.rs)
    let buf = [0u8; 27];
    let _ = (buf, ulidgen_r);
    todo!("main not yet implemented")
}
