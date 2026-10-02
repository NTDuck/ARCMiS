---
name: mkreport
description: >
  Build the ARCMiS project report in docs/report/ (LaTeX). Use when the
  user says "/mkreport", "write the report", "update the project report",
  or asks for the campaign report. Reads live campaign numbers from
  .artifacts/experiments/ at build time. Style follows the resdir.pptx
  tokens mapped to LaTeX.
---

# /mkreport — ARCMiS project report

Build the project report. Source lives in `docs/report/paper.tex`. Build
output is `docs/report/paper.pdf`. Read the campaign numbers at build
time. Never copy stale numbers into this skill.

## Source of truth

- `docs/report/paper.tex` — the existing report source. Edit it. Do not
  start a parallel format.
- `docs/report/references.bib` — bibliography.
- `.artifacts/experiments/ROUNDS.yaml` — per-round ledger. Verdicts:
  SOLVED, CLEARED, NOT CLEARED, ABORTED, IN FLIGHT.
- `.artifacts/experiments/SUMMARY.md` — campaign arc, scoreboard,
  champion, in-flight rounds.
- `assets/ReCodeAgent/data/tool_projects` — the dataset
  (crust/oxidizer/skel/alphatrans families).

## Report structure

Sections in this order:

1. **Title** — project name ARCMiS, one-line pitch, author block.
2. **Abstract / executive summary** — what ARCMiS does, the campaign
   state in two sentences, the headline numbers from today's ledger.
3. **Introduction / problem** — legacy C/Java/Go-to-Rust migration by
   hand. Why automation with LLM agents helps. What ARCMiS adds:
   hierarchical multi-agent orchestration with evidence-gated phase
   exits and toolchain-only scoring.
4. **Methodology** — the MAS hierarchy (orchestrator tier-1, leads
   tier-2, specialists tier-3, ADR 0022 and ADR 0026), the coverage
   sweep pipeline (`scripts/sweep.sh`), the autooptimise Meta-Harness
   loop (arXiv:2603.28052, `.omp/skills/autooptimise/SKILL.md`), and
   the toolchain-only scoring protocol (`scripts/rescore.py`).
5. **Inspirations** — the same three as /mkslides, each with relevance
   plus the codebase mapping:
   a. **Orchestration-pattern survey** — 2-axis taxonomy:
      centralized/decentralized/hierarchy x static/dynamic-adaptive.
      ARCMiS sits in hierarchy x dynamic-adaptive. Mapping: tier-2
      teams in `ARCMiS/lib/orchestrator/src/hierarchy.rs`, lead inner
      loop in `lead.rs`. Verify both files exist before you assert the
      mapping.
   b. **Modernization workflows** — ReCodeAgent (arXiv:2604.07341,
      ASE 2026, vendored at `assets/ReCodeAgent`): dataset source
      (tool_projects) and four-stage pipeline discipline. Claude
      Modernization Plugin: discovery-to-brief-to-execution flow with
      evidence-gated phase exits (ADR 0022).
   c. **System-one models: jev + laya** — present in this order:
      llm-as-a-judge, then jev-as-a-judge (`jev_judge.rs`, ADR 0023,
      `jev_triage.rs` ADR 0027), then typed decisions in general
      (choice/score/noul in one forward pass, arXiv:2609.26550,
      `.omp/skills/laya`). State the honest wiring: the harness wires
      per-dispatch triage with a min-confidence cascade, and the round
      policy defaults to `observe` (verdicts recorded, no round stops).
      Verify against `round_triage.rs` and `round_triage/policy.rs`
      before you write.
6. **Benchmarks** — live numbers at build time:
   - Dataset: count `assets/ReCodeAgent/data/tool_projects` subdirs
     (alphatrans + crust + oxidizer + skel). The campaign brief says
     114 problems. State both the counted total and the brief number.
   - Progress: count unique cleared problems from ROUNDS.yaml (a set
     counts once across retries. Verdict SOLVED or CLEARED).
   - Estimated time to complete: rate = cleared / campaign days.
     Remaining = (total - cleared) / rate. Label it an estimate from
     the current rate.
   - Comparison vs ReCodeAgent published record (99.4% compilation
     success, 86.5% test pass rate, Claude 4.5 Sonnet, 118 projects).
     LABEL IT: different protocol. Directional context only.
7. **References** — `references.bib` entries, ADR list, dataset.

## Style mapping (from .omp/skills/mkslides/references/resdir.pptx)

The slides skill holds the extracted tokens. Map to LaTeX:

| resdir token | LaTeX |
|---|---|
| Calibri body | `\usepackage[defaultfam]{tabularray}` no — use `\setmainfont{Calibri}` under fontspec when the font exists, else `\usepackage{helvet}` scaled 95 for the sans headings and keep a serif body |
| Titles 44pt, navy #1F497D | `\usepackage{titlesec}` + `\definecolor{arcnavy}{HTML}{1F497D}` + colored section titles |
| Accent #4F81BD | `\definecolor{arcblue}{HTML}{4F81BD}` for rules, table headers, links |
| Negative #C0504D / positive #9BBB59 | verdict table cell colors: NOT CLEARED / ABORTED in red, CLEARED / SOLVED in green |
| Footer 12pt | `\usepackage{fancyhdr}`: left "ARCMiS", right page number |
| 4:3 | keep the default article page. The deck carries the 4:3 constraint, not the report |

## Build

```bash
cd docs/report
pdflatex paper.tex && bibtex paper && pdflatex paper.tex && pdflatex paper.tex
```

Or `latexmk -pdf paper.tex`. Four passes when the bib changes.

## Verify

1. `pdflatex` exits 0, no unresolved `\ref` or citation warnings
   (`grep "undefined" paper.log`).
2. `paper.pdf` exists with the expected section order.
3. Open the PDF and verify: navy section titles, footer, verdict table
   colors, live numbers match today's ROUNDS.yaml.

## STE rules

All report prose follows the installed asd-ste100 skill. Short
sentences, active voice, one meaning per word.
