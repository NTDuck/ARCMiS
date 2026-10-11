# 0034. mas-core transport seam

- **Date:** 2026-10-11
- **Status:** accepted
- **Extends:** [0030](0030-omprpc-transport-replaces-rig.md), [0031](0031-mas-core-conductor-over-omp-transport.md)
- **Scope:** `ARCMiS/lib/mas-core` on the dev branch

## Context

ADR 0031 puts the conductor runtime over the omp transport, and ADR 0030
names `ARCMiS-omprpc` the single stdio client. The conductor needs to
dispatch prompts and drain frames, but the conductor module must stay
testable: a run loop that requires a live `omp --mode rpc` child for
every test spawns processes, depends on the installed binary, and
couples session lifecycle to run logic.

The concrete friction shows up in the type itself. `omprpc::Session`
implements no `Debug`, holds the child handle, and exposes
fire-and-forget requests with terminal frames on the event stream. A
conductor written directly against `Session` cannot substitute a
scripted transport without generics over the concrete type, and the
conductor test in `tests/conductor.rs` would need a real child.

## Decision

`mas-core` defines a local `Transport` trait with three methods:
`send_prompt` (fire-and-forget, returns the request id), `recv` (next
decoded frame), and `recv_within` (deadline variant). A thin
`OmpTransport` adapter wraps one `omprpc::Session` and maps
`omprpc::Error` onto a local `TransportError` (`Closed`, `Send`,
`Timeout`, `BadRequest`). The conductor holds any `T: Transport`.

Consequences:

- Tests inject a scripted transport. `tests/conductor.rs` runs the full
  phase walk over an in-memory transport with no child process.
- Session lifecycle (spawn, kill-on-drop, reader task) stays inside
  `OmpTransport`. The conductor never sees a child handle.
- `Session` needs no changes: the adapter is the only coupling point.
  The missing `Debug` impl never leaks past the adapter.
- The trait uses async-fn-in-trait (Rust 2024 prelude `Future`), so it
  stays object-safe at the call site through the generic parameter
  rather than dynamic dispatch.

## Alternatives considered

- Direct `omprpc::Session` in the conductor: rejected. Every conductor
  test would spawn a child, and the run loop could not exercise
  regression edges or budget exhaustion deterministically.
- Upstreaming the trait into `omprpc`: rejected. omprpc owns one
  implementation and the trait exists for injection. Keeping it in
  mas-core avoids a second public contract in omprpc.

## References

- `ARCMiS/lib/mas-core/src/transport.rs` — trait and adapter.
- `ARCMiS/lib/mas-core/tests/conductor.rs` — scripted transport in use.
