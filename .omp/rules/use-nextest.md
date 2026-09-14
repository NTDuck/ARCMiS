# Use cargo-nextest

Run all Rust tests through `cargo-nextest` (installed locally, 0.9.143). Never use `cargo test` to run tests.

- Local runs: `cargo nextest run --profile ci --workspace --all-targets --no-tests=pass`.
- CI (`.github/workflows/test.yml`) already uses the same command. Keep both surfaces on nextest.
- Do not replace this command with `cargo test`. nextest runs each test in its own process. It gives per-test isolation, better output, and stable retries.
- If you need cargo's test binary features that nextest cannot run (for example doctests), run that one thing with `cargo test` and name the exception. nextest does not run doctests.

## Why

- One test runner for every surface: local, CI, agents. One command to read, one set of flags to learn.
- Process-per-test stops tests from sharing state. It stops a flaky test from poisoning other tests.
- `--no-tests=pass` keeps empty crates green instead of failing the run.
