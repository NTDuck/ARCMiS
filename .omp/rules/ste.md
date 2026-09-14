---
description: Write every message, comment, and wording per the asd-ste100 skill. Run its linter on touched files. Zero hard violations before commit.
---

# Simplified Technical English

Write all agent-facing English per the `asd-ste100` skill (installed at `~/.omp/agent/skills/asd-ste100`, from `danyuchn/asd-ste100-skill` via skills.sh).

## Scope

- Rules, code comments, doc comments, commit messages, PR titles and bodies, error and log messages, ADRs, README files, and inter-agent text.
- Strict mode applies to procedures, instructions, and messages. STE-flavored mode applies to READMEs and explanatory prose. The skill defines both modes.

## Rules (structural, per the skill)

- Use active voice. Name the actor.
- Put one instruction in each sentence. Keep instructions at or below 20 words. Keep descriptions at or below 25 words.
- Do not use semicolons. Split the sentence.
- Do not use phrasal verbs. Use the single plain verb. Write "start", not a two-word verb.
- Do not rotate synonyms. Use one verb for one action, one name for one thing.
- Keep the author's hedges. Do not promote "may" to a fact. Confidence is content.
- Do not stack more than three nouns. Rewrite long noun clusters as a phrase with "of" or as a relative clause.

## Gate

- Before you commit, run the linter over each touched file:

```bash
python3 ~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py <files>
```
- For Rust files, lint the prose lines only (comments and doc comments). Rust code needs its statement semicolons. Extract them and lint the extract:

```bash
grep -E '^\s*//+!?\s?' src/main.rs | python3 ~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py
```

- Lint Markdown, TOML, and feature files in full. Code tokens in those files (for example a `;` in a snippet) follow the surrounding language, not this rule.
- The run must report 0 hard violations (exit 0). Advisory findings (passive voice, compound tenses) need a look. Fix them when the fix loses no meaning.
- Where a longer phrase carries a hedge or a scope qualifier, keep the phrase. Record the exception in the commit body.

## Reference

- ADR 0005 records the adoption decision and its context.
- Skill source: [danyuchn/asd-ste100-skill](https://github.com/danyuchn/asd-ste100-skill).
