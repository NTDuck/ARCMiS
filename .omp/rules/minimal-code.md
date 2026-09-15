---
description: Write the minimal amount of code. Prefer an external, maintained library over reinventing one. Every wheel re-invented needs a why.
---

# Minimal Code

Write the least code that meets the requirement. Treat every line as a cost.

## Prefer external libraries

- Before you implement a general capability (parsing, serialization, retries, process handling, time, argument parsing), look for a maintained crate and use it. Standard library first, then established crates.
- Do not re-implement what a dependency already provides. Duplicated capability is a maintenance debt with two owners.
- A small amount of glue code around a library is normal. A private re-implementation of the library is not.

## When you must write it yourself

- The capability is specific to this repository and no crate covers it.
- Write the smallest version that works. Add no speculative parameters, no unused modes, no "later" branches.
- Every hand-rolled general capability carries a why comment: what you evaluated and why the library route lost. See `.omp/rules/decisions.md`.

## Tests and examples follow the same rule

- Reuse the test utilities that exist. Do not fork a helper per test file.
- Delete demo code the moment it stops demonstrating.

## Violations

- A hand-written JSON walker next to `serde_json` in the dependency tree.
- A retry loop with backoff re-implemented inline.
- Three slightly different file-reading helpers in one crate.
