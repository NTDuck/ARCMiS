# 0014. Replace the rig skill graph with a verbatim upstream corpus

- **Date:** 2026-09-18
- **Status:** accepted (supersedes ADR 0003)

## Context

ADR 0003 distilled `0xPlaygrounds/rig` into an AREX-shaped skill graph: a
router plus 7 sub-skills with Ground Truth sections (per-claim `upstream/...`
citations), pinned to upstream commit `6828097` (2026-09-14). The distillation
went stale within days: upstream `main` moved to `9b94481` (2026-09-17) and its
examples already showed API drift against the pin (`OpenAI::from_env()?.bound()?`
returning `driver::Bound`, two-arg `client.embedding(model, None)`, wire types
under `anthropic::wire`/`ollama::wire`). The dangling `upstream/` checkout
reference inside Ground Truth citations had also rotted. Maintaining
hand-written Ground Truth against a fast-moving upstream costs continuous
re-grounding work and still drifts.

## Decision

- Delete the 7-sub-skill distillation. The rig skill becomes a thin router
  `SKILL.md` plus `references/`: the upstream `examples/` tree copied
  **byte-for-byte** from upstream `main` at capture commit `9b94481`
  (2026-09-17).
- The router carries navigation only: provenance pin, layout, usage steps, a
  task → example cheat sheet, and a refresh procedure. It makes no API claims.
- Ground Truth now lives where upstream maintains it: runnable examples plus
  `references/README.md`. "See source" in upstream prose resolves locally.
- I reworded `.omp/rules/skills.md` from "distilled skill graph" to
  "verbatim upstream corpus" semantics. The verification duty did not change:
  every upstream symbol in a diff must be grounded in the corpus or verified
  against the rig version in use.

## Consequences

- Zero maintenance against API drift inside the corpus: re-capture is a copy
  plus a commit-pin bump (procedure in the router).
- The skill no longer answers "how does X work" without reading source. Agents
  must read example code. The compiler remains the cheapest verifier.
- The AREX distillation pattern stays available for stable frameworks, but rig
  moves too fast for it. If a future skill distills again, it must re-justify
  the maintenance cost against this ADR.
