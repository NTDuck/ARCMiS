#!/usr/bin/env python3
"""Aggregate the 220926 survey sweep.

Walks experiments/220926/runs/<project>/<method>/<model>/rep<N>/, reads
the harness artifacts of every attempt (successful or not), and writes:

- experiments/220926/results/per-run.yml: one record per attempt
- experiments/220926/results/aggregated.yml: per (project, method,
  model) mean compile rate, mean pass rate, mean tests, wall time

Success is always the toolchain's verdict from result/per_problem.json,
never the agent's self-report. A run that failed before scoring (rc
non-zero, no result dir) records the failure and zero metrics.

Output is YAML rendered by hand: the record schema is a flat dict of
scalars and one list, no anchors, so no yaml library is needed.
"""
import json
import math
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
RUNS = REPO / "experiments" / "220926" / "runs"
OUT = REPO / "experiments" / "220926" / "results"


def parse_result_yml(path: Path) -> dict:
    """Read the flat scalar keys of one run result yaml."""
    out = {}
    for line in path.read_text().splitlines():
        m = re.match(r"^(\w+): (.*)$", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def parse_elapsed(text: str) -> float:
    """Turn `123.456789s` or `1m 2s` style debug output into seconds."""
    m = re.match(r"([\d.]+)s$", text or "")
    if m:
        return float(m.group(1))
    total = 0.0
    for value, unit in re.findall(r"([\d.]+)([smh])", text or ""):
        total += float(value) * {"s": 1, "m": 60, "h": 3600}[unit]
    return total


def wall_from_log(run_dir: Path) -> float:
    """Read the wall seconds the sweep driver logged for this cell."""
    key = f"\"{run_dir.parent.name}\",\"rep\":{run_dir.name.replace('rep', '')},"
    log = REPO / "experiments" / "220926" / "survey.log"
    if not log.is_file():
        return math.nan
    for line in log.read_text().splitlines():
        if run_dir.parent.parent.name in line and key in line:
            m = re.search(r'"wall_s":(\d+)', line)
            if m:
                return int(m.group(1))
    return math.nan


def collect_run(run_dir: Path) -> dict:
    project, method, model = run_dir.parent.parent.parent.name, run_dir.parent.parent.name, run_dir.parent.name
    rec = {
        "project": project,
        "method": method,
        "model": model,
        "rep": run_dir.name,
    }
    manifest = run_dir / "manifest.json"
    if manifest.is_file():
        data = json.loads(manifest.read_text())
        rec["model"] = data.get("model", rec["model"])
        rec["method"] = data.get("method", rec["method"])
        rec["max_turns"] = data.get("budgets", {}).get("max_turns")
    problems = run_dir / "result" / "per_problem.json"
    scored = False
    if problems.is_file():
        try:
            rows = json.loads(problems.read_text())
            row = rows[0] if rows else {}
            rec["tests_passed"] = row.get("tests_passed", 0)
            rec["tests_failed"] = row.get("tests_failed", 0)
            rec["test_success"] = bool(row.get("success"))
            scored = True
        except json.JSONDecodeError:
            pass
    if not scored:
        rec["tests_passed"] = 0
        rec["tests_failed"] = 0
        rec["test_success"] = False
    rec["scored"] = scored
    # Compile truth: the toolchain rerun scored the workspace; a run
    # without a score never produced a valid workspace.
    ymls = sorted((run_dir / "result").glob("*.yml")) if (run_dir / "result").is_dir() else []
    if scored:
        workspace = run_dir / "workspace"
        rec["compiled"] = (workspace / "Cargo.toml").is_file()
    else:
        rec["compiled"] = False
    if ymls:
        y = parse_result_yml(ymls[-1])
        rec["reported_pass_rate"] = y.get("test_pass_rate", "n/a")
        rec["reported_compiled"] = y.get("compiled", "false")
        rec["elapsed_s"] = round(parse_elapsed(y.get("elapsed", "")), 1)
    else:
        rec["reported_pass_rate"] = "n/a"
        rec["reported_compiled"] = "false"
        rec["elapsed_s"] = math.nan
    rec["wall_s"] = wall_from_log(run_dir)
    # Failure class from the harness log tail, for the failure table.
    stderr_log = run_dir / "stderr.log"
    stdout_log = run_dir / "stdout.log"
    text = ""
    for log in (stdout_log, stderr_log):
        if log.is_file():
            text += log.read_text()
    if scored and rec["test_success"]:
        rec["failure_class"] = ""
    elif scored:
        rec["failure_class"] = "tests_failed"
    elif "MaxTurnsError" in text:
        rec["failure_class"] = "max_turns"
    elif "500 Internal Server Error" in text or "peg-native" in text:
        rec["failure_class"] = "model_output"
    elif "unknown model architecture" in text:
        rec["failure_class"] = "model_load"
    else:
        rec["failure_class"] = "other"
    return rec


def mean(values):
    vals = [v for v in values if not (isinstance(v, float) and math.isnan(v))]
    return sum(vals) / len(vals) if vals else math.nan


def main():
    runs_root = RUNS
    records = []
    for run_dir in sorted(runs_root.glob("*/*/*/rep*")):
        if not run_dir.is_dir():
            continue
        records.append(collect_run(run_dir))

    results = REPO / "experiments" / "220926" / "results"
    results.mkdir(parents=True, exist_ok=True)

    per_run = results / "per-run.yml"
    with per_run.open("w") as fh:
        for rec in records:
            fh.write(yaml_record(rec))

    groups = {}
    for rec in records:
        key = (rec["project"], rec["method"], rec["model"])
        cell = groups.setdefault(key, [])
        cell.append(rec)

    aggregate = {
        "survey": "220926",
        "runs_total": len(records),
        "cells": [],
    }
    for (project, method, model), recs in sorted(groups.items()):
        n = len(recs)
        compiled = sum(1 for r in recs if r["compiled"])
        scored = [r for r in recs if r["scored"]]
        pass_rates = [r["tests_passed"] / max(1, r["tests_passed"] + r["tests_failed"]) for r in scored]
        aggregate["cells"].append(
            {
                "project": project,
                "method": method,
                "model": model,
                "reps": n,
                "compiled": compiled,
                "compile_rate": round(compiled / n, 3) if n else math.nan,
                "scored": len(scored),
                "test_pass_rate_mean": round(mean(pass_rates), 3) if pass_rates else math.nan,
                "tests_passed_mean": round(mean([r["tests_passed"] for r in recs]), 2),
                "tests_failed_mean": round(mean([r["tests_failed"] for r in recs]), 2),
                "wall_s_mean": round(mean([r["wall_s"] for r in recs]), 1),
                "failure_classes": {
                    cls: sum(1 for r in recs if r["failure_class"] == cls)
                    for cls in sorted({r["failure_class"] for r in recs if r["failure_class"]})
                },
            }
        )

    write_aggregate(results / "aggregated.yml", aggregate)
    print(f"runs: {len(records)}; cells: {len(aggregate['cells'])}")
    print(f"wrote {per_run}")


def yaml_record(rec: dict) -> str:
    lines = ["-"]
    for key, value in rec.items():
        if isinstance(value, str) and (value == "" or " " in value or ":" in value):
            lines.append(f"  {key}: \"{value}\"")
        elif isinstance(value, float):
            lines.append(f"  {key}: {value:.6g}")
        elif isinstance(value, bool):
            lines.append(f"  {key}: {'true' if value else 'false'}")
        else:
            lines.append(f"  {key}: {value}")
    return "\n".join(lines) + "\n"


def write_aggregate(path: Path, aggregate: dict) -> None:
    lines = [f"# Survey 220926 aggregate over {aggregate['runs_total']} attempts", "cells:"]
    for cell in aggregate["cells"]:
        lines.append("-")
        for key, value in cell.items():
            if key == "failure_classes":
                lines.append("  failure_classes:")
                for cls, count in value.items():
                    lines.append(f"    {cls}: {count}")
            elif isinstance(value, str):
                lines.append(f"  {key}: \"{value}\"")
            elif isinstance(value, float):
                lines.append(f"  {key}: {value:.6g}")
            else:
                lines.append(f"  {key}: {value}")
    path.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
