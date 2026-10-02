# 0029. Round parallel dispatch

- **Date:** 2026-10-02
- **Status:** accepted
- **Extends:** [0026](0026-orchestrator-rename-hierarchical-static-orchestration.md)
  (static hierarchy), [0022](0022-hierarchical-dynamic-adaptive-orchestration.md)
  (single-dial fanout semantics).
- **Supersedes:** nothing. The tier-1 grammar keeps its meaning. The lead
  loop joins the same execution contract.

## Context

The orchestration serialized specialist work at two points. Survey run s27
recorded roughly 500 translator dispatches that executed strictly one at a
time: the task-graph ready set dispatched one job per round, and the
tier-2 lead parsed at most one `DECISION: delegate` line per round. Each
dispatch also paid the model round trip before the next one started. The
single-engine setup offered no way to overlap them.

The new inference engine serves about 6 parallel instances at workable
tokens per second, so concurrent dispatches now finish instead of queueing.
The smoke run `c15-gr-hierarchy-smoke` showed the second cost directly: the
migration-lead exhausted its 8-turn budget by dispatching one member at a
time while re-sending its own verbose brief every round, and the run died
on the run-level stagnation breaker.

## Decision

Four layers, each independent:

1. **Tier-1 ready set.** The task-graph executor dispatches the ready set
   concurrently: one `tokio::spawn` job per dispatch, joined through the
   extracted `run_bounded` helper. Results join in ready-set (emission)
   order, not completion order, so the outcome stays deterministic. A
   panicked job becomes one failure row whose id falls back to the job's
   task id. The rest of the batch survives and no task stays `InProgress`.
   Status flips (`InProgress` before spawn, `Done`/`Blocked` after join)
   run serially outside the concurrent section. `set_status` reads and
   writes the whole file, so concurrent flips would lose updates.
2. **Lead rounds.** The cutover deletes `parse_lead_decision`. The new
   `parse_lead_round` parses every `DECISION` line in the lead's answer
   and returns `Done` or a dispatch list. Member dispatches run
   concurrently through the same spawn-and-join shape, bounded by
   `fanout`. Over-fanout dispatches hit the truncation point at the
   execution site. The harness leaves one `[deferred]` note per truncated
   dispatch in the transcript, so the lead re-emits it next round with
   fresh results in view. No
   in-code queue exists: excess work re-prompts naturally. Per-result jev
   triage (ADR 0027/0028) and the enforce-policy stop run after the round
   joins, so a stop considers the whole round. A panic on join becomes one
   failed `TaskResult` carrying the batch task id.
3. **One concurrency dial.** `fanout` in `MasConfig` bounds both the
   tier-1 ready set and the lead's member dispatches. Its documented
   meaning is engine-instance parallelism: how many independent
   dispatches the harness may run at once. It carries no hardcoded count
   and no per-set value (no-tuning rule). The round prompt's
   multi-delegate note matches this semantics. The harness defers
   over-fanout dispatches to a later round.
4. **Outer-loop slots stay separate.** `scripts/sweep.sh` already sizes
   its run waves with `SLOTS` and picks the engine with `ENGINE_URL` plus
   `HARNESS`. Those environment knobs belong to the sweep driver, not to
   the harness config. This ADR changes nothing about them.

## Consequences and limits

- Concurrent member dispatches that write the same file race. The last
  writer wins. The prompt grammar steers delegates toward different deliverable
  paths, and this change adds no file locking.
- The cargo target-directory lock still serializes the build steps inside
  concurrent dispatches. Parallelism applies to model calls, not to cargo.
- Trace and ledger lines from concurrent dispatches interleave in append
  order. Every line names its role and task, so append-order reading stays
  sound.
- The tree-wide nightly-1.100 `rustfmt` drift across untouched files is a
  toolchain artifact, out of scope here. Commits format only the files
  they touch.

## Measurement protocol

Measure parallel dispatch on closed rounds only. Pair one pre-change round
against one post-change round at equal compile rate and equal test pass
rate, then compare `wall_seconds`. Numbers come from post-unpause rounds.
The campaign paused on 2026-10-02, so this ADR states the protocol and no
results.
