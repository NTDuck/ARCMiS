---
name: autooptimise
description: Drive Meta-Harness optimization over ARCMiS migration harnesses. One experiment per directory under .artifacts/experiments/, proposer reads raw history, harness mutable, problem set and evaluator fixed
---

# autooptimise — Meta-Harness outer loop for ARCMiS

Optimize the ARCMiS migration harness the way Meta-Harness
(arXiv:2603.28052) prescribes: an agentic proposer reads the source
code, scores, and raw execution traces of every prior candidate from
the filesystem, then proposes one executable harness change per
iteration. Existing text optimizers compress feedback too aggressively;
here the proposer reads the raw tree through its own tools, with no
summary layer in between. The model under test is fixed. The problem
set is fixed. You, the proposer, own every diagnosis and hypothesis.

## Contract (ADR 0020)

One candidate = one immutable directory `.artifacts/experiments/<id>/`:
`manifest.json` (method, model, budgets, git revision, parents,
hypothesis), `config.yml`, `traces/turns.jsonl` (one JSON line per
model call, tool call, tool result), `workspace/` (the translated
codebase), `stdout.log`, `stderr.log`, and `result/` after scoring.

- Read anything in the tree; never write into an existing experiment
  directory.
- Success is decided by a toolchain rerun in the workspace (`cargo
  build` plus the config test command). The harness exit code reports
  only the agents' build verdict.

## One experiment

```bash
bash scripts/experiment.sh <config.yml> <candidate-id> [parents] [hypothesis]
```

- `<config.yml>`: one of `assets/configs/<problem-set>/config.yml`,
  for example `assets/configs/GildedRose-Refactoring-Kata/config.yml`.
  The script renders a copy with the workspace inside the experiment
  dir.
- `<candidate-id>`: unique name; UTC timestamp plus slug, e.g.
  `20260920T090000Z-rerank-retrieval`.
- `[parents]`: comma-separated candidate ids this proposal descends
  from; empty for a seed.
- `[hypothesis]`: one sentence — what failed before, what you change,
  why it should help.

One run costs 1–3 minutes on the local 27B model, capped by
`timeout 3600`. Cap candidates per iteration at 2.

## Score the tree

```bash
python3 scripts/frontier.py [tree-root]
```

Per-candidate aggregates (problems, tests passed/failed, pass rate,
failure stages), lineage, and the Pareto frontier over (pass rate,
tests passed, tests failed). Exit 2 when nothing scored.

## The loop

1. **Baseline**: run one experiment per problem set, no parents.
2. **Read**: `frontier.py`, then the raw directories — grep
   `traces/turns.jsonl` for the failure stage, diff two candidates'
   workspaces, read the args the model sent. The paper's ablation
   shows raw traces, not summaries, carry the diagnostic signal.
3. **Hypothesize**: one causal sentence naming the harness decision
   to change. Mutable harness surfaces: agent preambles
   (`ARCMiS/lib/agents/src/<method>/`), tool budgets in the config,
   method orchestration, validator prompts.
4. **Propose**: apply the change to the working tree, run
   `experiment.sh` with parents and the hypothesis, then revert the
   working tree — the experiment dir keeps its own copy of every
   artifact, including the rendered candidate config.
5. **Rescore**: `frontier.py`. Keep the frontier, not a champion.
6. **Stop**: no improvement in the frontier's primary metric (pass
   rate, then tests passed) over 3 consecutive propose→evaluate
   cycles, or after the 20-iteration budget — whichever first.
7. **Report** the frontier and the holdout plan: never tune against a
   future test set.

## Leakage rules

- Search configs and holdout configs must come from different problem
  sets. Today the repo ships one problem set
  (`assets/GildedRose-Refactoring-Kata`), so a holdout needs a second
  one before any generalization claim.
- The proposer may read prior experiments. It may not edit them, may
  not edit `scripts/`, and may not change the model under test.
