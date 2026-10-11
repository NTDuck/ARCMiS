---
name: laya
description: "Operating knowledge for the laya-rs crate (typed decisions: single-forward-pass choice/score/noul answers over Candle) — the references/ corpus is the reference. Taken verbatim from the published crate. Read the closest module before you write laya code. Use when classifying, scoring, routing, or answering constrained questions without an LLM: judge outputs, rubric grades, gate decisions, language routing."
---

# laya — Typed Decisions in Rust (laya-rs)

This skill holds **no distilled ground truth**. The reference material is the
published `laya-rs` crate source, copied **verbatim** under `references/` —
every file byte-identical to the version pinned below. `src/` IS the API: the
crate ships no `examples/` directory, so `tests/agent.rs` and
`tests/model_parity.rs` are the usage corpus.

The package is `laya-rs` (the crates.io name; `laya` was taken). The library
target is `laya`, so imports read `use laya::…`.

## Provenance

- Corpus: `laya-rs` 0.1.0 crate source from crates.io
  (checksum `c651803bc2cf3a7c30730ccc0b45b831a3162d72f9851149befb9e29e0b25c05`),
  published 2026-09-20. Upstream repo `bob-rietveld/laya-rs` (a port of
  [mizorewww/laya-mlx](https://github.com/mizorewww/laya-mlx) from
  [Convai Innovations' Laya](https://huggingface.co/convaiinnovations/laya);
  neither endorses the port). Re-capture before writing against a different
  version.
- Refresh: download
  `https://crates.io/api/v1/crates/laya-rs/<version>/download`, extract, and
  copy `src/ tests/ README.md Cargo.toml` into `references/` verbatim, then
  update the pin here and in `references/PROVENANCE.txt`.

## What laya is (for tool selection)

A bidirectional ModernBERT encoder with an RL-trained decision head answers
**constrained questions** — `choice`, `score`, `noul` (proposition
probability) — in **one forward pass**: no token decoding, no generated JSON
parsing, **0 output tokens**, microseconds per call. Use laya where an LLM
judge would burn a full decode for a label. Do not use it for open-ended text
generation — it cannot generate text.

## Layout

```
SKILL.md            ← this file (navigation only)
references/         ← verbatim crate source (see PROVENANCE.txt)
  src/lib.rs        ← re-exports: Agent, AgentBuilder, Prediction, Answer,
                       Router, Router route/ModelName, presets, question…
  src/agent.rs      ← AgentBuilder (dtype/device/batch_size/cache_prompts),
                       Agent::from_pretrained/from_dir/predict/predict_map
  src/hub.rs        ← from_pretrained (Hub id or local path), resolve_model
  src/router.rs     ← Router: language detection → ModelName (English,
                       Multilingual, TypedDecisions), RouteDecision
  src/model.rs      ← DecisionModel internals (only if touching weights)
  tests/agent.rs    ← END-TO-END USAGE: the three question types, answers
  tests/fixtures/   ← tiny checkpoint for tests (not a real model)
  README.md         ← upstream quickstart + CLI
```

## How to use

1. **Read `references/tests/agent.rs` first.** It is the closest thing to an
   example suite: builds an `Agent` from a checkpoint and answers all three
   question types.
2. **Copy the pattern, not the code.** Verify each symbol against this corpus
   (grep under `references/src/`) or the version you compile against. The
   compiler is the cheapest verifier.
3. **Never write a laya symbol from memory.** No symbol in your diff may exist
   only in a model prior. Ground it here.

## Cheat sheet: task → source

| Task | Read first |
| --- | --- |
| Load a model (Hub id or local dir) | `src/hub.rs` `Agent::from_pretrained`, `from_pretrained_with` |
| Build with dtype/device/batching | `src/agent.rs` `AgentBuilder` |
| Answer choice / score / noul | `tests/agent.rs`; `src/agent.rs` `predict`, `predict_map` |
| Read answers | `src/agent.rs` `Answer` (`choice`/`score`/`noul`/`probabilities`/`confidence`), `Prediction::answer(id)` |
| Question schema (`Choice`/`Score`/`Noul`) | `laya_core::question` (re-exported at `src/lib.rs`) |
| Route by language to a checkpoint | `src/router.rs` `Router::route`, `Router::load`, `preload`, `ModelName` |
| Convert an MLX/PyTorch checkpoint | `src/convert.rs`, CLI `laya-rs convert` |
| Prompt construction internals | `laya_core::prompt`, `presets` (via `src/lib.rs` re-exports) |
| Batch many states | `AgentBuilder::batch_size`, `pad_to_multiple`, `cache_prompts` |

## ARCMiS fit

Judge-shaped roles (Validator, Critic, FleetAnalyst) currently burn full LLM
turns on label answers. A laya `Agent` answers those in microseconds once a
checkpoint is converted and loaded. The `hub` feature (default) downloads
checkpoints on first use; pin a local path via `Agent::from_dir` in
sandboxed/CI runs.
