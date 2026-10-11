# ARCMiS MAS benchmark — 64d34fdb03f9

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
| crust | 4 | 4 | 4 | 100.0% | 5400s |

## Comparison with ReCodeAgent (per family)

ReCodeAgent numbers are the paper's published pass@1 on the same
dataset. Caveat before comparing: ReCodeAgent used frontier models
(Bedrock/OpenRouter), ran inside Docker with per-language test
harnesses, and scored with developer-written tests; this sweep runs
a local 27B model and scores with the produced crate's own suite.

| family | ARCMiS MAS (this sweep) | ReCodeAgent | best baseline in paper |
|---|---|---|---|
| crust | 100.0% pass, 4/4 compiled | 88.0% (CS 89/100) | sweagent: 78.3% |
| oxidizer | not run | 100.0% | oxidizer: 67.2% |
| skel | not run | 100.0% | skel: 93.2% |
| alphatrans | not run | subset (Table 1) | alphatrans: subset% |
| overall | not run | 86.5% (CS 99.4) | competing: 25.7% |

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
| crust | 2dpartint | c→rust | tests_green | ok | 25/25 | 100% | 5400s |
| crust | 42-kocaeli-printf | c→rust | tests_green | ok | 24/24 | 100% | 1598s |
| crust | aes128-simd | c→rust | tests_green | ok | 66/66 | 100% | 3675s |
| crust | amp | c→rust | tests_green | ok | 5/5 | 100% | 5400s |
