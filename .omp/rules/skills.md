---
description: For any task that uses a skill with a verbatim upstream corpus (for example rig), read its SKILL.md first. Verify every upstream symbol against the corpus before you write. Audit the result claim by claim. Never write framework code from memory.
---

# Skill Corpus Usage

This rule applies to every skill directory under `.omp/skills/`. Today the
skill is `rig`: a thin router `SKILL.md` plus a verbatim upstream `references/`
corpus (no distilled ground truth).

## 1. Route First, Never From Memory

1. Before you write code against a skill-backed framework, read its `SKILL.md`.
   It routes you to the closest upstream reference example. Do not load more of
   the corpus than the task needs.
2. If no example matches the task, record the gap. Do not improvise from memory.
3. If a task spans several examples, read each. Treat each as a separate
   verification duty.

## 2. Verify Before You Write
- Back every upstream symbol in your change with one of these sources:
  - The verbatim upstream corpus at `.omp/skills/<name>/references/` and its
    pinned commit.
  - A direct verification of the pinned source: the upstream checkout, `docs.rs` at the pinned version, or compiling the snippet.
- Grep or read the pinned upstream before you trust a signature. If the pin is stale against the version in use (inspect `Cargo.toml` or `cargo tree`), first re-ground against the real version. Then note the version skew.
- A symbol you cannot verify is a finding, not a guess. Stop. Verify. Then write. Mark anything still unverified `[INFERENCE]` and tell the user.

## 3. Audit Before You Yield

Before you yield work that touches a corpus-backed framework, audit the change:

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
1. Fix the wrong guidance in the skill, or record why the example misled you.
2. Re-capture the corpus from the new upstream commit (see the skill's Refresh
   procedure) and update its commit pin.
3. Add the failure mode to the router's Pitfalls section when the same trap can
   recur.
4. Tell the user what the skill got wrong.

A skill that stays wrong after falsification is worse than no skill. It injects errors with authority. Corrections are part of the task, not follow-up work.

## 5. Provenance Discipline

- The corpus pins an upstream commit in the router's Provenance section.
  Claims are valid against that pin.
- If you write code against a different upstream version, first verify the
  delta (changelog, docs.rs version switcher, or the version's examples). Then
  trust corpus-derived claims. Record the skew in the task output.

## Methodology References

- [VectorSpaceLab/AREX-Skill](https://github.com/VectorSpaceLab/AREX-Skill/) — four-stage distillation (scope, ground, construct, verify), progressive-disclosure router, provenance-pinned repo skills.
- [AREX: Towards a Recursively Self-Improving Agent for Deep Research](https://arxiv.org/pdf/2607.21461) — discovery–verification asymmetry. Inner research loop plus outer audit loop. Unresolved-claim tracking. Compact improvement state.
