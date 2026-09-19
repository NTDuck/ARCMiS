---
name: arcmis-autooptimise
description: Drive Meta-Harness optimization over ARCMiS migration harnesses: one experiment per directory under .artifacts/experiments/, proposer reads history, harness fixed, evaluator fixed
---

# ARCMiS autooptimise (Meta-Harness outer loop)

Optimize the ARCMiS migration harness the way Meta-Harness
(arXiv:2603.28052) prescribes: propose an executable harness change,
run it as one experiment, leave the full forensic record on disk, and
read the history back before the next proposal. The model under
optimization is fixed. The problem set is fixed. You, the proposer,
own every diagnosis and every hypothesis.

## The contract (ADR 0020)

- One candidate = one immutable directory `.artifacts/experiments/<id>/`:
  `manifest.json` (method, model, budgets, parents, hypothesis),
  `config.yml`, `traces/turns.jsonl` (one JSON line per model call,
  tool call, tool result), `workspace/` (the translated codebase),
  `stdout.log`, `stderr.log`, and, after scoring, `result/`.
- The proposer reads anything in the tree through its own tools. The
  proposer never writes into an existing experiment directory.
- Success is decided by a toolchain rerun in the workspace
  (`cargo build` + the config test command). The harness exit code
  reports the agents' build verdict only.

## One experiment

```bash
bash scripts/experiment.sh <config.yml> <candidate-id> [parents] [hypothesis]
```

- `<config.yml>`: one of `assets/configs/<method>/config.yml`. The
  script renders a copy with the workspace inside the experiment dir.
- `<candidate-id>`: unique directory name. Use the UTC timestamp plus
  a slug, for example `20260920T090000Z-rerank-retrieval`.
- `parents`: comma-separated candidate ids this proposal descends from.
- `hypothesis`: one sentence. What failed before, what you change, why.

Budget: one run costs 1 to 3 minutes on the local 27B model and is
capped by `timeout 3600`. Cap candidates per iteration at 2.

## Score the tree

```bash
python3 scripts/frontier.py [tree-root]
```

Prints per-candidate aggregates (problems, tests passed/failed, pass
rate, failure stages), lineage, and the Pareto frontier over
(pass rate, tests passed, tests failed). Exit 2 when nothing scored.

## The loop

1. Baseline: run one experiment per method with no parents. Record.
2. Read: `frontier.py` output, then the raw directories. Grep
   `traces/turns.jsonl` for the failure stage, diff two candidates'
   workspaces, read the args the model sent. Do not skip this step:
   the ablation in the paper shows raw traces beat summaries.
3. Hypothesize: one causal sentence naming the harness decision to
   change. Harness surfaces: agent preambles (`ARCMiS/lib/agents/src/
   <method>/`), tool budgets in the config, method orchestration,
   validator prompts. Never touch `assets/` problem data, scoring
   scripts, or the model.
4. Propose: apply the change to the working tree, run
   `experiment.sh` with parents and the hypothesis, revert the
   working tree change (the experiment dir keeps its own copy of
   every artifact. The candidate config diff lives in
   `<candidate>/config.yml`).
5. Rescore: `frontier.py`. Keep the frontier, not a champion.
6. Repeat from 2 until the iteration budget (default 20) or the
   wall-clock budget runs out. Report the frontier and the holdout
   plan: never tune against a future test set.

## Leakage rules

- Search configs and holdout configs come from different problem
  sets. Today the kata ships one C problem. A holdout needs a second
  language pair (for example `assets/GildedRose-Refactoring-Kata/go`)
  before any generalization claim.
- The proposer may read prior experiments. It may not edit them, may
  not edit `scripts/`, and may not change the model under test.
