# 0004. Mise tasks instead of cargo aliases

- **Date:** 2026-09-14
- **Status:** accepted

## Context

The repository kept its two lint shortcuts as cargo aliases in `.cargo/config.toml` (`lint-rustfmt`, `lint-clippy`), copied from litmus. This coupling had three costs. Only cargo could invoke them. The aliases were invisible to tooling outside cargo. The repository already uses mise (2026.8.6 installed) as its tool manager, so a second alias mechanism sat next to an existing one.

## Decision

- Move both aliases to `mise.toml` as mise tasks: `mise run lint-rustfmt`, `mise run lint-clippy`.
- Delete `.cargo/config.toml` and the empty `.cargo/` directory.
- Command definitions stay byte-for-byte the same as the old aliases. Verified: both tasks run green before the cutover commit.

## Consequences

- `cargo lint-rustfmt` no longer works. Use `mise run lint-rustfmt` (and `mise run lint-clippy`).
- One alias mechanism remains: mise. Future repo commands (lint, test, release steps) belong in `mise.toml`.
- Agents that assume cargo aliases must read `mise.toml`. `AGENTS.md` names both tasks.
