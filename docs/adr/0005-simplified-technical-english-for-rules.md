# 0005. Simplified Technical English for rules and agent text

- **Date:** 2026-09-14
- **Status:** accepted

## Context

AI agents read rules, comments, and messages without a human present to resolve ambiguity. Dense or hedged text invites misreading. Misreading has a real cost: a misparsed rule produces wrong work. ASD-STE100 is the established controlled-language standard for text that must not be misread. The `asd-ste100` skill (v0.4.0, installed at `~/.omp/agent/skills/asd-ste100` from `danyuchn/asd-ste100-skill` via skills.sh) encodes its structural rules with a mechanical linter.

## Decision

- All rules under `.omp/rules/`, the agent guide (`AGENTS.md`), agent-facing comments, and messages follow ASD-STE100. The rules: active voice, one instruction per sentence, no semicolons, short sentences, no phrasal verbs, no synonym rotation.
- Strict mode applies to rules and instructions. STE-flavored mode applies to READMEs and explanatory prose.
- The mechanical gate is `python3 ~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py` over those files. Current state: 0 violations.
- Lexical rules (the official ~900-word dictionary) are a direction of travel only. The skill does not redistribute the ASD dictionary. The linter's structural checks are the enforceable subset.

## Consequences

- New rules and agent-facing text must pass the linter before commit. Authors run it on changed files.
- Some precision costs length: where a longer phrase carries a hedge or a scope qualifier, keep the phrase (the skill's own rule).
- The linter is advisory tooling, not a repo gate. The commits rule still governs what may be committed.
