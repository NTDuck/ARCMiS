//! Black-box tests for the `ulidgen` binary: `main`, clap `Args`, and
//! `parse_long` (exercised through the `-n` value parser).

use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ulidgen"))
}

fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26
        && ulid
            .bytes()
            .all(|c| ulidgen::B32_ALPHABET.as_bytes().contains(&c))
}

#[test]
fn default_prints_one_ulid() {
    let out = bin().output().expect("run ulidgen");
    assert!(out.status.success(), "exit status must be 0 on success");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "default mode prints exactly one ULID: {stdout:?}");
    assert!(is_valid_ulid(lines[0]), "line must be a valid ULID: {lines:?}");
}

#[test]
fn n_mode_prints_n_ulids() {
    let out = bin().arg("-n").arg("3").output().expect("run ulidgen -n 3");
    assert!(out.status.success(), "exit status must be 0 on success");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "-n 3 must print 3 ULIDs: {stdout:?}");
    for line in &lines {
        assert!(is_valid_ulid(line), "each line must be a valid ULID: {line}");
    }
    // Consecutive same-millisecond ULIDs must be unique (shared buffer).
    assert_ne!(lines[0], lines[1]);
    assert_ne!(lines[1], lines[2]);
}

#[test]
fn n_zero_prints_nothing() {
    let out = bin().arg("-n").arg("0").output().expect("run ulidgen -n 0");
    assert!(out.status.success(), "exit status must be 0 on success");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "", "-n 0 prints nothing");
}

#[test]
fn invalid_n_fails_with_parse_long_error() {
    // `parse_long` rejects non-numeric input; clap reports it and exits non-zero.
    let out = bin().arg("-n").arg("abc").output().expect("run ulidgen -n abc");
    assert!(!out.status.success(), "invalid -n must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid number"),
        "stderr must carry the parse_long error message: {stderr:?}"
    );
}

#[test]
fn t_mode_prefixes_each_stdin_line() {
    let out = bin()
        .arg("-t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ulidgen -t");

    use std::io::Write;
    let mut child = out;
    {
        let stdin = child.stdin.take().expect("stdin pipe");
        let mut w = std::io::BufWriter::new(stdin);
        w.write_all(b"a\nb\n").expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait for ulidgen -t");
    assert!(output.status.success(), "exit status must be 0 on success");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "-t must prefix each input line: {stdout:?}");
    assert_eq!(lines[0].split(' ').nth(1).unwrap_or(""), "a");
    assert_eq!(lines[1].split(' ').nth(1).unwrap_or(""), "b");
    for line in &lines {
        let ulid = line.split(' ').next().unwrap_or("");
        assert!(is_valid_ulid(ulid), "prefix must be a valid ULID: {line}");
    }
}

#[test]
fn t_mode_empty_stdin_prints_nothing() {
    let out = bin()
        .arg("-t")
        .stdin(Stdio::piped())
        .output()
        .expect("run ulidgen -t with empty stdin");
    assert!(out.status.success(), "exit status must be 0 on success");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "", "empty stdin prints nothing");
}
