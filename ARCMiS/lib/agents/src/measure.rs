//! Result measurement for a migration run. Compiles the output codebase
//! and runs the translated tests; both numbers are outcomes, not gates.

/// Final measurement of one migration run.
#[derive(::core::fmt::Debug, ::serde::Serialize)]
pub struct Measurement {
    /// Build status of the output codebase: pass or fail.
    pub compile: &'static str,
    /// Test pass rate: passed / total. `None` when the build failed or no
    /// test runner reported.
    pub test_pass_rate: ::core::option::Option<f64>,
    /// Raw runner output for the log.
    pub detail: ::std::string::String,
}

/// Build the output package and run the test command. Both steps run in the
/// output dir; `test_command` is one shell line from the config.
pub async fn measure(output_dir: &::std::path::Path, test_command: &str) -> Measurement {
    let build = run_line(output_dir, "cargo build").await;
    if !build.0 {
        return Measurement {
            compile: "fail",
            test_pass_rate: ::core::option::Option::None,
            detail: build.1,
        };
    }
    let (ok, out) = run_line(output_dir, test_command).await;
    let test_pass_rate = parse_pass_rate(&out);
    if test_pass_rate.is_none() {
        ::tracing::warn!(runner_ok = ok, "test runner reported no counts");
    }
    Measurement {
        compile: "pass",
        test_pass_rate,
        // Surface an unparseable runner as part of the detail, not silence.
        detail: if test_pass_rate.is_none() {
            format!("test runner reported no counts; ok={ok}\n{out}")
        } else {
            out
        },
    }
}

/// Run one shell line in `dir`. Returns (success, combined output).
async fn run_line(dir: &::std::path::Path, line: &str) -> (bool, ::std::string::String) {
    let output = ::tokio::process::Command::new("sh").arg("-c").arg(line).current_dir(dir).output().await;
    match output {
        ::core::result::Result::Ok(out) => {
            let text = format!(
                "{}{}",
                ::std::string::String::from_utf8_lossy(&out.stdout),
                ::std::string::String::from_utf8_lossy(&out.stderr),
            );
            (out.status.success(), text)
        },
        ::core::result::Result::Err(error) => (false, format!("spawn failed: {error}")),
    }
}

/// Extract a pass rate from common test-runner output shapes.
/// Known shapes: `N passed; M failed` (cargo), `X tests, Y assertions, Z failures`.
fn parse_pass_rate(output: &str) -> ::core::option::Option<f64> {
    let passed = find_count(output, &["passed", " ok"]);
    let failed = find_count(output, &["failed", "FAILED", "failures:"]);
    let total = passed + failed;
    if total == 0 {
        return ::core::option::Option::None;
    }
    ::core::option::Option::Some(passed as f64 / total as f64)
}

/// Sum integers that precede any of the markers.
fn find_count(output: &str, markers: &[&str]) -> u64 {
    markers
        .iter()
        .filter_map(|marker| {
            let index = output.find(marker)?;
            let before = &output[..index];
            let digits: ::std::string::String =
                before.chars().rev().take_while(|c| c.is_ascii_digit() || *c == ' ').collect();
            digits.split_whitespace().next().and_then(|n| n.parse::<u64>().ok())
        })
        .sum()
}
