# ARCMiS MAS benchmark — cafe8be44df5

## Method

The ARCMiS MAS (manager + 10 specialist roles over a blackboard,
qwen3.8:27b-mtp-q4_K_M via ollama) ran one migration cell per
(family, project, source→target) pair from ReCodeAgent's
`tool_projects` dataset, with generous ceilings (400 rounds, 200
worker turns, unlimited stagnation) and a per-cell wall-clock
timeout. Score = toolchain rerun (compile + tests) in the produced
workspace, judged by the harness, never by agent self-report.

## Results

| family | cells | compiled | tests green | avg pass rate | median time |
|---|---|---|---|---|---|
| crust | 2 | 1 | 1 | 100.0% | 2100s |

## Comparison with ReCodeAgent (per family)

ReCodeAgent numbers are the paper's published pass@1 on the same
dataset. Caveat before comparing: ReCodeAgent used frontier models
(Bedrock/OpenRouter), ran inside Docker with per-language test
harnesses, and scored with developer-written tests; this sweep runs
a local 27B model and scores with the produced crate's own suite.

| family | ARCMiS MAS (this sweep) | ReCodeAgent | best baseline in paper |
|---|---|---|---|
| crust | 100.0% pass, 1/2 compiled | 78.2% | transcoder: 3.2% |
| oxidizer | not run | 66.7% | oxidizer_baseline: 40.0% |
| alphatrans | not run | 75.0% | alphatrans_baseline: 52.0% |
| skel | not run | 81.3% | skeleton_baseline: 63.5% |

## Baselines compared inside the ReCodeAgent paper

- **TransCoder** (unsupervised C/Rust/Go/Java/Python): near-zero
  compilation on repository-level inputs; strong only on isolated
  functions.
- **CodeT** (program synthesis with test selection): mid single
  digits to ~22% depending on family.
- **AlphaTrans** (Java→Python): the strongest Java→Python baseline
  in the paper; ReCodeAgent beats it by relying on validation and
  repair.
- **Oxidizer** (C→Rust with verified tests): closest specialized
  system for the crust family.
- **Skeleton translation** (skel): fill-in-the-skeleton baseline
  for the Python/JavaScript targets.

ARCMiS differences to note when reading the table: our scoring uses
the produced workspace's own test suite (self-authored tests can be
weaker than the reference developer tests ReCodeAgent scores
against), so pass rates are not directly commensurable; treat them
as an upper-bound estimate for our system.

## Per-cell detail

| family | project | pair | status | compile | tests | pass rate | time |
|---|---|---|---|---|---|---|---|
| crust | leftpad | c→rust | timeout | error | 0/0 | None% | 2100s |
| crust | nandc | c→rust | tests_green | ok | 2/2 | 100% | 1657s |
