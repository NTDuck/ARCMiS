---
description: Log every historical decision not directly inferrable from code — as a WHY comment at the site and a numbered ADR in docs/adr — so future sessions stay traceable.
---

# Historical Decision Logging (Comments + ADRs)

Every historical decision **not directly inferrable in code** must be traceable in future sessions. Log it twice:

1. **In code** — a comment at the site stating the *why* (the decision, not a restatement of what the code does). A reader six months out must reconstruct the decision from the comment alone; link the commit, issue, or upstream source that motivated it when one exists.
2. **In docs** — a numbered Architecture Decision Record under `docs/adr/` (see `docs/adr/README.md` for the format).

Do not comment obvious code: decision comments exist for choices a competent reader would otherwise question.

## 1. When a Decision Changes

- Never edit an accepted ADR in place. Write a superseding ADR that links back; mark the old one superseded (status line) pointing at the new number.
- When a decision reverses or a pin bumps, update or delete the stale site comment **in the same change**. Stale comments are worse than none.

## 2. ADR File Convention

- One file per decision: `docs/adr/NNNN-kebab-case-title.md`, monotonic numbering, starting `0001-`.
- Immutable once accepted: supersede, don't rewrite.
- Keep each record to roughly one page; link supporting material instead of inlining it.

## 3. Format (per record)

```markdown
# NNNN. <Decision title>

- **Date:** YYYY-MM-DD
- **Status:** accepted | superseded by [NNNN](NNNN-title.md)

## Context
The problem or force that demanded a decision. Name concrete constraints.

## Decision
The choice, stated in one or two sentences.

## Consequences
What this makes easier, harder, or impossible. What to re-evaluate if context changes.
```

## 4. Examples

- `// Pinned to 2026.8.250 so the store hash stays reproducible; bump both strings together.` + ADR when the choice was architectural.
- `// 2026-09-14: 4d keep-since retained every gen under heavy iteration; tightened to 1d.`

## References

- Distilled from [OpenAI: Harness engineering](https://openai.com/index/harness-engineering/) (repository-as-system-of-record; decisions live in-repo, not in chat threads or heads).
- [Martin Fowler: ArchitectureDecisionRecord](https://martinfowler.com/bliki/ArchitectureDecisionRecord.html) (ADR form: short, numbered, immutable, supersede-don't-edit).
