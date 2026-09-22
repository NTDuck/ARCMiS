# 0021. Survey sweep across methods and models

- **Date:** 2026-09-23
- **Status:** accepted

## Context

ADR 0018 fixed the three methodologies (monolith ReAct, ledger
manager-worker per arXiv:2608.26480, recode four-agent loop per
arXiv:2604.07341) and ADR 0020 fixed the per-run artifact contract.
Comparing methods required a full factorial sweep with real execution:
no fabricated numbers, every run persisted. The grid chosen:

- 4 projects from the ReCodeAgent dataset (crust split, C to Rust),
  picked for size spread and single-problem shape: `fft` (233 C LOC),
  `totp` (443), `cjson` (1101), `expr` (1110), cloned as submodules
  under `assets/`.
- 3 methodologies, 3 ollama models (`openbmb/minicpm5-2b:q8_0`,
  `qwen3.8:27b-mtp-q4_K_M`, `smtek/Swift-Qwen3.8-27B:dflash2`),
  5 repetitions per cell = 180 sequential runs.

The plan included `SparkLLM/Spark-X2.5-4B`, but we excluded it: its
`spark2_5` GGUF architecture fails to load in the local ollama
daemon.

## Decisions

- **Sweep driver as a shell script** (`scripts/survey.sh`), not Rust. The harness binary already handles one run. The
  driver only walks
  the grid, times each cell, and appends to a log. A second runner in
  Rust would duplicate the harness CLI.
- **Sanitized model directory names** (`/` and `:` to `_`). Colons in
  model tags break cargo manifests placed in paths below the run dir
  ("path segment contains separator" build failure). The evaluator
  builds every produced workspace inside that path.
- **Method as `--method` CLI flag** on the harness, with the config
  directory name as fallback: one config tree per project, method
  selected per run. The manifest records the resolved name so
  artifacts stay self-describing.
- **Model offload between every run** (`--offload`). The 27B pair is
  17-19 GB each. Leaving one resident while loading the next fills
  the 3090. Offload unloads everything and polls `/api/ps` until
  empty before and after each run.
- **Toolchain as the only scorer**. The harness records agent
  self-reports but never trusts them. Success requires the
  evaluator's rerun of `cargo build` and `cargo test` in the produced
  workspace.
- **Run artifacts under `experiments/220926/runs/`** (227 MB), kept
  out of git: per-run `manifest.json`, `config.yml`, `stdout.log`,
  `traces/turns.jsonl`, workspace, result yaml, per-problem JSON.
  We commit the aggregates (`results/aggregated.yml`,
  `results/per-run.yml`) and the sweep log.

## Outcome

180 cells, 5 h 07 m wall. 7 toolchain-verified successes, all on
`fft` (5 monolith, 2 ledger qwen3.8 on cjson/totp). Failure mix:
42% MaxTurnsError, 39% other model-protocol errors, 13% daemon
rejections of malformed tool-call arguments, 2% JSON parse. Full
tables in `docs/bench/2026-09-22-survey-220926.md`.
