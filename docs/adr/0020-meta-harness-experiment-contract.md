# 0020. Meta-Harness experiment contract

- **Date:** 2026-09-20
- **Status:** accepted

## Context

Meta-Harness (arXiv:2603.28052) optimizes an executable harness through an
agentic outer loop. The proposer reads the full experiment history from
the filesystem: harness source, scores, and raw execution traces. The
ablation shows raw traces, not summaries, carry the diagnostic signal.
For ARCMiS to be Meta-Harness-compatible, every migration run must leave
a complete forensic record, and the mutable harness must stay separated
from the immutable evaluation environment.

The existing bench tooling (`scripts/run-bench.sh`, `scripts/score.py`)
already records per-rep stdout, stderr, result yaml, and a rescored
per-workspace JSON. Three gaps block an optimizer: run records carry no
per-problem identity, no failure taxonomy, and no lineage; the result
yaml holds only an aggregate; and no driver constructs a searchable
experiment tree.

## Decision

- One experiment per directory under `.artifacts/experiments/`:
  `.artifacts/experiments/<candidate-id>/`. The id is the UTC timestamp
  plus a slug, for example `20260920T091500Z-monolith`. The directory is
  immutable after the run: the proposer reads, never writes.
- Each experiment directory holds:
  - `manifest.json`: harness id, method, model, budgets, config path,
    problem set, git revision, parents, hypothesis, created_at.
  - `result/result.yml`: the existing aggregate result document.
  - `result/per_problem.json`: one record per problem (task) with
    success, failure stage, and counts.
  - `traces/turns.jsonl`: one JSON line per model call and tool call,
    emitted by the run log hook.
  - `workspace/`: the translated codebase the run produced.
  - `stdout.log`, `stderr.log`: the captured process streams.
- Failure taxonomy on every problem record, staged by the run order:
  `setup`, `analyze`, `plan`, `translate`, `validate`, `evaluate`.
  `success: true` only when translation completes and the toolchain
  test command passes in the workspace.
- Lineage lives in the manifest: `parents` (experiment ids) and
  `hypothesis` (proposer text). Search scripts never compute lineage;
  the proposer owns it.
- The driver is `scripts/experiment.sh <config> <out-dir>`: one
  experiment, one directory, no aggregation. Aggregation and frontier
  selection stay in `scripts/frontier.py`, which reads only the
  experiment tree. The proposer reads the same tree through its own
  tools; no summary layer sits between.
- The problem set stays outside the optimizer. Configs, scoring rules,
  and the kata assets are fixed inputs. A candidate may change only
  what its own directory carries.

## Consequences

- The proposer gets a stable, greppable history: it can diff two
  candidates, read their traces, and form a causal hypothesis without
  any bespoke tooling from this repo.
- Runs stay ~2 to 3 minutes on the local 27B model, so a 20-iteration
  search with 2 candidates per iteration fits the existing 3600 s
  per-run timeout with headroom.
- We fixed the old `compilation_status` string defect (rc was 1 even
  on success). The agent responses now carry the boolean `compiled`:
  the build gate. The process exit code derives from `compiled`. The
  quality score still comes from the toolchain rerun, which
  `frontier.py` reads.
- The hook currently logs to the console only. Trace capture lands in
  the run log module, so console output and the JSONL stay identical.
