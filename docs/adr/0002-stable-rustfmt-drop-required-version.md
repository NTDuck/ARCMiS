# 0002. Stable rustfmt: drop required_version

- **Date:** 2026-09-14
- **Status:** accepted

## Context

litmus's `.rustfmt.toml` ends with `required_version = "1.8.0"`. Empirically, rustfmt **stable** 1.97.1 (the only local toolchain) fails on that line (`error: invalid value '1.8.0' for '--config-...'`), while accepting the remainder of the config with only warnings for the nightly-only options (`imports_granularity`, `group_imports`, `wrap_comments`, `format_strings`, `unstable_features`, …). Removing just that line makes `cargo fmt --check` pass on stable, exit 0. CI's lint job already runs nightly rustfmt, where the full option set applies.

## Decision

Adopt litmus's `.rustfmt.toml` verbatim **minus** the `required_version = "1.8.0"` line. Do not pin a `rust-toolchain.toml` to nightly for formatting.

## Consequences

- Local stable formatting is a subset of nightly formatting; the nightly-only opts are ignored with warnings locally and enforced in CI lint (nightly). Both channels share one config file.
- No toolchain pin: contributors use their default toolchain. If a future rustfmt stabilizes these options, behavior converges without a change here.
- If someone reintroduces `required_version`, stable-only machines break again — that line was the sole hard failure.
