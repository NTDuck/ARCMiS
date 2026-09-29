# ADR 0024: Autooptimise campaign close-out and rotation-validation limit

Date: 2026-09-29

## Status

Accepted (campaign closed)

## Context

The autooptimise meta-harness loop (ADR 0020) ran 14 cycles
(base through c14) over 4 days on `smtek/Swift-Qwen3.8-27B:map-k4v`,
46+ scored runs, 8 problem sets. The champion
(`20260928T173947Zc10-c10b-oxidizer-checkdigit`, pass 1/1, 48 tests) is
a translated Rust crate that compiles and passes its suite; two
oxidizer workspaces and one nandc lineage are toolchain-verified.

## Decision

Close the harness-iteration loop. Convergence per the 3-cycle rule:
checkdigit plateaued across c11-c13 (compile passes, no depth gain),
fileupload is parked after 6 runs (2/3 Discovery failures), and the
nandc champion is stable. The rotation-validation cycle (c14) ran two
fresh sets (alphatrans commons-cli, crust leftpad) under the exact
tuned defaults and both stalled before Contract - generality of the
tuned harness is NOT established and is recorded as the campaign's
honest limit.

The campaign record lives in `.artifacts/experiments/SUMMARY.md`
(gitignored); this ADR carries the durable conclusions into the repo:

1. Ceiling policy: translator-only 16384 override dominates all-roles
   overrides; 8192-burn retries cost 1-2 min, 16384-burn retries 10-20.
   Total wrapper throughput, not per-turn survival, is the metric.
2. The wrapper timeout is not the binding constraint; the model's
   round-budget burn rate is.
3. Traces and score paths must count tool-call activity; empty text on
   a tool-only turn is normal rig behavior.
4. 16384 num_ctx is a real Discovery wall; 32768 with read-gating
   clears it; 65536 adds nothing.
5. The tuned harness wins on small single-purpose C sets; it has not
   cleared Discovery on any java alphatrans set.

## Consequences

- Future harness work should start from the c14 evidence: any new
  harness idea must first clear Discovery on a java alphatrans set
  before it can claim generality.
- The ceiling-policy finding transfers to any future model: re-derive
  per-model, do not assume 16384 dominates.
- The champion and verified workspaces remain the evaluation baseline
  for any resumed sweep.
