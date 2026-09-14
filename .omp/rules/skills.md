---
description: For any task that uses a distilled skill graph (for example rig), read its SKILL.md first. Verify every upstream symbol before you write. Audit the result claim by claim. Never write framework code from memory.
---

# Distilled Skill Usage

This rule applies to every skill graph under `.omp/skills/` (each router `SKILL.md` plus its `sub-skills/`). Today the graph is `rig`.

## 1. Route First, Never From Memory

1. Before you write code against a distilled framework, read its router `SKILL.md`. Load exactly one matching sub-skill. Do not load the whole graph.
2. If no sub-skill matches, record the gap. Do not improvise from memory.
3. If a task spans two sub-skills, load both. Treat each as a separate verification duty.

## 2. Verify Before You Write
- Back every upstream symbol in your change with one of these sources:
  - The sub-skill Ground Truth section and its `upstream/...` citation.
  - A direct verification of the pinned source: the upstream checkout, `docs.rs` at the pinned version, or compiling the snippet.
- Grep or read the pinned upstream before you trust a signature. If the pin is stale against the version in use (inspect `Cargo.toml` or `cargo tree`), first re-ground against the real version. Then note the version skew.
- A symbol you cannot verify is a finding, not a guess. Stop. Verify. Then write. Mark anything still unverified `[INFERENCE]` and tell the user.

## 3. Audit Before You Yield

Before you yield work that touches a distilled framework, audit the change the way the AREX outer loop audits an answer:

1. List each upstream API claim in the change: symbol, signature, behavior.
2. Mark each claim **verified** (source, citation, or compile) or **unresolved**.
3. Resolve every unresolved claim. Or hand it to the user with the `unresolved` mark. Do not ship a change with silently unresolved claims.

The compiler is the cheapest verifier. When a snippet is cheap to compile, `cargo check` beats reading.

## 4. Correction Loop

When reality contradicts a skill, the observed behavior wins:

- The compiler rejects a documented signature.
- The upstream API moved.
- A workflow step is missing.

Then, in the same session:

1. Correct the claim in the sub-skill.
2. Update its `Provenance` section with the new upstream commit and date.
3. Add the failure mode to its `## Pitfalls` section.
4. Bump the router provenance pin when the upstream commit changes.

A skill that stays wrong after falsification is worse than no skill. It injects errors with authority. Corrections are part of the task, not follow-up work.

## 5. Provenance Discipline

- Each distilled skill pins an upstream commit in its Provenance section. Claims are valid against that pin.
- If you write code against a different upstream version, first verify the delta (changelog, docs.rs version switcher). Then trust the sub-skill claims. Record the skew in the task output.
- New distillations follow the AREX shape: Scope, Ground Truth (cited), Workflow, Pitfalls, Verify, Provenance. Do not write uncited symbol lists.

## Methodology References

- [VectorSpaceLab/AREX-Skill](https://github.com/VectorSpaceLab/AREX-Skill/) — four-stage distillation (scope, ground, construct, verify), progressive-disclosure router, provenance-pinned repo skills.
- [AREX: Towards a Recursively Self-Improving Agent for Deep Research](https://arxiv.org/pdf/2607.21461) — discovery–verification asymmetry. Inner research loop plus outer audit loop. Unresolved-claim tracking. Compact improvement state.
