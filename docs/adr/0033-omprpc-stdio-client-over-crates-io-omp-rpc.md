# 0033. omprpc stdio client over the crates.io omp-rpc crate

- **Date:** 2026-10-11
- **Status:** accepted
- **Related:** [0030](0030-omprpc-transport-replaces-rig.md)

## Context

ADR 0030 names `omp --mode rpc` as the agent transport. Two candidate
implementations exist:

1. The published `omp-rpc` 0.1.0 and `omp-proto` 0.1.0 crates on
   crates.io.
2. The thin in-tree stdio JSONL client under `ARCMiS/lib/omprpc`.

The parallel transport probe collected verbatim evidence about the
published crates:

- `omp-proto` 0.1.0 defines gRPC services `Gateway(Hello)`, `Auth`,
  `Blob`, and `Inference` (raw LLM turns and facets). No session,
  prompt, or subagent service exists in the proto surface.
- `omp-rpc` 0.1.0 carries Unix-domain-socket connect and listen, mTLS,
  the `Hello` handshake, and `grpc.health.v1` plumbing. Nothing else.
- The local omp binary (18.6.1) has no `gateway`, `daemon`, or `serve`
  subcommand. Each falls through to `omp launch`.
- The protocol that drives agent sessions exists only over `omp --mode
  rpc` (and `omp acp`) stdio: banner skip, ready handshake,
  `negotiate_protocol`, `set_subagent_subscription`, `prompt` /
  `prompt_result`, and subagent routing by `payload.agent`.

The published crates cannot drive an agent session: their service set
has no overlap with the frames the conductor needs.

## Decision

Keep `ARCMiS/lib/omprpc`, the thin in-tree stdio JSONL client, as the
sole agent transport. Reject the crates.io `omp-rpc` 0.1.0 for now.

## Consequences

- The in-tree client stays the only code path that can start, drive,
  and observe an agent session. Its conformance tests are the
  transport's record.
- Revisit this decision when omp ships a daemon surface whose service
  set covers session, prompt, and subagent routing. Until then, a
  gRPC dependency buys nothing the stdio protocol does not already
  give.
- The crates.io packages remain out of the dependency graph. Adding
  them later is a normal dependency bump once the evidence changes.
