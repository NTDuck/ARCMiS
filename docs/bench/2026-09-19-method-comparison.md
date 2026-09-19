# Migration method benchmark: Monolith vs Ledger vs Recode
## Revision 2 (2026-09-19, separate-agent implementations)

The v1 measurement below ran two flattened methods: one file per paper,
both shapes collapsed toward a manager-tool design. After the restructure
to paper-faithful agent teams, the same 5x3 protocol ran again: same task,
same model, same budgets, fresh workspaces. Verification against the
papers changed the code in three ways that affect the measurement: the
Recode pipeline now runs four specialized agents in a deterministic loop
with a coverage-gap pass (paper 3.5(b)), the Ledger manager delegates
through a worker tool and re-curates the task list, and phase agents
return prompted JSON instead of plain text.

| Method | Compilation rate | Full-test-pass rate | Test pass rate | Wall time per pass |
|---|---|---|---|---|
| Monolith | 5/5 | 0/5 | 0.50 += 0.00 | 97.4 += 34.0 s |
| Ledger (arXiv:2608.26480) | 5/5 | 1/5 | 0.60 += 0.22 | 116.3 += 21.6 s |
| Recode (arXiv:2604.07341) | 5/5 | 1/5 | 0.65 += 0.22 | 160.5 += 19.1 s |

Per-pass pass rates (tests passed / total):

| Method | Rep 1 | Rep 2 | Rep 3 | Rep 4 | Rep 5 |
|---|---|---|---|---|---|
| Monolith | 1/2 | 1/2 | 1/2 | 1/2 | 1/2 |
| Ledger | 1/2 | 1/2 | 1/2 | 1/2 | 2/2 pass |
| Recode | 1/2 | 2/2 pass | 1/2 | 3/4 | 1/2 |

Reading:

- The ordering flipped with the faithful implementations: the two
  multi-agent methods now sit above the monolith (0.65 and 0.60 vs
  0.50), while v1 had the monolith on top. The monolith's pass rate
  collapsed to a flat 0.50 with zero variance across all five passes.
  The v1 monolith numbers came from a run before the validator prompt
  and output-mode changes, so the v1 and v2 monolith columns are not
  one method measured twice. They are two harness generations.
- Wall time ranking is stable: more agents, more seconds. Recode pays
  for the analyzer and planner phases plus the coverage pass on top of
  the translate-validate loop.
- The deltas still sit inside one SD of each other (n=5). The v1
  conclusion stands in revised form: on this task the faithful
  scaffolds no longer hurt, but the evidence for them paying is one
  full pass in five for each method.
- Raw artifacts live in `.artifacts/bench/` (the v2 run overwrote the
  v1 run directories). The v2 aggregates live in
  `.artifacts/bench/summary-2026-09-19-restructure.json`.

---

# Migration method benchmark: Monolith vs Ledger vs Recode

- **Date:** 2026-09-19
- **Task:** GildedRose-Refactoring-Kata, C to Rust, `cargo test`
- **Model:** qwen3.8:27b-mtp-q4_K_M (local ollama, 100% on the RTX 3090, zero CPU layers)
- **Budgets:** num_ctx 16384, max_output_tokens 4096, think off, temperature 0.2, max_retries 1. Turn budgets: monolith 14, ledger 14, recode 40.
- **Protocol:** 5 independent passes per method, fresh workspace per pass, sequential runs, one warm model instance (`OLLAMA_KEEP_ALIVE=-1`).

## Result table

Score per pass = tests passed / (tests passed + tests failed), measured by
re-running `cargo test` in each workspace after the run. `x += y` is mean
pass rate += sample SD (divisor n-1) over the 5 passes.

| Method | Compilation rate | Full-test-pass rate | Test pass rate | Wall time per pass |
|---|---|---|---|---|
| Monolith | 5/5 | 2/5 | 0.75 += 0.25 | 40.8 += 7.6 s |
| Ledger (arXiv:2608.26480) | 5/5 | 1/5 | 0.60 += 0.22 | 47.8 += 15.3 s |
| Recode (arXiv:2604.07341) | 5/5 | 0/5 | 0.53 += 0.08 | 45.0 += 20.9 s |

## Per-pass detail

| Method | Rep 1 | Rep 2 | Rep 3 | Rep 4 | Rep 5 |
|---|---|---|---|---|---|
| Monolith | 3/3 pass | 1/2 | 1/2 | 3/4 | 2/2 pass |
| Ledger | 2/2 pass | 1/2 | 1/2 | 1/2 | 1/2 |
| Recode | 1/2 | 1/2 | 1/2 | 1/2 | 2/3 |

## Reading

- Every pass of every method produced a compiling package (15/15). The
  methods differ in whether the translated test suite survives.
- The task is small (6.8 KB source). The manager and coordinator loops
  spent their turn budget on delegation round trips rather than on the
  single decisive fix. The monolith applied every turn directly to the
  code.
- The monolith's exit code was FAILURE on every pass even when its own
  validator reported success. Root cause: the harness compares
  `compilation_status == "pass"`, but the schema tells the model to report
  a boolean, so the model answered `true`. The measured scores above come
  from re-running the toolchain, not from that string.
- The scaffold deltas point the same direction as the papers' own
  conclusions but inverted for scale: the papers find the largest gains on
  hard, long-horizon problems, and the scaffold "buys most where the model
  unaided is weakest". On a small kata with a 27B model, the single agent
  is already above the regime where decomposition pays, and the extra
  orchestration turns subtract from the fix budget.

## Runtime artifacts

- `.artifacts/bench/<method>/rep<N>/config.yml` — the exact config per pass.
- `.artifacts/bench/<method>/rep<N>/workspace/` — the translated codebase,
  its `.ARCMiS/result/*.yml` harness report, and build caches.
- `.artifacts/bench/<method>/rep<N>/stdout.log`, `stderr.log` — full harness
  logs including every model turn and tool call.
- `.artifacts/bench/bench.log` — one JSON record per pass with rc and wall
  time.
- `.artifacts/bench/scores.jsonl` — per-workspace rescoring output.
- `.artifacts/bench/summary.json` — the aggregated table above.
- `.artifacts/bench/run-bench.sh`, `.artifacts/bench/score.py` — the runner
  and the scorer.

## Threats to validity

- n=5 per method. The deltas (0.75 vs 0.60 vs 0.53) sit inside one SD of
  each other. The sample size supports no significance claim.
- One task, one language pair, one model. The paper effect (scaffold wins
  on hard problems, hurts on easy ones) predicts exactly this ordering on a
  kata. A LiveCodeBench-style suite would give the informative re-run.
- The harness result yaml `compilation_status` string mismatch means exit
  codes carry no signal. The scorer ignores them and re-runs the toolchain.
