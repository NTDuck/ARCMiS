# 0003. Distill rig into AREX-style skill graph

- **Date:** 2026-09-14
- **Status:** accepted

## Context

Agentic development against the rig Rust LLM framework hallucinates APIs when done from memory: upstream ships standing breaking changes (their README warns "future updates will contain breaking changes"), the surface spans 29 crates, and model priors lag the moving API. The AREX methodology (VectorSpaceLab/AREX-Skill four-stage distillation; AREX arXiv:2607.21461's discovery–verification asymmetry and constraint-wise outer audit) exists precisely for this: capture verified operating knowledge once, then make every future use re-ground against it.

## Decision

- Distill `0xPlaygrounds/rig` into an AREX-shaped skill graph at `.omp/skills/rig/`: a router `SKILL.md` + 7 sub-skills (`agents`, `tools`, `rag-vector`, `extraction`, `multi-agent`, `memory`, `providers`), each with Scope / Ground Truth (per-claim `upstream/...` citations) / Workflow / Pitfalls / Verify / Provenance.
- Pin the graph to upstream commit `6828097ce102fbb4e26d3aeadd50201cc65c4150` (2026-09-14). Claims are valid against that pin; drift requires re-grounding, and the falsification loop in `.omp/rules/arex-skill-utilization.md` updates the skill in-session.
- Sub-skills are written by parallel scoped agents, each owning exactly one file, each required to grep-verify symbols against the pinned checkout before writing (AREX "ground" stage).
- A binding utilization rule (`.omp/rules/arex-skill-utilization.md`) makes routing + grounding + constraint-wise audit mandatory for any rig task, so the graph cannot silently rot into hallucination fuel.

## Consequences

- Future sessions writing rig code start from verified, cited operating knowledge instead of memory; the compiler + pinned checkout remain the cheap verifiers.
- The pin will go stale as upstream moves; the utilization rule's RSI loop and Provenance Discipline section make refreshing a governed action rather than silent drift.
- The same pattern extends to future frameworks: distill into `.omp/skills/<framework>/` with a router, and extend the rule's coverage list.
