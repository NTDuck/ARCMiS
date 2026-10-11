#!/usr/bin/env python3
"""Aggregate one benchmark sweep: totals, per-tool table, REPORT.md.

Walks benchmarks/{commit}/**/result.yml (one record per benchmark cell,
written by bench/run-benchmarks.sh through bench/score-run.sh) and writes:

  benchmarks/{commit}/result.yml   global totals + per-tool breakdown
  benchmarks/{commit}/REPORT.md    human report with links to every cell

Toolchain-only scoring: compilation status and test pass rates come from the
target toolchain in the produced workspace, never from agent self-reports.

Usage: python3 bench/aggregate.py [COMMIT]
       COMMIT defaults to the lexicographically latest benchmarks/ entry.
Stdlib only: parses the flat result.yml records without PyYAML.
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Top-level keys of a cell result.yml (flat scalars, one per line). Unknown
# lines (the embedded aggregate block, comments) are kept out of the record.
SCALAR_KEYS = (
    "tool",
    "project",
    "source_language",
    "target_language",
    "commit",
    "output_dir",
    "duration_seconds",
    "harness_exit",
    "score_status",
    "compilation_status",
    "tests_pass",
    "tests_fail",
    "test_pass_rate",
)


def load_cell(path: Path) -> dict | None:
    """Parse one flat result.yml; None when missing, empty, or malformed."""
    try:
        record: dict = {}
        for line in path.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            key, _, value = line.partition(":")
            if key not in SCALAR_KEYS:
                continue
            value = value.strip()
            record[key] = None if value == "null" else value
        if not record:
            return None
        record["cell_dir"] = str(path.parent.relative_to(ROOT))
        return record
    except OSError:
        return None


def as_int(value) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def main() -> int:
    args = sys.argv[1:]
    bench_root = ROOT / "benchmarks"
    commit = args[0] if args else None
    if commit is None:
        commits = sorted(p for p in bench_root.iterdir() if p.is_dir())
        if not commits:
            print("no benchmark commits found under benchmarks/")
            return 1
        commit = commits[-1].name
    commit_dir = bench_root / commit
    if not commit_dir.is_dir():
        print(f"no such benchmark dir: {commit_dir}")
        return 1

    cells = sorted(commit_dir.glob("*/*/*/result.yml"))
    records = [r for c in cells if (r := load_cell(c))]
    if not records:
        print(f"no cell result.yml under {commit_dir}")
        return 1

    by_tool: dict[str, list[dict]] = {}
    for record in records:
        by_tool.setdefault(record.get("tool") or "?", []).append(record)

    tool_rows = []
    for tool in sorted(by_tool):
        tool_cells = by_tool[tool]
        compiled = sum(1 for r in tool_cells if r.get("compilation_status") == "ok")
        tested = [
            r for r in tool_cells
            if (v := as_int(r.get("test_pass_rate"))) is not None
        ]
        green = sum(1 for r in tool_cells if r.get("score_status") == "tests_green")
        rates = [v for r in tool_cells if (v := as_int(r.get("test_pass_rate"))) is not None]
        avg_rate = sum(rates) / len(rates) if rates else 0.0
        tool_rows.append(
            {
                "tool": tool,
                "cells": len(tool_cells),
                "compiled": compiled,
                "compile_rate": round(compiled * 100 / len(tool_cells), 1),
                "tests_green": green,
                "avg_test_pass_rate": round(avg_rate, 1),
            }
        )

    total_runs = len(records)
    total_compiled = sum(1 for r in records if r.get("compilation_status") == "ok")
    all_rates = [v for r in records if (v := as_int(r.get("test_pass_rate"))) is not None]
    total_green = sum(1 for r in records if r.get("score_status") == "tests_green")

    summary = {
        "commit": commit,
        "runs": total_runs,
        "compiled": total_compiled,
        "compile_rate": round(total_compiled * 100 / total_runs, 1),
        "tests_green": total_green,
        "cells_with_tests": len(all_rates),
        "avg_test_pass_rate": round(sum(all_rates) / len(all_rates), 1) if all_rates else 0.0,
        "tools": tool_rows,
    }
    summary_path = commit_dir / "result.yml"
    lines = [f"{key}: {value}" for key, value in summary.items() if key != "tools"]
    for row in tool_rows:
        lines.append(f"- tool: {row['tool']}")
        for key, value in row.items():
            if key != "tool":
                lines.append(f"  {key}: {value}")
    summary_path.write_text("\n".join(lines) + "\n")

    report: list[str] = [
        f"# ARCMiS benchmark {commit}",
        "",
        "One MAS run per (tool, project, source->target) cell from the",
        "ReCodeAgent `tool_projects` dataset. Compilation status and test pass",
        "rate are measured by the target toolchain in the produced workspace",
        "(`bench/score-run.sh`); agent self-reports are ignored.",
        "",
        "## Totals",
        "",
        f"- Runs: {total_runs}",
        f"- Compile rate: {summary['compile_rate']}% ({total_compiled}/{total_runs})",
        f"- Cells with runnable tests: {len(all_rates)}",
        f"- Avg test pass rate: {summary['avg_test_pass_rate']}%",
        f"- Tests green: {total_green}",
        "",
        "## Per-tool breakdown",
        "",
        "| tool | cells | compiled | compile rate | tests green | avg pass rate |",
        "|---|---|---|---|---|---|",
    ]
    for row in tool_rows:
        report.append(
            f"| {row['tool']} | {row['cells']} | {row['compiled']} | "
            f"{row['compile_rate']}% | {row['tests_green']} | {row['avg_test_pass_rate']}% |"
        )
    report += ["", "## Cells", "", "| cell | status | compile | pass rate | duration |", "|---|---|---|---|---|"]
    for record in records:
        cell = record["cell_dir"]
        rate = record.get("test_pass_rate")
        rate_text = f"{rate}%" if rate is not None else "-"
        report.append(
            f"| [{cell}]({cell}/result.yml) | {record.get('score_status', '?')} | "
            f"{record.get('compilation_status', '?')} | {rate_text} | "
            f"{record.get('duration_seconds', '?')}s |"
        )
    report.append("")
    (commit_dir / "REPORT.md").write_text("\n".join(report))

    print(f"aggregated {total_runs} cells -> {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
