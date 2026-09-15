---
description: No hand-tuning against the problem set. General solutions only — no magic constants, special cases, or prompts tuned to make one known task pass.
---

# No Problem-Set Tuning

Build solutions against the problem class. Do not tune against the specific problem instance in front of you.

## Rules

- No magic constants derived from one known input. Do not choose a per-file byte cap because the current sources are small. Do not tune turn counts until one run passes. Do not write paths or names that only fit the demo case.
- No special-casing known inputs: an `if` branch that exists to handle one file in the input set is a violation. Handle the class, not the instance.
- No prompt or rule text that names specific files, models, or outcomes of the current test case to steer one run. General instructions only. Run-specific values belong in the config file. The config describes the problem instance. That is its job, and its only job.
- Before you merge a fix, ask: does this change hold for a different codebase, a different model, a different file set? If not, redesign until it does.
- Measured model facts (for example: a model runs away in think blocks) are legitimate general knowledge. Record them as facts. Apply them as defaults for that model class. Do not hide them as switches for one run.

## Permitted Practices

- Values that describe the problem set live in the run config: model, budgets, languages, paths. This is configuration, not tuning.
- Algorithmically derived limits work fine: a percentage of context size, a count from the input. State the formula in the code. An example: cap each input file at a quarter of the context window, converted at 4 bytes per token.
- A why comment that references an experiment is fine, when the resulting behavior is general (see `.omp/rules/decisions.md`).

## Violations

- `max_output_tokens: 8192` with the comment "the 2B model writes long files" in a general rule file.
- A prompt that says "the input has a GildedRose.c file".
- A default constant chosen by trial until one run passed, with no general justification.

## Exceptions

- Verified model-behavior facts (documented in ADRs) may inform *defaults* shipped in config files, because config exists to carry per-run values. They must not leak into shared code, prompts, or rules.
