# 0032. Mandatory config-file CLI

- **Date:** 2026-10-11
- **Status:** accepted
- **Related:** [0031](0031-mas-core-conductor-over-omp-transport.md)

## Context

The harness grew per-project configuration, layered heavily with
implicit conventions spread loosely from agent behavior into
environment and code defaults. Missing one often made a misleading
run pass validation: defaults guessed nonchalantly at problem
shape, languages, budgets, and critical thresholds, hiding run bias
and masking operator intent.

Meanwhile, `.omp/rules/config.md` and `.omp/rules/no-tuning.md`
established two principles: bubble every setting up to a call
site, and forbid hand-tuning baked against one problem instance.
A discoverable form that violates neither principle needs every
per-run decision to live in exactly one inspectable place.

## Decision

`bin/cli` accepts a run configuration file path as a mandatory
argument. No default file name applies, and no fallback configuration
path ever runs a modernization implicitly. Any invocation lacking
a config file path fails with an actionable error pointing to
`configs/example.yml`.

The config schema (`lib/run-config`) separates validation findings
from deserialization errors. Findings aggregate: the operator fixes
the whole file in one pass.

## Consequences

- Scripts driving ARCMiS must name their configuration explicitly:
  `mise run cli -- configs/example.yml`. Every run leaves one
  diffable config file as its record.
- Ad-hoc exploration costs one extra file. That cost buys
  reproduce-by-inspection: feeding the config file back to the CLI
  recreates the run.
- Partial files slow down nobody silently. Aggregated findings cite
  every rejected field at once, so one invocation surfaces every
  fix needed.
- One rule covers unknown input: any key or section outside the
  schema fails validation. `serde(default)` applies only to the
  documented optional sections when they are absent. A typo such as
  `laya.checkpointdir` surfaces as a preflight finding, never as
  silent fallback behavior.
