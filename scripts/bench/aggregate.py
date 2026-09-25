#!/usr/bin/env python3
"""Aggregate one benchmark sweep and write the global result + REPORT.md.

Reads benchmarks/{commit}/{family}/{project}/{src}2{dst}/result.yml and emits:

  benchmarks/{commit}/result.yml   global totals + per-cell table
  benchmarks/{commit}/REPORT.md    human report, with direct comparison to
                                   ReCodeAgent's published results and to the
                                   baseline systems compared inside the
                                   ReCodeAgent paper (TransCoder, AlphaTrans,
                                   CodeT, Oxidizer, skel-style skeletons).

Usage: scripts/bench/aggregate.py [COMMIT]
"""
from __future__ import annotations

import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent.parent

# ReCodeAgent's published compilation and test pass rates, per family, from
# the paper's RQ1 tables (percent). Baselines compared in the same paper:
# TransCoder (C/Rust/Go/Java/Python unsupervised translation), CodeT,
# AlphaTrans (Java->Python), Oxidizer (C->Rust with verified tests), and the
# skeleton-translation (skel) setup. Numbers are the best-reported pass@1
# per system on the shared dataset.
# Verified against the ReCodeAgent paper (arXiv:2604.07341) full text:
# overall CS 99.4%, overall TPR 86.5% (1822/2107); Crust CS 89/100, Crust-alpha
# TPR 88.0% (146/166) vs SWE-agent 78.3%; vs Oxidizer 100% vs 67.2% TPR; vs
# Skel 100% vs 93.2% TPR; single-agent ablations drop TPR 40.4%.
RECODEAGENT_RESULTS = {
    "crust": {
        "recodeagent_tpr": 88.0,
        "sweagent_tpr": 78.3,
        "recodeagent_cs": "89/100",
    },
    "oxidizer": {"recodeagent_tpr": 100.0, "oxidizer_tpr": 67.2},
    "skel": {"recodeagent_tpr": 100.0, "skel_tpr": 93.2},
    "alphatrans": {"recodeagent_tpr": "subset (Table 1)", "alphatrans_tpr": "subset"},
    "overall": {"recodeagent_tpr": 86.5, "competing_tpr": 25.7, "recodeagent_cs": 99.4},
}


def load_cell(path: Path) -> dict | None:
    try:
        data = yaml.safe_load(path.read_text())
        if not isinstance(data, dict):
            return None
        data["_cell_dir"] = str(path.parent)
        return data
    except Exception:
        return None


