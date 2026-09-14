---
description: Use Conventional Commits 1.0.0; commit granularly and incrementally after every verified change, never batch unrelated changes.
---

# Granular Incremental Conventional Commits

Commit every logical change immediately after it verifies green. Never batch unrelated changes into one commit; never wait until the end of a task to commit everything at once.

## 1. Format

Follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <subject>`, optional blank-line-separated body, optional footers.

- Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`, `style`.
- Scope: the area touched (crate, module, or subsystem); optional when the subject alone identifies it.
- Breaking changes: `!` after type/scope (`feat(api)!: ...`) and/or a `BREAKING CHANGE: <description>` footer.
- Subject: imperative mood, present tense, ≤72 chars, no trailing period. Body explains *why*, not *what*, only when needed.

## 2. Granularity

- One commit per logical change, immediately after its verification passes (`cargo fmt`/`clippy`/`test` or equivalent gates).
- Never mix types in one commit (`feat` + `fix` → split).
- Never commit a red tree. If gates fail, fix before committing.

## 3. Examples

```text
feat(rules): add granular commit policy
fix(clippy): pin absolute-paths-max-segments to zero
docs(adr): record 0002 stable-rustfmt decision
chore: track codegraph database gitignore
```
