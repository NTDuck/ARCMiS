---
name: mkslides
description: >
  Build a Slidev deck for the ARCMiS code-migration harness and export it to
  docs/slides/**. Use when the user says "/mkslides", "make slides", "build
  the project deck", or asks for a presentation of the ARCMiS campaign. The
  deck reads live campaign numbers at build time. The deck follows the
  resdir.pptx style contract.
---

# /mkslides — ARCMiS slide deck

Build a Slidev deck for the ARCMiS code-migration harness. Export the
source and the built output to `docs/slides/`. Read the campaign numbers at
build time. Never copy stale numbers into this skill.

## Style contract (from references/resdir.pptx)

`references/resdir.pptx` is the byte-as-is style reference. It is an
Office theme, "On-screen Show (4:3)". Extracted tokens follow.

| Token | Value |
|---|---|
| Aspect | 4:3 — presentation.xml `sldSz cx=18288000 cy=10287000` (20in x 11.25in nominal. 4:3 ratio) |
| Base font | Calibri (major + minor latin, theme1.xml) |
| Body font | Calibri. Body size 32pt (master bodyStyle). Body font Arial fallback |
| Title size | 44pt (master titleStyle) |
| Title placement | top-left: x 0.5in, y 0.3in, width 9.0in, height 1.25in |
| Body placement | x 0.5in, y 1.75in, width 9.0in, height 4.95in |
| Footer | 12pt. Date left, footer center, slide number right. All at y 6.95in |
| Accent colors | accent1 #4F81BD (blue), accent2 #C0504D (red), accent3 #9BBB59 (green), accent4 #8064A2 (purple), accent5 #4BACC6 (cyan). accent6 #F79646 (orange) |
| Text colors | dk1 #000000, lt1 #FFFFFF, dk2 #1F497D (dark navy for headings) |

Encode in Slidev:

- Deck headmatter: `theme: default`. Set `aspectRatio: 4/3` via
  `canvasWidth` settings. Use fonts Calibri.
- Slidev headmatter font block:

  ```yaml
  fonts:
    sans: Calibri
    serif: Calibri
  ```

- Titles: left-aligned at the top, dark navy #1F497D, sized like 44pt
  (use `text-5xl` or the theme default h1).
- Accent color for emphasis: #4F81BD. Use #C0504D only for negative
  findings (NOT CLEARED verdicts), #9BBB59 for cleared rounds.
- Footer on every content slide: left = "ARCMiS", center = section name,
  right = slide number, 12pt, gray.

## Required slide structure

Sections in this order. `{section}` = one divider slide plus its topic
slides.

1. **Title slide** — project name ARCMiS, one-line pitch, author block
   bottom-left per the resdir master (date + footer row).
2. **Contents** — the section list.
3. **{section} Introduction & problem description** — legacy
   C/Java/Go-to-Rust migration by hand. Explain why automation (LLM
   agents) helps.
   what ARCMiS is: a code-migration harness with hierarchical multi-agent
   orchestration.
4. **{section} Methodology** — the deck MUST include one real mermaid diagram:
   the ARCMiS MAS hierarchy (orchestrator tier-1, leads tier-2,
   specialists tier-3). Add the coverage sweep pipeline
   (`scripts/sweep.sh`: two staggered GPU slots, proposer and scorer
   CPU pipeline, one candidate per set. Verdict from
   `scripts/rescore.py` toolchain-only scoring).
5. **{section} Inspirations** — all three below are MANDATORY, each with
   relevance plus the codebase implementation:

   a. **Orchestration-pattern survey (the "Q2 paper")** — the 2-axis
      taxonomy: centralized/decentralized/hierarchy x
      static/dynamic-adaptive. ARCMiS sits in **hierarchy x
      dynamic-adaptive**. Verify the mapping before you write it:
      `ARCMiS/lib/orchestrator/src/hierarchy.rs` (tier-2 team resolution,
      config-owned under `mas.hierarchy`) and
      `ARCMiS/lib/orchestrator/src/lead.rs` (lead inner loop). ADR 0022
      and ADR 0026 record the design. Do not name the tier-2 layer
      "hierarchical" without those files present.

   b. **Modernization workflows** — ReCodeAgent (arXiv:2604.07341, ASE
      2026, vendored at `assets/ReCodeAgent`): a multi-agent workflow for
      language-agnostic repository translation. ARCMiS borrows its
      tool_projects dataset (crust/oxidizer/skel/alphatrans families) and
      the four-stage pipeline discipline (ADR 0018, ADR 0022). The Claude
      Modernization Plugin contributes the discovery-to-brief-to-execution
      flow with evidence-gated phase exits (ADR 0022, "modernization
      plugin" paragraph).

   c. **System-one models: jev + laya** — present in this exact order:
      1. llm-as-a-judge: a full LLM decode classifies an outcome. Cost:
         one model turn per judgment.
      2. jev-as-a-judge: the same judgment from a local specialist
         checkpoint. In ARCMiS: `jev_judge.rs` (guard ask arbitration,
         ADR 0023) and `jev_triage.rs` (per-dispatch triage, ADR 0027).
      3. typed decisions in general: laya answers `choice`, `score`,
         and `noul` questions in one forward pass, no token decoding
         (arXiv:2609.26550. See `.omp/skills/laya`).

      **Bottleneck claim — verify, do not overclaim.** Read
      `ARCMiS/lib/orchestrator/src/jev_triage.rs` and
      `ARCMiS/lib/orchestrator/src/round_triage.rs` +
      `round_triage/policy.rs`. State of the last verification: the
      harness wires the per-dispatch triage (`TriageVerdict` from five
      `agent_trace_observability` questions, min-confidence cascade). The
      round policy defaults to `observe` — the harness records verdicts
      but never stops a round on them. Say exactly that on the slide:
      "wired as an observer. Enforcement (`policy: enforce`) exists but
      is not the campaign default."

   More inspirations are welcome (for example Meta-Harness,
   arXiv:2603.28052, cited in `.omp/skills/autooptimise/SKILL.md`), but
   the three above must be complete.

6. **{section} Benchmarks** — read the live numbers at build time:
   - `.artifacts/experiments/ROUNDS.yaml` (per-round ledger, verdicts
     SOLVED / CLEARED / NOT CLEARED / ABORTED / IN FLIGHT)
   - `.artifacts/experiments/SUMMARY.md` (campaign arc, scoreboard,
     champion, in-flight rounds)
   - Dataset: `assets/ReCodeAgent/data/tool_projects` — 114 problems
     (count them: alphatrans 4 + crust 100 + oxidizer 7 + skel 8 =
     119 directories. The campaign brief says 114. State both the
     counted total and the brief number. Note the delta).
   - Progress: count cleared problems (unique sets with a SOLVED or
     CLEARED verdict counting each problem once — retries of one set
     count once).
   - Estimated time to complete: rate = cleared problems / campaign
     days so far. Remaining days = (total - cleared) / rate. Label it
     an estimate from the current rate.
   - Comparison vs ReCodeAgent published numbers: 99.4% compilation
     success, 86.5% test pass rate (Claude 4.5 Sonnet, 118 projects,
     protocol on their developer-test suites (their term). LABEL IT: different protocol.
     ARCMiS runs a local 27B model, toolchain-only scoring, one set at
     a time. The numbers are directional context, not a like-for-like
     comparison.
7. **{section} References** — papers, ADRs, dataset, model links.
8. **{section} Thank you** — closing slide.

## Build

```bash
cd docs/slides
npx slidev slides.md build --base /slides/ --out dist   # static SPA
# PDF (needs playwright-chromium):
npx slidev slides.md export slides.pdf
```

Pin the deck source at `docs/slides/slides.md`. Built output goes to
`docs/slides/dist/`. Keep `package.json` in `docs/slides/` minimal
(slidev + playwright-chromium) or run with `npx` and no package.json.

## Verify

1. Build exits 0 and `docs/slides/dist/index.html` exists.
2. Serve and verify it: `python3 -m http.server` in `docs/slides/dist/`,
   then open a browser tab, screenshot the title slide, the methodology
   diagram slide, and the benchmark slide. Verify: 4:3 feel, Calibri,
   navy titles, footer present, mermaid diagram rendered.
3. If export ran, verify the `slides.pdf` page count matches the slide count.

## STE rules

All slide prose follows the installed asd-ste100 skill. Short sentences,
active voice, one meaning per word, no marketing adjectives.
