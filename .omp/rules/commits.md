---
description: Use Conventional Commits 1.0.0. Commit each verified change at once. Do not batch unrelated changes.
---

# Conventional Commits

Make one commit for each logical change. Make it when verification passes. Do not batch unrelated changes. Do not save commits for the end of a task.

## 1. Format

Follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <subject>`. Add an optional body and optional footers.

- Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`, `style`.
- The scope names the crate, module, or subsystem. It is optional when the subject alone identifies the change.
- Mark breaking changes with `!` after the type or scope. Example: `feat(api)!: ...`. Or add a `BREAKING CHANGE: <description>` footer.
- Write the subject in the imperative and present tense. Keep it at or below 72 characters. Do not end it with a period. Use the body to explain why, not what. Add a body only when needed.

## 2. Granularity

- Make one commit per logical change. Make it immediately after verification passes (`cargo fmt`, `clippy`, tests, or equivalent gates).
- Do not mix types in one commit. Put `feat` and `fix` in separate commits.
- Never commit a red tree. If gates fail, fix the code first.

## 3. Examples

```text
feat(rules): add granular commit policy
fix(clippy): pin absolute-paths-max-segments to zero
docs(adr): record 0002 stable-rustfmt decision
chore: track codegraph database gitignore
```
