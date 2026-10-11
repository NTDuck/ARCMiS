# 0030. omp-RPC transport replaces Rig

- **Date:** 2026-10-11
- **Status:** accepted
- **Supersedes:** nothing (ADR 0022 and ADR 0026 carry over at the methodology layer)
- **Scope:** the `init` branch's Rig-based orchestration harness

## Context

The `init` branch drove its multi-agent modernization harness on Rig
(`rig = "0.42.0"`), hosting agents, orchestrator, and tools as
Rig-native artifacts. Three forces ended that marriage:

1. **SLM-first posture.** The default model class consists of small
   quantized models served locally. Rig's abstraction tax bought
   little here, while its reactive type surface raised the maintenance
   floor.
2. **Protocol-level control.** The omp coding-agent system already
   defined a precise service boundary. Owning that protocol
   end-to-end gave us controlled negotiation, subscription, and
   routing surfaces without adapter seams.
3. **Subagent observability.** `omp --mode rpc` exposes typed frames
   for subagent progress and routing. Each fleet decision becomes
   first-class: subscriptions emit forward progress, and routing
   decisions keyed on `payload.agent` map naturally onto our
   tier hierarchy. As a bonus, typed trace events render zero-cost.

## Decision

Drop Rig from the workspace. Introduce `lib/omprpc`, an `omp --mode rpc`
JSONL transport owning banner skip, ready handshake,
`negotiate_protocol`, `set_subagent_subscription`, `prompt` /
`prompt_result`, and subagent routing by `payload.agent`, with camelCase
serde tagging.

## Consequences

- The dev branch removes `ARCMiS/lib/{agents, orchestrator, blackboard,
  tools, middleware, snapcompact}` and `bin/harness`. Methodologies from
  those crates persist in specs (see `docs/specs/TECH-SPECS.md`),
  ADRs, and prose papers. Retiring the code does not retire its lessons.
- Upstream Rig improvements no longer flow in automatically. Protocol
  fidelity tracks our own transport conformance tests instead.
- Rig-based reasoning about tool/agent traits applies only through
  git history, superseded ADRs 0003/0009/0012, and their successors.
