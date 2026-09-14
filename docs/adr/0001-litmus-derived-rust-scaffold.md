# 0001. Litmus-derived Rust scaffold

- **Date:** 2026-09-14
- **Status:** accepted

## Context

ARCMiS started as a bare repository (LICENSE + README + staged `.codegraph/.gitignore`). It needed initial Rust state: manifest, lib root, lint/format configs, test profile, and CI. The owner's own [NTDuck/litmus](https://github.com/NTDuck/litmus/tree/67713d67a50c33a94f0e2b9eae7c65c041a31fe7) repository (macro-free BDD harness for Rust) is the sibling crate in the same org, and the request named it as the model. Reusing its config wholesale keeps style uniform across NTDuck crates and gives day-one CI (fmt/clippy on nightly, cross-platform nextest matrix, cargo-deny/cargo-audit) with zero bespoke YAML.

## Decision

- Copy from litmus@67713d67: `.github/workflows/{ci,lint,test,build,dependencies-check}.yml`, `.clippy.toml`, `.rustfmt.toml`, `.cargo/config.toml`, `.config/nextest.toml`, `.gitignore`.
- New, ARCMiS-specific: minimal `Cargo.toml` (`name = "arcmis"`, `edition = "2021"`, `publish = false`, litmus-matching `[lints.clippy]`), placeholder `src/lib.rs`.
- Kept the pre-existing BSD-3-Clause LICENSE (same copyright holder, NTDuck, as litmus) and set `license = "BSD-3-Clause"` in the manifest to match.

## Consequences

- `.config/nextest.toml` keeps litmus's `[profile.ci] fail-fast = false` but drops the litmus-specific `profile.default.default-filter` (it excludes a `cucumber-rs` example binary that does not exist here; nextest 0.9.143 rejects filters referencing unknown binary names).
- Crate starts at v0.1.0 unpublished; no `[[example]]` entries yet.
- If litmus's config drifts, this repo drifts — syncing is manual.
