//! Integration tests for the CLI (`main` in `src/main.rs`, port of `src/ulidgen.c`).
//!
//! Exercises the binary built by cargo:
//! - `-n N` generates N valid, distinct ULIDs (default N = 1)
//! - `-t` prefixes each stdin line verbatim with `ULID `
//! - bad arguments (unknown option, `-n` without/with bad operand) exit with status 2
//!   and print the usage text on stderr (C `getopt` error path).

use std::io::Write;
use std::process::{Command, Stdio};

fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26 && ulid.chars().all(|c| ulidgen::B32_ALPHABET.contains(c))
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ulidgen")
}

/// `-n 3` prints exactly 3 valid, distinct ULIDs, one per line, exit 0.
#[test]
fn cli_n_generates_n_ulids() {
    let out = Command::new(bin())
        .arg("-n")
        .arg("3")
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got: {stdout:?}");
    let mut seen = std::collections::HashSet::new();
    for line in &lines {
        assert!(is_valid_ulid(line), "invalid ULID: {line}");
        assert!(seen.insert(*line), "duplicate ULID: {line}");
    }
}

/// No arguments: default is 1 ULID, exit 0.
#[test]
fn cli_default_single_ulid() {
    let out = Command::new(bin()).output().expect("run ulidgen");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 line, got: {stdout:?}");
    assert!(is_valid_ulid(lines[0]), "invalid ULID: {}", lines[0]);
}

/// `-t` prefixes each stdin line verbatim with `ULID `, exit 0.
#[test]
fn cli_t_tags_stdin_lines() {
    let mut child = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen");
    let mut stdin = child.stdin.take().expect("stdin pipe");
    stdin
        .write_all(b"alpha\nbeta\n")
        .expect("write stdin");
    drop(stdin);
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got: {stdout:?}");
    let (u1, rest1) = lines[0].split_once(' ').expect("ULID + space + line");
    let (u2, rest2) = lines[1].split_once(' ').expect("ULID + space + line");
    assert!(is_valid_ulid(u1), "invalid ULID: {u1}");
    assert_eq!(rest1, "alpha", "line not preserved verbatim: {rest1:?}");
    assert!(is_valid_ulid(u2), "invalid ULID: {u2}");
    assert_eq!(rest2, "beta", "line not preserved verbatim: {rest2:?}");
    assert_ne!(u1, u2, "tagged ULIDs must differ");
}

/// `-n` with a non-numeric operand: usage on stderr, exit 2.
#[test]
fn cli_bad_n_operand_exits_2() {
    let out = Command::new(bin())
        .arg("-n")
        .arg("x")
        .output()
        .expect("run ulidgen");
    assert_eq!(out.status.code(), Some(2), "expected exit 2, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage: ulidgen"), "missing usage text: {stderr:?}");
}

/// `-n` with a missing operand: usage on stderr, exit 2.
#[test]
fn cli_missing_n_operand_exits_2() {
    let out = Command::new(bin())
        .arg("-n")
        .output()
        .expect("run ulidgen");
    assert_eq!(out.status.code(), Some(2), "expected exit 2, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage: ulidgen"), "missing usage text: {stderr:?}");
}

/// Unknown option: usage on stderr, exit 2.
#[test]
fn cli_unknown_option_exits_2() {
    let out = Command::new(bin())
        .arg("-z")
        .output()
        .expect("run ulidgen");
    assert_eq!(out.status.code(), Some(2), "expected exit 2, got {:?}", out.status);
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("usage: ulidgen"), "missing usage text: {stderr:?}");
}

/// `-n 0` prints no lines and exits 0 (C `atol("0")` → zero iterations).
#[test]
fn cli_n_zero_prints_nothing() {
    let out = Command::new(bin())
        .arg("-n")
        .arg("0")
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    assert_eq!(stdout.lines().count(), 0, "expected no lines, got: {stdout:?}");
}

/// `-t` with empty stdin prints nothing and exits 0.
#[test]
fn cli_t_empty_stdin() {
    let out = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run ulidgen");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    assert_eq!(stdout.lines().count(), 0, "expected no lines, got: {stdout:?}");
}

/// `-t` preserves lines containing spaces verbatim after `ULID `.
#[test]
fn cli_t_preserves_spaces_in_line() {
    let mut child = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen");
    let mut stdin = child.stdin.take().expect("stdin pipe");
    stdin
        .write_all(b"hello world  foo\n")
        .expect("write stdin");
    drop(stdin);
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "expected exit 0, got {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let line = stdout.lines().next().expect("one output line");
    let (ulid, rest) = line.split_once(' ').expect("ULID + space + line");
    assert!(is_valid_ulid(ulid), "invalid ULID: {ulid}");
    assert_eq!(rest, "hello world  foo", "line not preserved verbatim: {rest:?}");
}
