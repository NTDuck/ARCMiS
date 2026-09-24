//! Integration tests for the `ulidgen` CLI (`fn main` in src/main.rs).
//!
//! These express the same observable behavior as the C `src/ulidgen.c`
//! `main`: argument parsing (`-n N`, `-t`), tag mode prefixing each stdin
//! line with a ULID, generate mode printing N ULIDs, exit status 0 on
//! success and 1 on bad arguments or a failed stdout write
//! (mirrors C `exit(!!ferror(stdout))`).

use std::io::Write;
use std::process::{Command, Stdio};

/// Crockford base32 alphabet (no I, L, O, U) — same as `B32` in src/lib.rs.
const B32: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ulidgen")
}

/// Port of `is_valid_ulid` from tests/test.c: 26 chars, all in the alphabet.
fn is_valid_ulid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|c| B32.contains(&c))
}

#[test]
fn cli_default_prints_one_valid_ulid() {
    let out = Command::new(bin()).output().expect("run ulidgen");
    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "default is one ULID, got: {stdout:?}");
    assert!(is_valid_ulid(lines[0]), "invalid ULID: {}", lines[0]);
}

#[test]
fn cli_n_prints_n_sorted_unique_ulids() {
    let out = Command::new(bin())
        .args(["-n", "5"])
        .output()
        .expect("run ulidgen -n 5");
    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 5, "expected 5 ULIDs, got: {stdout:?}");
    for line in &lines {
        assert!(is_valid_ulid(line), "invalid ULID: {line}");
    }
    // Same-millisecond calls must be strictly increasing (stateful
    // increment path), hence unique and lexicographically sorted.
    for window in lines.windows(2) {
        assert!(
            window[0] < window[1],
            "ULIDs not strictly increasing: {:?} vs {:?}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn cli_n_zero_prints_nothing_and_succeeds() {
    let out = Command::new(bin())
        .args(["-n", "0"])
        .output()
        .expect("run ulidgen -n 0");
    assert!(out.status.success(), "exit status: {:?}", out.status);
    assert!(out.stdout.is_empty(), "expected no output, got {:?}", out.stdout);
}

#[test]
fn cli_tag_mode_prefixes_every_stdin_line() {
    let mut child = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen -t");
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin
        .write_all(b"alpha\nbeta\n")
        .expect("write stdin");
    drop(stdin); // EOF
    let out = child.wait_with_output().expect("wait for ulidgen -t");
    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 tagged lines, got: {stdout:?}");
    for (i, line) in lines.iter().enumerate() {
        assert!(line.len() > 27, "line too short: {line:?}");
        assert!(is_valid_ulid(&line[..26]), "invalid ULID prefix: {line:?}");
        assert_eq!(line.as_bytes()[26], b' ', "expected space after ULID: {line:?}");
        let expected = if i == 0 { "alpha" } else { "beta" };
        assert!(
            line.ends_with(expected),
            "line {i} should end with {expected:?}, got: {line:?}"
        );
    }
}

#[test]
fn cli_tag_mode_empty_input_succeeds_silently() {
    let out = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run ulidgen -t");
    assert!(out.status.success(), "exit status: {:?}", out.status);
    assert!(out.stdout.is_empty(), "expected no output, got {:?}", out.stdout);
}

#[test]
fn cli_non_numeric_n_exits_1_with_usage() {
    let out = Command::new(bin())
        .args(["-n", "abc"])
        .output()
        .expect("run ulidgen -n abc");
    assert_eq!(out.status.code(), Some(1), "expected exit 1, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("usage"), "expected usage on stderr, got: {stderr:?}");
}

#[test]
fn cli_bare_n_exits_1_with_usage() {
    let out = Command::new(bin())
        .arg("-n")
        .output()
        .expect("run ulidgen -n");
    assert_eq!(out.status.code(), Some(1), "expected exit 1, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("usage"), "expected usage on stderr, got: {stderr:?}");
}

#[test]
fn cli_unknown_flag_exits_1_with_usage() {
    let out = Command::new(bin())
        .arg("-x")
        .output()
        .expect("run ulidgen -x");
    assert_eq!(out.status.code(), Some(1), "expected exit 1, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("usage"), "expected usage on stderr, got: {stderr:?}");
}

#[test]
fn cli_write_failure_exits_1() {
    // Close the read end of stdout immediately. The writer cannot finish
    // (100_000 lines >> pipe buffer), so a write eventually fails with
    // EPIPE. Rust ignores SIGPIPE, so the write returns Err and main must
    // exit(1) — mirroring C `exit(!!ferror(stdout))`.
    let mut child = Command::new(bin())
        .args(["-n", "100000"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen -n 100000");
    let stdout = child.stdout.take().expect("stdout piped");
    drop(stdout); // close the read end
    let status = child.wait().expect("wait for ulidgen");
    assert_eq!(
        status.code(),
        Some(1),
        "expected exit 1 on write failure, got {:?}",
        status
    );
}
