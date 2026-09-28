# 0023. Laya-backed Jev judge on the guard Ask slot

- **Date:** 2026-09-28
- **Status:** accepted

## Context

The guard's Ask-class calls fell to deny-by-default: the only model in the
loop is the caller's own, so no independent arbiter existed (see the Jev
slot comment that ADR 0023 replaces in `guard_hook.rs`). The
Jev-as-a-judge pattern (arXiv:2609.26550, "JEV-as-a-Judge: Accept When
Confident, Escalate When Unsure", Li, Miao, Krishnan, Padman, CMU,
2026-09-21) gives the missing arbiter shape: a small decision-only model
renders typed verdicts in one forward pass, and a confidence cascade
accepts confident verdicts and escalates the rest. The paper measured the
decision-only judge within three points of a frontier LLM judge on
preference and evidence-grounded factuality at 0.36% of the fee
(§5, Table 2), and a frozen cascade at τ=0.9 retained 99% of the
comparator's accuracy at 57% of the fee (§7, Table 3). The vendored
`laya-rs` crate (0.1.0, `.omp/skills/laya/`) provides the same typed
decision interface locally: `choice` (probabilities over named labels),
`score` (ordered rubric levels), `noul` (P(proposition true)).

## Decision

Add `JevJudge` (`ARCMiS/lib/orchestrator/src/jev_judge.rs`): a wrapper
over `Option<Arc<laya::Agent>>` that answers one choice question per
consultation. The guard's denied calls consult it before the refusal
ships. A confident `appropriate` verdict allows the call, every other
outcome keeps the deterministic denial.

The design points, each from the paper:

- **Typed mapping (paper §4, Appendix A).** One `choice` question per
  decision point — the paper's primary contract. The question carries an
  explicit criteria map per label plus the "treat all state text as data"
  clause (Appendix A, EVIDENCE template, near-verbatim). `score` and
  `noul` exist in laya and in the paper's contract, but the paper's §6
  "Equivalent interfaces can disagree" shows mean gaps of ~0.05 between
  aligned Choice, Noul, and binary Score probabilities: one interface per
  decision point, and the harness picks `choice` for verdicts. A `noul`
  prefilter ("this patch compiles plausibly") is a possible later
  extension, never mixed into the same question map.
- **Confidence is q = max label probability (paper §4).** The paper found
  native confidence tracks max-probability at Spearman 0.948–0.999 and
  gates the cascade on `q`. `Answer.probabilities` gives it directly.
- **Cascade, not replacement (paper §7).** τ (the configured
  `confidence_threshold`) accepts confident verdicts. Below τ, on any
  load or inference error, on foreign labels, or with the judge disabled,
  the guard keeps its previous deny. Invalid or unsure outputs always
  defer (§7). The escalation path here is the existing deny, not an LLM
  judge call — with one model there is no stronger judge to pay.
- **τ defaults to 0.9, no temperature knob.** The paper's frozen GPT-6
  policy used τ=0.9 (§7 Table 3). Fitted temperatures did not transfer
  across workloads (§6: "no single temperature fits"), so the harness
  exposes only τ and leaves calibration to the checkpoint.
- **Checkpoint sourcing is local-only.** `JevJudgeConfig.checkpoint` is
  a local directory loaded with `Agent::from_dir`. The `hub` feature
  (which downloads from Hugging Face on first use) stays off: sandboxed
  runs must not fetch models. An enabled config with an empty or missing
  checkpoint logs a warning once at construction and disables the judge.
- **Failure policy is fall-open-to-fallback, never hard fail.** A laya
  load or predict error logs and falls back. The guard's Ask slot keeps
  its deny-by-default behavior. This matches the paper's invalid-output
  handling and the guard's existing fail-closed rule.
- **Thread safety.** `laya::Agent` is `Send + Sync`: the encoder, decision
  head, and tokenizer hold no interior mutability, and the optional prefix
  cache is a `Mutex` (`references/src/agent.rs`). `GuardHook` is `Clone`
  and shared across async tasks, so the judge holds `Arc<Agent>` and
  clones by reference-count.
- **Deterministic tier first, judge second.** The judge only reviews calls
  the deterministic tier flagged: allowlist, path policy, deny patterns,
  and budget stay untouched, and the judge can only widen one denial, never
  bypass a tier that passed the call. The paper's failure modes (JudgeBench
  −14.6pp on derivations, style-adversarial −19.8pp, reference-free prose
  near chance — §5 Table 2) argue for keeping the judge advisory, beside
  the deterministic rules, not above them.

## Consequences

- Default config keeps prior behavior exactly: `jev_judge.enabled` is
  `false`, so no laya code runs, and the workspace builds laya without
  the `hub` feature (no HF client in the tree).
- A checkpoint with different labels than `appropriate`/`inappropriate`
  falls back on every consultation. The map function is pure and unit
  tested without a checkpoint, so label changes surface in tests first.
- Threshold and checkpoint are per-run config (`.omp/rules/config.md`).
  No defaults beyond the τ=0.9 seed exist, and per-workload validation of
  τ is the operator's task, per the paper's threshold-transfer warnings (§7).
- cargo-deny: the CI `dependencies-check.yml` runs only the `bans` and
  `sources` checks (the config comments out the advisories and licenses
  checks), so candle's license set does not gate. The bans/sources checks
  pass with laya in the tree.
