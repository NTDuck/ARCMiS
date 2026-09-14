---
description: Any task touching an AREX-distilled repo skill (e.g. rig) must route through its SKILL.md, ground every upstream symbol before writing, and audit constraint-wise — never write framework code from memory.
---

# Distilled-Skill Utilization (Anti-Hallucination)

Applies to every AREX-style skill graph under `.omp/skills/` (each router `SKILL.md` + `sub-skills/`), currently `rig`.

## 1. Route First, Never From Memory

1. Before writing any code against a distilled framework, read its router `SKILL.md` and load **exactly one** matching sub-skill. Do not load the whole graph.
2. If no sub-skill matches, that is a gap to record — not license to improvise from memory.
3. When a task spans two sub-skills, load both but treat each as a separate grounding obligation.

## 2. Ground Before Write (discovery–verification asymmetry)

AREX's core insight: discovering an API from memory is expensive and unreliable; verifying a candidate claim is cheap and decomposable. Exploit the asymmetry:

- Every upstream symbol in your diff MUST be backed by (a) the sub-skill's Ground Truth section with its `upstream/...` citation, or (b) direct verification against the pinned source: the upstream checkout, `docs.rs` at the pinned version, or a compile check.
- Grep/read the pinned upstream before trusting a signature. If the pin is stale vs. the version actually used (`Cargo.toml`/`cargo tree`), re-ground against the real version first and note the version skew.
- An unverifiable symbol is a finding, not a guess: stop, ground, then write. Mark anything still ungrounded `[INFERENCE]` and surface it to the user.

## 3. Constraint-Wise Audit Before Yield

Before yielding any work touching a distilled framework, audit the diff the way AREX's outer loop audits an answer:

1. List each upstream-API claim in the change (symbol, signature, behavior).
2. Mark each **verified** (source/citation/compile) or **unresolved**.
3. Resolve every unresolved claim or hand it to the user explicitly marked. A diff with silently unresolved claims does not ship.

The compiler is the cheapest verifier: when snippets are cheap to compile, `cargo check` beats reading.

## 4. Recursive Self-Improvement Loop

When reality contradicts a skill — compiler rejects a documented signature, upstream API drifted, a step is missing:

1. The observed behavior wins; the skill text loses.
2. Fix the sub-skill in the same session: correct the claim, update its `Provenance` (new upstream commit/date), add the failure mode to `## Pitfalls`.
3. Bump the router provenance pin when the upstream commit changes.

A skill that stays wrong after being falsified is worse than no skill: it injects hallucinations with authority. Corrections are part of the task, not follow-up work.

## 5. Provenance Discipline

- Distilled skills pin an upstream commit in their Provenance section. Claims are valid against that pin.
- Writing code against a different upstream version than the pin → verify the delta (changelog, docs.rs version switcher) before trusting any sub-skill claim; record the skew in the task output.
- New distillations follow the AREX shape: Scope, Ground Truth (cited), Workflow, Pitfalls, Verify, Provenance. Uncited symbol lists are prohibited.

## Methodology References

- [VectorSpaceLab/AREX-Skill](https://github.com/VectorSpaceLab/AREX-Skill/) — four-stage distillation (scope → ground → construct → verify), progressive-disclosure router, provenance-pinned repo skills.
- [AREX: Towards a Recursively Self-Improving Agent for Deep Research](https://arxiv.org/pdf/2607.21461) — discovery–verification asymmetry; inner research loop + outer constraint-wise audit loop; unresolved-claim tracking; compact improvement state.
