# ADR 0028: Per-round laya triage with an observe/enforce policy

## Status

Accepted

## Context

ADR 0027 instruments each member dispatch with the laya typed-decision
judge, but the verdicts only land in the ledger. The bottleneck evidence
from the v3 sweep corpus (see `scripts/bottlenecks.py`) shows runs that
burn thousands of wall seconds after the failure mode is already
determinable from the round's own counters: stalled-round runs, runs with
repeated output-cap and max-turns deaths, and runs whose context length
failures keep recurring. Nobody reads the ledger mid-run, so the
instrumentation cannot act.

The typed-decisions checkpoint is fine-tuned on exactly four workflows.
The round consultation must stay inside the trained
`agent_trace_observability` schema: same five questions, same labels, one
`predict` call per round. The round evidence maps into the trained state
field names (`phase`, `passed`, `output`); the `role` field carries the
run subject rather than an agent role, which is a documented fidelity
limit. Measured round-level confidences on closed sweep rounds cluster
near 0.31-0.33, below the 0.9 cascade threshold, so under the default
cascade the round judge falls back on today's real data. That fidelity is
reported as measured; the state mapping was not tuned to force agreement
(see the no-tuning rule).

## Decision

1. **Round evidence as a pure read** (`RoundEvidence` in
   `lib/orchestrator/src/round_triage.rs`): counters scanned from
   `run/ledgers/*` (stalled rounds, max-turns deaths via `MaxTurnsError`,
   output-cap deaths via `finish_reason=Length`, escalations,
   context-length failures, total failure rows) plus the aggregate fields
   (phase reached, completed, stop reason, round/task counts, wall
   seconds). Missing files count as zero; nothing is fabricated.

2. **Round consult** (`RoundTriage`): one laya `predict` on the round
   state with the byte-identical trained question map and prediction
   mapping reused from ADR 0027's dispatch path. The confidence cascade
   is unchanged. In-harness the consult fires once per orchestrator round
   with the cumulative counters; the read-only `harness triage
   <experiment-dir>` subcommand consults over closed rounds and emits one
   JSON line per consult through the cli_sink pattern.

3. **Ledger observation**: a decided round verdict appends a
   `jev_round_triage` observation with the round metrics and the verdict
   fields, so the proposer-side scans can join them like
   `jev_triage` dispatch rows.

4. **Policy knob** (`mas.jev_triage.policy`, `observe` default): under
   `observe` the behavior is unchanged. Under `enforce`, a confident
   (`>= confidence_threshold`) verdict with the trained `action=stop`
   label or the `outcome=harmful` label ends the run (round level) or the
   lead batch (dispatch level) early and appends a failure row with
   category `triage`. The gate is a pure function over the consultation,
   tested without a checkpoint. `Fallback` verdicts never stop anything.

5. **Request timeout** (`run.request_timeout_secs`, default 0): bounds
   each model-call HTTP request on the shared reqwest client built in
   `Provider::client`. 0 preserves the pre-knob behavior (no timeout). A
   timed-out request surfaces as a completion error on the existing
   transient-retry/backoff path in the round loop. This bounds the silent
   300-800 s inference turns observed on the repairer in v3s6.

## Consequences

- The default configuration is behaviorally identical to the pre-ADR
  harness: policy observe, timeout 0. Rollout and any A/B remain with the
  campaign session; the sweep templates are not changed here.
- The round judge's measured fidelity on closed rounds is low (fallback
  under the 0.9 cascade). The enforce policy only acts when the judge is
  confident, which keeps the worst case of enforce at "stops a run the
  counters also condemn".
- `scripts/bottlenecks.py` gives the proposer a deterministic ranked
  bottleneck scan over all `*v3s*` experiments; unknowns are printed as
  `unknown`, never guessed.
