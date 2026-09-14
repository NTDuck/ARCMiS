---
description: Log each decision that the code does not show. Put a why comment at the site and a numbered ADR in docs/adr. Future sessions can then trace it.
---

# Decision Logging

Log every historical decision that the code does not show. Future sessions must be able to trace it. Write the record twice:

1. **In code** — put a comment at the site. State why the code is this way. Do not restate what the code does. A reader six months later must reconstruct the decision from the comment. Link the commit, issue, or upstream source that motivated it, when one exists.
2. **In docs** — write a numbered Architecture Decision Record under `docs/adr/`. See `docs/adr/README.md` for the format.

Do not comment obvious code. Decision comments exist for choices that a competent reader would question.

## 1. When a Decision Changes

- Never edit an accepted ADR. Write a superseding ADR that links back. Mark the old one superseded in its status line and point to the new number.
- When a decision reverses or a pin moves, update the site comment in the same change. Delete the comment if it no longer applies. A stale comment is worse than no comment.

## 2. ADR File Convention

- Use one file per decision: `docs/adr/NNNN-kebab-case-title.md`. Start numbering at `0001-`. Give each new file the next number.
- Do not rewrite an accepted record. Supersede it.
- Keep each record to about one page. Link supporting material. Do not inline it.

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

- `// Pinned to 2026.8.250 so the store hash stays reproducible; bump both strings together.` Add an ADR when the choice was architectural.
- `// 2026-09-14: 4d keep-since retained every gen under heavy iteration; tightened to 1d.`

## References

- [OpenAI: Harness engineering](https://openai.com/index/harness-engineering/) — the repository is the record of decisions. Decisions live in the repo, not in chat threads or in people's heads.
- [Martin Fowler: ArchitectureDecisionRecord](https://martinfowler.com/bliki/ArchitectureDecisionRecord.html) — the ADR form: short, numbered, immutable. Supersede a record. Do not rewrite it.
