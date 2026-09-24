//! Integration tests for the CLI entry point (`main` in src/main.rs,
//! port of src/ulidgen.c). Drives the built binary via std::process::Command.

use std::process::{Command, Stdio};

use ulidgen::B32_ALPHABET;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ulidgen")
}

/// Same validity rule as tests/ulid.rs:is_valid_ulid.
fn is_valid_ulid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|b| B32_ALPHABET.contains(&b))
}

/// `ulidgen` with no arguments prints exactly one valid ULID line.
#[test]
fn test_cli_default_one_ulid() {
    let out = Command::new(bin())
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "exit status: {}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout:?}");
    assert!(is_valid_ulid(lines[0]), "not a valid ULID: {lines:?}");
    assert!(stdout.ends_with('\n'), "missing trailing newline: {stdout:?}");
}

/// `ulidgen -n 3` prints 3 valid, non-decreasing ULID lines.
#[test]
fn test_cli_n3() {
    let out = Command::new(bin())
        .arg("-n")
        .arg("3")
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "exit status: {}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "stdout: {stdout:?}");
    for l in &lines {
        assert!(is_valid_ulid(l), "not a valid ULID: {l:?}");
    }
    // Same-millisecond generation must be strictly increasing (in-place
    // increment of the random part), never equal or decreasing.
    assert!(lines[0] < lines[1], "not increasing: {lines:?}");
    assert!(lines[1] < lines[2], "not increasing: {lines:?}");
    assert!(stdout.ends_with('\n'), "missing trailing newline: {stdout:?}");
}

/// `ulidgen -n 0` prints nothing and exits 0.
#[test]
fn test_cli_n0() {
    let out = Command::new(bin())
        .arg("-n")
        .arg("0")
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "exit status: {}", out.status);
    assert!(out.stdout.is_empty(), "stdout: {:?}", String::from_utf8_lossy(&out.stdout));
}

/// `ulidgen -t` prefixes each stdin line with a valid ULID, preserving the
/// trailing newline (byte-for-byte `printf("%s %s", ulid, line)` semantics).
#[test]
fn test_cli_tag() {
    let mut child = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen");
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().expect("stdin");
        stdin
            .write_all(b"a\nb\n")
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait ulidgen");
    assert!(out.status.success(), "exit status: {}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "stdout: {stdout:?}");
    for (i, l) in lines.iter().enumerate() {
        let expected_line = ["a", "b"][i];
        let mut parts = l.splitn(2, ' ');
        let ulid_part = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        assert!(is_valid_ulid(ulid_part), "not a valid ULID: {l:?}");
        assert_eq!(rest, expected_line, "line mismatch: {l:?}");
    }
    assert!(stdout.ends_with('\n'), "missing trailing newline: {stdout:?}");
}
