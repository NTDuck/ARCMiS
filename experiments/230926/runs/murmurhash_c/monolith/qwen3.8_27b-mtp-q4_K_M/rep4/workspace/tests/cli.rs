//! `cli` - integration tests for the `murmur` command line utility

use std::io::Write;
use std::process::{Command, Stdio};

fn run_murmur(args: &[&str], input: &[u8]) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_murmur"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn murmur");

    {
        let stdin = child.stdin.as_mut().expect("no stdin");
        stdin.write_all(input).expect("failed to write stdin");
    }

    let output = child.wait_with_output().expect("failed to wait on murmur");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn test_cli_kinkajou() {
    let (code, stdout, _stderr) = run_murmur(&[], b"kinkajou");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "3067714808");
}

#[test]
fn test_cli_panda_seed10() {
    let (code, stdout, _stderr) = run_murmur(&["--seed=10"], b"panda");
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "1406483717");
}

#[test]
fn test_cli_version() {
    let (code, _stdout, stderr) = run_murmur(&["-V"], b"");
    assert_eq!(code, 0);
    assert_eq!(stderr.trim(), murmurhash::VERSION);
}

#[test]
fn test_cli_help() {
    let (code, _stdout, stderr) = run_murmur(&["-h"], b"");
    assert_eq!(code, 0);
    assert!(stderr.contains("usage: murmur"));
    assert!(stderr.contains("--seed=[seed]"));
}

#[test]
fn test_cli_unknown_option() {
    let (code, _stdout, stderr) = run_murmur(&["--bogus"], b"");
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown option"));
}
