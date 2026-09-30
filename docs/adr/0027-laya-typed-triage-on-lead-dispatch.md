# 0027. Laya typed-decision triage on lead member dispatch

- **Date:** 2026-09-30
- **Status:** accepted
- **Extends:** [0023](0023-laya-backed-jev-judge.md). The cascade rule, the
  confidence statistic, and the local-checkpoint policy carry over. This
  ADR moves the same judge shape to the lead inner loop and to the
  typed-decisions checkpoint's own trained workflow.
- **Supersedes:** nothing (ADR 0023 remains accepted)

## Context

The lead's inner loop (ADR 0026) dispatches one member per round and reads
the result back as text. Every classification of that result — did the
member fail, does it need a human look, how risky is it — currently costs a
lead LLM turn, and model turns dominate the harness's wall time.
The `convaiinnovations/laya-typed-decisions` checkpoint (ModernBERT-large,
421M params, staged at `assets/models/laya-typed-decisions/`, ~850 MB
safetensors) answers constrained questions in one forward pass: no token
decoding, microseconds-to-seconds per call (measured ~3.4 s CPU for all
five questions on one state, once per dispatch, not per turn).

The checkpoint is a specialist. It is fine-tuned on exactly four workflows
of the `LocalLLaMA/typed-decisions` dataset, and its model card warns that
off-workflow question schemas produce base-checkpoint behavior (accuracy
~0.36 on the benchmark's random-guess floor). One of the four trained
workflows is `agent_trace_observability`: it triages agent-run traces —
exactly the shape of a lead's member-dispatch result.

## Decision

Add `JevTriage` (`ARCMiS/lib/orchestrator/src/jev_triage.rs`): a wrapper
over `Option<Arc<laya::Agent>>` that answers all five
`agent_trace_observability` questions about one member dispatch result in
one `predict` call. The lead's inner loop consults it after each
`run_single` result.

- **The question map is the checkpoint's trained schema, not a harness
  invention.** The five question ids (`action`, `needs_review`, `outcome`,
  `risk`, `urgency`), their types (`choice`, `noul`, `choice`, `score`,
  `score`), and every label and rubric level come verbatim from the
  trained workflow (`LocalLLaMA/typed-decisions` dataset,
  `agent_trace_observability/` split), verified against the staged
  checkpoint by running it. A custom schema on this checkpoint would be
  off-workflow and produce garbage — documented limit, not a workaround
  target. Custom schemas require fine-tuning, which is outside the
  harness's scope.
- **One forward pass, five answers.** `Agent::predict` takes the state and
  the whole question map once (`references/src/agent.rs`, `predict` ->
  `predict_map`). `TriageVerdict` maps all five answers: outcome
  (`success`/`partial`/`failure`/`harmful`), `needs_review` as P(true) at
  or above 0.5, `risk` and `urgency` as argmax rubric levels 0-3, `action`
  (`continue`/`observe`/`human_review`/`stop`), and a confidence.
- **Confidence is the minimum top-label probability across the five
  answers.** The paper's `q` (arXiv:2609.26550 §4) per answer, gated on the
  weakest: a verdict is only as good as its least sure answer. The
  statistic reuses `jev_judge::confidence` (max label probability).
- **Cascade with fallback = current behavior.** Below τ, on any load or
  inference error, on a missing question id, or on a foreign label, the
  consult reports `Fallback` and the lead loop proceeds byte-identically
  (arXiv:2609.26550 §7: invalid and unsure outputs always defer). τ
  defaults to 0.9 like ADR 0023. The checkpoint's model card flags ECE
  0.213 (over-confidence), so per-workload τ validation is the operator's
  task.
- **Deterministic tier first.** The consult is advisory and follows the
  deterministic pass/fail judgment that `run_single` already produced. The
  triage never changes a verdict. It classifies alongside it.
- **Checkpoint sourcing is local-only.** `JevTriageConfig.checkpoint` is a
  local directory loaded with `Agent::from_dir`. The `hub` feature stays
  off (ADR 0023): sandboxed runs must not fetch models. An enabled config
  with an empty or missing checkpoint logs a warning once at construction
  and disables the judge. The staged weights live under `assets/models/`,
  and .gitignore covers that directory: checkpoints never enter git.
- **Round 1 is instrumentation only.** A `Decided` verdict on a `fail`
  result with `needs_review=true` lands as a `jev_triage` ledger
  observation with the full verdict fields, and a tracing log with the
  structured verdict. Dispatch-policy changes (routing on `stop`, urgent
  re-dispatch) are a deliberate follow-up so a campaign can A/B the
  signal before the loop acts on it.
- **Built once per run.** The harness constructs `JevTriage` at preflight
  and threads it to the loop. The judge is `Clone` over `Arc<Agent>`, so
  the 850 MB checkpoint loads once per run, not per batch or per dispatch.

## Consequences

- Default config keeps prior behavior exactly: `mas.jev_triage.enabled` is
  `false`, no laya code runs, and the ledger and traces are byte-identical.
- A label-set mismatch between the question map and a future checkpoint
  turns every consult into a fallback. The mapping is pure and unit tested
  without a checkpoint (question ids must equal the workflow signature,
  labels must equal the trained sets), so a schema change surfaces in
  tests first.
- The question map hardcodes the trained schema in a const citing the
  staged `rl_agent_config.json`-backed dataset split. If the checkpoint is
  replaced by a differently fine-tuned one, the map must be re-derived
  from that checkpoint's training data.
- The judge reads a capped output head (2000 chars) of the dispatch result
  as the trace summary. Long outputs lose their tail to the cap, and the
  pass/fail signal comes from the deterministic judgment, not from the
  model.
- The state field names (`role`, `phase`, `passed`, `output`) are the
  harness's, not the training set's (`agent`, `task`, `trace_summary`).
  The benchmark measured the workflow's accuracy (0.730, lowest of the
  four) on training-shaped states. On harness-shaped states the numbers
  are
  unvalidated. The A/B ledger observation exists to measure exactly this
  gap before the loop trusts the verdict.
