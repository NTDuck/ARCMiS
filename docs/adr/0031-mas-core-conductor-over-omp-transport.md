# 0031. mas-core conductor: static hierarchy, dynamic routing, blackboard

- **Date:** 2026-10-11
- **Status:** accepted
- **Extends:** [0022](0022-hierarchical-dynamic-adaptive-orchestration.md), [0026](0026-orchestrator-rename-hierarchical-static-orchestration.md), [0027](0027-laya-typed-triage-on-lead-dispatch.md), [0030](0030-omprpc-transport-replaces-rig.md)
- **Scope:** runtime orchestration layer on the dev branch

## Context

The Rig-retirement ADR (0030) retires the orchestration crates' code
without retiring the orchestration science. ADRs 0022, 0026, and 0027
define a three-tier static hierarchy with dynamic selection at the
bottlenecks, evidenced broadly enough (campaign ledgers, `break`
post-mortems, and per-round harness snapshots) to rebuild rather than
reinvent.

Meanwhile, arbiter routing favors the deterministic path first: it
bounds expensive arbitrators away from positions that cheaper
agreement can fill. Arbiters back the remaining uncertainty, never
mask it.

## Decision

`lib/mas-core` owns the conductor runtime, comprising:

1. **A static hierarchy registry.** Role roster, preambles, and team
   partitions compiled from the run configuration. Immutable for a
   run's duration.
2. **A task graph with a phase machine.** Preflight → Discovery →
   Contract → Planning → Pilot → Migration → Integration → Hardening
   → FinalValidation → Done. Forward movement requires evidence-gated
   phase exit. Failures traverse backward along deterministic
   regression edges.
3. **A dynamic router** that composes layers: deterministic
   keyword/state pass first, laya typed decisions at the
   pre-identified bottlenecks, orchestrator model arbitration only for
   the ambiguous remainder.
4. **A shared blackboard store plus direct messaging** for all
   inter-agent communication.
5. **A jev-as-a-guard tool gateway**: deterministic allowlist, path
   policy, deny patterns, and budget checks before any tool call
   reaches an agent. Lay down an attack only for the ambiguous
   remainder (never mask the verdict).
6. **Snapcompact-style context compaction** attached uniformly to all
   agents, driven by the configured token thresholds.

These compose into one conductor because they share one lifecycle: a
single process lifetime, a single transport, and one synchronous view
of the task graph.

## Consequences

- The laya typed-decision guard keeps the trained-schema constraint
  (ADR 0027). Consultations outside the
  `agent_trace_observability` schema produce unreliable answers.
  Cascade fallback fires below the confidence threshold, honors the
  observation that retry-with-uncertainty beats confident guessing, and
  never masks a verdict.
- The blackboard transport keeps every exchange observable: each
  write and message has one recorded sender, recipient, and topic.
- Tools delivered by omp arrive as a versioned typed set per
  negotiated protocol version. Gateway enforcement therefore
  shields agent reasoning from knowing that detail.
- Suite configurations never rely on the consolidated global order
  of distributed arrivals. See ADR 0032 for the mandatory,
  explicit configuration contract that makes every such assumption
  inspectable.
