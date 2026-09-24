//! Integration tests for the CLI `main` (src/main.rs), port of `src/ulidgen.c`.
//!
//! Exercises both modes of the binary:
//!   -n N   generate N ULIDs (default 1)
//!   -t     prefix each stdin line with a ULID (byte-exact, newline preserved)
//! plus the usage-error exit path.

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ulidgen")
}

fn is_valid_ulid(s: &str) -> bool {
    const A: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    s.len() == 26 && s.bytes().all(|c| A.as_bytes().contains(&c))
}

/// Run the binary in tag mode with the given stdin bytes, return stdout bytes.
fn run_tag_mode(stdin_bytes: &[u8]) -> (std::process::ExitStatus, Vec<u8>) {
    let mut child = Command::new(bin())
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen -t");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(stdin_bytes)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for ulidgen -t");
    (out.status, out.stdout)
}

#[test]
fn cli_default_prints_one_ulid() {
    // no args: n defaults to 1, one ULID line, exit 0
    let out = Command::new(bin()).output().expect("run ulidgen");
    assert!(out.status.success(), "expected exit 0");
    let s = String::from_utf8(out.stdout).expect("utf8 stdout");
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 1, "expected exactly one line, got {s:?}");
    assert!(is_valid_ulid(lines[0]), "invalid ULID: {}", lines[0]);
    assert_eq!(s, format!("{}\n", lines[0]), "unexpected trailing bytes");
}

#[test]
fn cli_n_generates_n_unique_ulids() {
    // -n 5: exactly 5 lines, each a valid, unique ULID, exit 0
    let out = Command::new(bin())
        .arg("-n")
        .arg("5")
        .output()
        .expect("run ulidgen -n 5");
    assert!(out.status.success(), "expected exit 0");
    let s = String::from_utf8(out.stdout).expect("utf8 stdout");
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 5, "expected 5 lines, got {s:?}");
    for l in &lines {
        assert!(is_valid_ulid(l), "invalid ULID: {l}");
    }
    let unique: std::collections::HashSet<&str> = lines.iter().copied().collect();
    assert_eq!(unique.len(), 5, "ULIDs must be unique: {s:?}");
    // same-millisecond increments keep the sequence lexicographically sorted
    let sorted: Vec<&str> = lines.clone();
    let mut sorted2 = lines.clone();
    sorted2.sort();
    assert_eq!(sorted, sorted2, "ULIDs must be sorted: {s:?}");
}

#[test]
fn cli_n_zero_prints_nothing() {
    // -n 0: empty output, exit 0
    let out = Command::new(bin())
        .arg("-n")
        .arg("0")
        .output()
        .expect("run ulidgen -n 0");
    assert!(out.status.success(), "expected exit 0");
    assert!(out.stdout.is_empty(), "expected empty stdout, got {:?}", out.stdout);
}

#[test]
fn cli_t_tags_each_line() {
    // -t: "a\nb\n" -> "<ulid> a\n<ulid> b\n" (newline preserved, byte-exact)
    let (status, stdout) = run_tag_mode(b"a\nb\n");
    assert!(status.success(), "expected exit 0");
    let s = String::from_utf8(stdout).expect("utf8 stdout");
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines, got {s:?}");
    let bodies = ["a", "b"];
    for (i, l) in lines.iter().enumerate() {
        let sp = l.find(' ').expect("space after ULID");
        assert_eq!(sp, 26, "ULID must be exactly 26 chars: {l:?}");
        assert!(is_valid_ulid(&l[..26]), "invalid ULID: {}", &l[..26]);
        assert_eq!(&l[27..], bodies[i], "line body mismatch: {l:?}");
    }
    assert_eq!(
        s,
        format!("{} a\n{} b\n", &lines[0][..26], &lines[1][..26]),
        "output must be byte-exact"
    );
}

#[test]
fn cli_t_preserves_missing_trailing_newline() {
    // -t with input "a" (no trailing newline): output "<ulid> a" with NO
    // trailing newline — mirrors printf("%s %s", ulid, line) byte-for-byte.
    let (status, stdout) = run_tag_mode(b"a");
    assert!(status.success(), "expected exit 0");
    let s = String::from_utf8(stdout).expect("utf8 stdout");
    assert!(!s.ends_with('\n'), "must not add a trailing newline: {s:?}");
    let sp = s.find(' ').expect("space after ULID");
    assert_eq!(sp, 26, "ULID must be exactly 26 chars: {s:?}");
    assert!(is_valid_ulid(&s[..26]), "invalid ULID: {}", &s[..26]);
    assert_eq!(&s[27..], "a", "line body mismatch: {s:?}");
}

#[test]
fn cli_t_empty_input_prints_nothing() {
    // -t with empty stdin: no output, exit 0
    let (status, stdout) = run_tag_mode(b"");
    assert!(status.success(), "expected exit 0");
    assert!(stdout.is_empty(), "expected empty stdout, got {:?}", stdout);
}

#[test]
fn cli_unknown_flag_exits_1_with_usage() {
    // unknown option: usage on stderr, exit 1 (mirrors getopt error path)
    let out = Command::new(bin())
        .arg("-x")
        .output()
        .expect("run ulidgen -x");
    assert_eq!(out.status.code(), Some(1), "expected exit code 1");
    let err = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(err.contains("usage"), "expected usage message, got {err:?}");
}