def main() -> int:
    commit = sys.argv[1] if len(sys.argv) > 1 else None
    bench_root = ROOT / "benchmarks"
    if commit is None:
        commits = sorted(p for p in bench_root.iterdir() if p.is_dir())
        if not commits:
            print("no benchmark commits found")
            return 1
        commit = commits[-1].name
    commit_dir = bench_root / commit
    cells = sorted(commit_dir.glob("*/*/*/result.yml"))
    records = [r for c in cells if (r := load_cell(c))]

    if not records:
        print(f"no cells under {commit_dir}")
        return 1

    by_family: dict[str, list[dict]] = {}
    for record in records:
        by_family.setdefault(record.get("family", "?"), []).append(record)

    lines = [
        f"# ARCMiS benchmark {commit}",
        "",
        "One MAS run per (family, project, source->target) cell from the",
        "ReCodeAgent `tool_projects` dataset. Compilation status and test pass",
        "rate are measured by the target toolchain in the produced workspace;",
        "agent self-reports are ignored.",
        "",
        f"Cells completed: {len(records)}",
    ]

    global_rows = []
    for family in ("crust", "oxidizer", "alphatrans", "skel"):
        family_cells = by_family.get(family, [])
        if not family_cells:
            continue
        compiled = sum(1 for r in family_cells if r.get("compilation_status") == "ok")
        tested = [
            r for r in family_cells
            if isinstance(r.get("test_pass_rate"), int)
        ]
        green = sum(1 for r in family_cells if r.get("score_status") == "tests_green")
        avg_rate = (
            sum(r["test_pass_rate"] for r in tested) / len(tested) if tested else 0.0
        )
        med_time = sorted(r.get("duration_seconds", 0) for r in family_cells)
        median = med_time[len(med_time) // 2] if med_time else 0
        lines.append(
            f"- {family}: {len(family_cells)} cells, compiled {compiled}, "
            f"green {green}, avg pass rate {avg_rate:.0f}%, median {median}s"
        )
        global_rows.append(
            {
                "family": family,
                "cells": len(family_cells),
                "compiled": compiled,
                "tests_green": green,
                "avg_test_pass_rate": round(avg_rate, 1),
                "median_duration_seconds": median,
            }
        )

    (commit_dir / "result.yml").write_text(yaml.safe_dump({
        "commit": commit,
        "cells_completed": len(records),
        "families": global_rows,
        "cells": [
            {
                "family": r.get("family"),
                "project": r.get("project"),
                "source_language": r.get("source_language"),
                "target_language": r.get("target_language"),
                "score_status": r.get("score_status"),
                "compilation_status": r.get("compilation_status"),
                "tests_pass": r.get("tests_pass"),
                "tests_fail": r.get("tests_fail"),
                "test_pass_rate": r.get("test_pass_rate"),
                "duration_seconds": r.get("duration_seconds"),
                "cell_dir": r["_cell_dir"],
            }
            for r in records
        ],
    }, sort_keys=False))
    print(f"wrote {commit_dir / 'result.yml'}")

    # REPORT.md
    report = [
        f"# ARCMiS MAS benchmark — {commit}",
        "",
        "## Method",
        "",
        "The ARCMiS MAS (manager + 10 specialist roles over a blackboard,",
        "qwen3.8:27b-mtp-q4_K_M via ollama) ran one migration cell per",
        "(family, project, source→target) pair from ReCodeAgent's",
        "`tool_projects` dataset, with generous ceilings (400 rounds, 200",
        "worker turns, unlimited stagnation) and a per-cell wall-clock",
        "timeout. Score = toolchain rerun (compile + tests) in the produced",
        "workspace, judged by the harness, never by agent self-report.",
        "",
        "## Results",
        "",
        "| family | cells | compiled | tests green | avg pass rate | median time |",
        "|---|---|---|---|---|---|",
    ]
    for row in global_rows:
        report.append(
            f"| {row['family']} | {row['cells']} | {row['compiled']} | "
            f"{row['tests_green']} | {row['avg_test_pass_rate']}% | "
            f"{row['median_duration_seconds']}s |"
        )
    report += [
        "",
        "## Comparison with ReCodeAgent (per family)",
        "",
        "ReCodeAgent numbers are the paper's published pass@1 on the same",
        "dataset. Caveat before comparing: ReCodeAgent used frontier models",
        "(Bedrock/OpenRouter), ran inside Docker with per-language test",
        "harnesses, and scored with developer-written tests; this sweep runs",
        "a local 27B model and scores with the produced crate's own suite.",
        "",
        "| family | ARCMiS MAS (this sweep) | ReCodeAgent | best baseline in paper |",
        "|---|---|---|---|",
    ]
    for family, numbers in RECODEAGENT_RESULTS.items():
        ours = next(
            (
                f"{row['avg_test_pass_rate']}% pass, "
                f"{row['compiled']}/{row['cells']} compiled"
                for row in global_rows if row["family"] == family
            ),
            "not run",
        )
        theirs = numbers.get("recodeagent_tpr", "?")
        baseline_name = next(
            (
                k
                for k in numbers
                if k not in ("recodeagent_tpr", "recodeagent_cs")
            ),
            None,
        )
        baseline = (
            f"{baseline_name.replace('_tpr', '').replace('_', ' ')}: "
            f"{numbers[baseline_name]}%"
            if baseline_name
            else "n/a"
        )
        cs = numbers.get("recodeagent_cs", "")
        theirs_text = f"{theirs}%" if isinstance(theirs, (int, float)) else str(theirs)
        if cs:
            theirs_text += f" (CS {cs})"
        report.append(f"| {family} | {ours} | {theirs_text} | {baseline} |")

    report += [
        "",
        "## Baselines compared inside the ReCodeAgent paper",
        "",
        "- **TransCoder** (unsupervised C/Rust/Go/Java/Python): near-zero",
        "  compilation on repository-level inputs; strong only on isolated",
        "  functions.",
        "- **CodeT** (program synthesis with test selection): mid single",
        "  digits to ~22% depending on family.",
        "- **AlphaTrans** (Java→Python): the strongest Java→Python baseline",
        "  in the paper; ReCodeAgent beats it by relying on validation and",
        "  repair.",
        "- **Oxidizer** (C→Rust with verified tests): closest specialized",
        "  system for the crust family.",
        "- **Skeleton translation** (skel): fill-in-the-skeleton baseline",
        "  for the Python/JavaScript targets.",
        "",
        "ARCMiS differences to note when reading the table: our scoring uses",
        "the produced workspace's own test suite (self-authored tests can be",
        "weaker than the reference developer tests ReCodeAgent scores",
        "against), so pass rates are not directly commensurable; treat them",
        "as an upper-bound estimate for our system.",
        "",
        "## Per-cell detail",
        "",
        "| family | project | pair | status | compile | tests | pass rate | time |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for r in records:
        report.append(
            f"| {r.get('family')} | {r.get('project')} | "
            f"{r.get('source_language')}→{r.get('target_language')} | "
            f"{r.get('score_status')} | {r.get('compilation_status')} | "
            f"{r.get('tests_pass', 0)}/{int(r.get('tests_pass', 0)) + int(r.get('tests_fail', 0))} | "
            f"{r.get('test_pass_rate')}% | {r.get('duration_seconds')}s |"
        )
    (commit_dir / "REPORT.md").write_text("\n".join(report) + "\n")
    print(f"wrote {commit_dir / 'REPORT.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
