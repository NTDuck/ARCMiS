# 0025. Extract role-agnostic hook middleware into its own crate

- **Date:** 2026-09-29
- **Status:** accepted

## Context

The rig `AgentHook` implementations lived inside the agents crate next to
the role domain logic: `mas/trace.rs` (TraceHook),
`mas/continuation.rs` (TrailingUserMessageHook), and
`mas/role_output_tokens.rs` (RoleOutputTokensHook). The hooks are
cross-cutting concerns over the model-call path. The role logic (which
hook, with which parameters for which role) is domain. Mixing them gave
the domain crate a second job and made the hooks unreachable to any
future consumer that does not want the role catalog.

Two candidate homes existed: keep them in `agents` behind a submodule,
or extract a dedicated crate. The snapcompact glue in `registry.rs`
complicates the choice: `SnapcompactHook` lives in the snapcompact crate,
and only the *wiring* (threshold per role, trailing query constant) sits
in the registry.

## Decision

Extract `ARCMiS-middleware` with lib name `middleware`. It holds the
role-agnostic hooks: `trace` (TraceHook, TraceSink), `continuation`
(TrailingUserMessageHook, `continuation_patch`, CONTINUATION_QUERY), and
`role_output_tokens` (RoleOutputTokensHook). It depends on rig +
serde_json + tracing only.

The split criterion: **a middleware hook never names a role, a fleet, or
the orchestration domain.** It reads only hook event data plus its own
constructor parameters. Anything that consults `Role`, `Fleet`, the
blackboard, or the task graph is domain logic and stays in `agents`
(which hook per role, per-role thresholds) or `orchestrator` (guard
policy).

The snapcompact *glue* stays in `registry.rs`. `SnapcompactHook` is
middleware-shaped, but its crate already exists and pulling the wiring
out would leave `registry` with a hole. The threshold selection is
per-role domain policy.

The guard stays in `orchestrator` without a `GuardPolicy` trait for now.
`GuardHook` is policy-embedded in a load-bearing way: the allowlist and
path policy are the delegation contract (ADR 0022's Jev-as-a-guard), and
the read-scoping gate consults the per-role tool-call budget set at
dispatch time. A trait seam would add an abstraction with exactly one
real implementation. Defer the seam until a second policy exists.

## Consequences

- New code under `agents/src/mas/` needs no mod.rs. We deleted the glue
  file and declared the modules inline in lib.rs per the layout rule.
- Middleware hooks are testable without the agents domain. The moved
  test modules pass unchanged.
- Adding a fourth hook (for example a per-tier trace decorator) lands in
  the middleware crate by default. The ADR 0026 hierarchy work uses
  this.
- Revisit the GuardPolicy seam when a second guard policy (for example a
  lead-scoped allowlist) appears.
