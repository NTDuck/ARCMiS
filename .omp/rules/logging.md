---
description: Never print. Log through tracing with structured fields, async-safe, levels per audience. Log messages follow asd-ste100.
---

# Logging

Do not print. All diagnostic output goes through the [`tracing`](https://docs.rs/tracing) ecosystem.

## Rules

- No `println!`, `eprintln!`, `dbg!`, or `print!` anywhere in the workspace. Library code logs through `tracing` macros. The binary installs one `tracing-subscriber` at startup.
- Pick the level by audience. Use `error` when the operator must act. Use `warn` for degraded but continuing. Use `info` for milestones a human follows. Use `debug` and `trace` for diagnosis.
- Log messages follow ASD-STE100 (see `.omp/rules/ste.md`): short, active, one idea per message.
- Attach data as structured fields, not string interpolation: `tracing::info!(turn = ctx.turn(), tool = event.tool_name, "tool call")`. Interpolate values into the message only when they are the message.
- Do not log secrets, tokens, or full file contents. Log names, paths, counts, and outcomes.
- Libraries never install subscribers and never configure output. Only the binary entry point does.

## Enforcement

- `python3 .omp/scripts/lint-rules.py` fails any `println!`, `eprintln!`, `print!`, or `dbg!` in workspace code.

## Violations

- `println!("=== measurement: ...")` in a binary. Use `tracing::info!`.
- A library crate calling `eprintln!` for a warning.
- A log line that string-formats a struct instead of using fields.
