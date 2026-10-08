## User directive log - ARCMiS autooptimise campaign

Consolidated from the orchestration transcript; newest last. Statuses as of 2026-10-06 16:0xZ.

| # | When | Directive (condensed, faithful) | Disposition |
|---|---|---|---|
| 1 | 2026-09-23 | Conduct survey 230926: diverse ReCodeAgent samples (6-8), min/max/avg LoC + max complexity; generous ceilings (unlimited turns logged), advocate agent work; persist like prior survey | done (earlier session) |
| 2 | 2026-10-03 | Check whether autooptimise parallelism can increase (GPU headroom); implement if possible | done: SLOTS=2, STAGGER=240 pair slots |
| 3 | 2026-10-04 | Switched back to ninfer; q27 abandoned, all q27 artifacts invalidated; continue autooptimise from last ninfer checkpoint | done: engine ninfer/qwen3.8-27b :8081 |
| 4 | 2026-10-04 | Model changed: past results need revalidation; rerun each completed codebase once more | folded into v3 wave protocol |
| 5 | 2026-10-06 | Ascertain whether project failures are harness-constraint-caused (max turns etc.); fix harness or nudge proposer; address absurd wall time | forensics done: MaxTurns pathology, role_think gap, memo-cache fix; in-tree fixes uncommitted |
| 6 | 2026-10-06 | PL generality: ARCMiS supports all PLs, not just *->Rust; assess Rust bias; affect codebase, paper, slides; rerun mkslides + mkpaper | done: rescore.py language table, test_command, stub prompts, bench fail-loud; paper 23pp rebuilt; deck redone v2 |
| 7 | 2026-10-06 | System rebooted: continue orchestrating; restart everything (constraint "never restart ninfer-serve" superseded when dead) | recovered: engine up, driver relaunched START=6, watchdog up; s33-strsim reboot-casualty |
| 8 | 2026-10-06 | User quit omp mid-turn: restart everything | fresh sessions spawned |
| 9 | 2026-10-06 | Start and monitor autooptimise | watch cadence 1200s, close protocol armed |
| 10 | 2026-10-06 | temp/ artifacts: agent hierarchy graph (each agent own block), state machine (2 lines, slide-sized), attempted-projects graph (max metrics: time, compile rate, test pass rate), rounds .md report (time, LoC, language pair, metrics, notes); python libs | done (slide-figures-2): agent-hierarchy.png + fig-hierarchy.py, state-machine.png + fig-state-machine.py, attempted-projects.png + fig-projects.py, rounds-report.md + fig-rounds-report.py (57 rows) |
| 11 | 2026-10-06 | Deck redo in resdir register (v1 rejected); ~30-35 slides, one concept per slide, APA citations, footer everywhere | done + verified; commit decision pending with user |
| 12 | 2026-10-06 | Dashboard must show everything to monitor: tok/s, time spent, proposer decisions; aggregate all requests since the beginning | in progress (dashboard-build) |
| 13 | standing | Score closes on landing (rescore + rounds_ledger + SUMMARY row on done); admission-503 report-only; fatal-launcher ladder >=2/wave report+wait; PSI cpu some avg10 >30% x2 cycles -> SLOTS=2; never name sessions q27; no harness/sweep.sh edits mid-campaign; .artifacts/ never committed; .env never committed; sudo password in dashboard .env; chtrie ruling: scoreboard stays 13/114, ledger SOLVED stands, VERIFIED-WORKSPACE retry candidate | active constraints |
| 14 | pending relay | ninfer-serve unit permanence; remimu retry; fleet-analyst wave-2 SUMMARY correction offer; commit decision (deck v2 + paper + PL fixes) | awaiting user |

Living document; update on each new directive.
