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
scalars and one list, with no anchors. No yaml library needed.
"""
import html
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
    """Read the wall seconds the sweep driver logged for this cell. The
    driver keys its lines by project, sanitized model tag, and rep."""
    log = REPO / "experiments" / "220926" / "survey.log"
    if not log.is_file():
        return math.nan
    model_dir = run_dir.parent.name
    rep_num = run_dir.name.replace("rep", "")
    project = run_dir.parent.parent.parent.name
    # The log line carries the raw model tag while the dir carries the
    # sanitized one. match either form.
    raw_model = ""
    manifest = run_dir / "manifest.json"
    if manifest.is_file():
        try:
            raw_model = json.loads(manifest.read_text()).get("model", "")
        except json.JSONDecodeError:
            pass
    # The log line carries a model tag in three forms across sweeps: the
    # sanitized dir name, the raw model tag, and the first sweep's
    # slash-less form (slash sanitized, colon kept). Match by comparing
    # all non-alphanumeric characters flattened.
    flat_dir = re.sub(r"[^a-zA-Z0-9]", "", model_dir)
    flat_raw = re.sub(r"[^a-zA-Z0-9]", "", raw_model)
    for line in log.read_text().splitlines():
        if f'"{project}"' not in line or f'"rep":{rep_num},' not in line:
            continue
        m = re.search(r'"model":"([^"]+)"', line)
        logged = m.group(1) if m else ""
        flat_logged = re.sub(r"[^a-zA-Z0-9]", "", logged)
        if flat_logged in (flat_dir, flat_raw) or model_dir in logged or (raw_model and raw_model in logged):
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
    # Compile truth: the toolchain rerun scored the workspace. a run
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
        # The evaluator reran build+test only when result steps exist.
        # Ledger records carry `steps: []` from the manager self-report.
        # those count as unverifiable regardless of what the record says.
        rec["evaluated"] = any(
            line.strip().startswith("- step:") for line in ymls[-1].read_text().splitlines()
        )
    else:
        rec["reported_pass_rate"] = "n/a"
        rec["reported_compiled"] = "false"
        rec["elapsed_s"] = math.nan
        rec["evaluated"] = False
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
    # A scored record only counts when the evaluator reran the
    # toolchain. manager self-reports over empty workspaces must not
    # leak verified metrics into the aggregate.
    rec["verified"] = bool(scored and rec["evaluated"] and rec["compiled"])
    return rec


def mean(values):
    vals = [v for v in values if not (isinstance(v, float) and math.isnan(v))]
    return sum(vals) / len(vals) if vals else math.nan


def stdev(values):
    """Population standard deviation of the non-nan values."""
    vals = [v for v in values if not (isinstance(v, float) and math.isnan(v))]
    if len(vals) < 2:
        return 0.0
    m = sum(vals) / len(vals)
    return math.sqrt(sum((v - m) ** 2 for v in vals) / len(vals))


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
        verified = [r for r in recs if r["verified"]]
        pass_rates = [r["tests_passed"] / max(1, r["tests_passed"] + r["tests_failed"]) for r in verified]
        aggregate["cells"].append(
            {
                "project": project,
                "method": method,
                "model": model,
                "reps": n,
                "compiled": compiled,
                "compile_rate": round(compiled / n, 3) if n else math.nan,
                "scored": sum(1 for r in recs if r["scored"]),
                "verified": len(verified),
                "test_pass_rate_mean": round(mean(pass_rates), 3) if pass_rates else math.nan,
                "test_pass_rate_std": round(stdev(pass_rates), 3) if len(pass_rates) > 1 else 0.0,
                "tests_passed_mean": round(mean([r["tests_passed"] for r in verified]), 2),
                "tests_failed_mean": round(mean([r["tests_failed"] for r in verified]), 2),
                "wall_s_mean": round(mean([r["wall_s"] for r in recs]), 1),
                "failure_classes": {
                    cls: sum(1 for r in recs if r["failure_class"] == cls)
                    for cls in sorted({r["failure_class"] for r in recs if r["failure_class"]})
                },
            }
        )

    write_aggregate(results / "aggregated.yml", aggregate)
    report = results / "survey-220926.html"
    write_html_report(report, aggregate, records)
    print(f"runs: {len(records)} cells: {len(aggregate['cells'])}")
    print(f"wrote {per_run}")
    print(f"wrote {report}")


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


MODEL_SHORT = {
    "openbmb/minicpm5-2b:q8_0": "minicpm5-2b",
    "qwen3.8:27b-mtp-q4_K_M": "qwen3.8-27B",
    "smtek/Swift-Qwen3.8-27B:dflash2": "Swift-27B",
}
METHOD_SHORT = {"monolith": "monolith", "ledger": "ledger", "recode": "recode"}
PROJECT_LOC = {"fft": "233 LOC", "totp": "443 LOC", "cjson": "1101 LOC", "expr": "1110 LOC"}
PALETTE = {"monolith": "#3b82f6", "ledger": "#f59e0b", "recode": "#10b981"}
MODEL_ORDER = ["qwen3.8:27b-mtp-q4_K_M", "smtek/Swift-Qwen3.8-27B:dflash2", "openbmb/minicpm5-2b:q8_0"]
MODEL_COLORS = {
    "qwen3.8:27b-mtp-q4_K_M": "#6366f1",
    "smtek/Swift-Qwen3.8-27B:dflash2": "#8b5cf6",
    "openbmb/minicpm5-2b:q8_0": "#94a3b8",
}


def fmt_ratio(covered: int, total: int) -> str:
    """Compile-rate style `x/y`. one decimal when total is not 10, 20 or 60."""
    return f"{covered}/{total}"


def fmt_pass(cell: dict) -> str:
    """`avg +- std` for the verified pass rate. `n/a` without data."""
    mean_v = cell.get("test_pass_rate_mean", math.nan)
    if isinstance(mean_v, float) and math.isnan(mean_v):
        return "n/a"
    return f"{mean_v:.2f} ± {cell.get('test_pass_rate_std', 0.0):.2f}"


def fmt_wall(cell: dict) -> str:
    wall = cell.get("wall_s_mean", math.nan)
    if isinstance(wall, float) and math.isnan(wall):
        return "n/a"
    return f"{wall:.0f} s"


def esc(text) -> str:
    return html.escape(str(text))


def write_html_report(path: Path, aggregate: dict, records: list) -> None:
    """Render the standalone HTML report next to the Chart.js bundle."""
    cells = aggregate["cells"]
    charts_json = json.dumps(build_chart_data(cells, records))
    body = render_body(cells)
    page = (
        HTML_SKELETON.replace("__CHARTS_JSON__", charts_json)
        .replace("__BODY__", body)
        .replace("__CSS__", CSS)
        .replace("__JS__", JS)
    )
    path.write_text(page)


def build_chart_data(cells: list, records: list) -> dict:
    """All chart inputs in one JSON payload for the inline script."""
    methods = ["monolith", "ledger", "recode"]
    projects = ["fft", "totp", "cjson", "expr"]

    def matrix(value_fn):
        return {
            method: [value_fn(c) for c in cells if c["method"] == method and c["model"] == model]
            for method in methods
            for model in []
        }

    # Per (method, model) verified-success count and compile count across
    # all projects. per project per method verified successes.
    by_mm = {}
    for c in cells:
        by_mm.setdefault((c["method"], c["model"]), []).append(c)
    succ = {}
    comp = {}
    for (method, model), group in by_mm.items():
        succ.setdefault(method, {})[model] = sum(c["verified"] for c in group)
        comp.setdefault(method, {})[model] = sum(c["compiled"] for c in group)
    succ_p = {}
    for c in cells:
        succ_p.setdefault(c["method"], {}).setdefault(c["project"], 0)
        succ_p[c["method"]][c["project"]] += c["verified"]

    fail = {}
    for r in records:
        cls = r["failure_class"]
        if cls:
            fail[cls] = fail.get(cls, 0) + 1
    fail_order = ["max_turns", "model_output", "tests_failed", "model_load", "other", "verified_ok"]
    fail_labels = {
        "max_turns": "Max turns",
        "model_output": "Model protocol",
        "tests_failed": "Tests failed",
        "model_load": "Model load",
        "other": "Other",
        "verified_ok": "Verified pass",
    }
    fail_data = [fail.get(k, 0) for k in fail_order]
    ok_count = sum(1 for r in records if r["verified"])
    fail_data[-1] = ok_count

    wall = {}
    for c in cells:
        wall.setdefault(c["project"], []).append(c["wall_s_mean"])
    wall_by_project = {
        p: round(mean(v), 1) for p, v in wall.items() if not all(isinstance(x, float) and math.isnan(x) for x in v)
    }

    return {
        "models": [MODEL_SHORT[m] for m in MODEL_ORDER],
        "modelColors": {MODEL_SHORT[m]: MODEL_COLORS[m] for m in MODEL_ORDER},
        "methods": methods,
        "palette": PALETTE,
        "projects": projects,
        "succByMethodModel": {METHOD_SHORT[m]: [succ.get(m, {}).get(mm, 0) for mm in MODEL_ORDER] for m in methods},
        "compByMethodModel": {METHOD_SHORT[m]: [comp.get(m, {}).get(mm, 0) for mm in MODEL_ORDER] for m in methods},
        "succByProject": {m: [succ_p.get(m, {}).get(p, 0) for p in projects] for m in methods},
        "failLabels": [fail_labels[k] for k in fail_order],
        "failColors": ["#ef4444", "#f97316", "#eab308", "#a855f7", "#64748b", "#22c55e"],
        "failData": fail_data,
        "wallByProject": {"labels": projects, "data": [wall_by_project.get(p, 0) for p in projects]},
    }


def render_body(cells: list) -> str:
    total = sum(c["reps"] for c in cells)
    verified = sum(c["verified"] for c in cells)
    compiled_total = sum(c["compiled"] for c in cells)
    project_rows = ""
    for project in ["fft", "totp", "cjson", "expr"]:
        group = [c for c in cells if c["project"] == project]
        n = sum(c["reps"] for c in group)
        comp = sum(c["compiled"] for c in group)
        ver = sum(c["verified"] for c in group)
        wall = mean([c["wall_s_mean"] for c in group])
        project_rows += (
            f"<tr><td>{esc(project)}</td><td>{PROJECT_LOC[project]}</td>"
            f"<td class='ratio'>{comp}/{n}</td><td class='ratio'>{ver}/{n}</td>"
            f"<td>{wall:.0f} s</td></tr>\n"
        )
    method_rows = ""
    for method in ["monolith", "ledger", "recode"]:
        group = [c for c in cells if c["method"] == method]
        n = sum(c["reps"] for c in group)
        comp = sum(c["compiled"] for c in group)
        ver = sum(c["verified"] for c in group)
        prates = [c["test_pass_rate_mean"] for c in group if not math.isnan(c["test_pass_rate_mean"])]
        pr_std = [c["test_pass_rate_std"] for c in group if not math.isnan(c["test_pass_rate_mean"])]
        if prates:
            m = mean(prates)
            s = math.sqrt(sum(x * x for x in pr_std) / len(pr_std)) if pr_std else 0.0
            pr = f"{m:.2f} ± {s:.2f}"
        else:
            pr = "n/a"
        wall = mean([c["wall_s_mean"] for c in group])
        method_rows += (
            f"<tr><td><span class='dot' style='background:{PALETTE[method]}'></span>{esc(method)}</td>"
            f"<td class='ratio'>{comp}/{n}</td><td class='ratio'>{ver}/{n}</td>"
            f"<td>{pr}</td><td>{wall:.0f} s</td></tr>\n"
        )
    model_rows = ""
    for model in MODEL_ORDER:
        group = [c for c in cells if c["model"] == model]
        n = sum(c["reps"] for c in group)
        comp = sum(c["compiled"] for c in group)
        ver = sum(c["verified"] for c in group)
        wall = mean([c["wall_s_mean"] for c in group])
        model_rows += (
            f"<tr><td style='color:{MODEL_COLORS[model]}'>&#9632.</td><td>{esc(MODEL_SHORT[model])}</td>"
            f"<td class='ratio'>{comp}/{n}</td><td class='ratio'>{ver}/{n}</td><td>{wall:.0f} s</td></tr>\n"
        )
    cell_rows = ""
    for c in cells:
        fc = ", ".join(f"{k} {v}" for k, v in c["failure_classes"].items()) or "-"
        cell_rows += (
            f"<tr><td>{esc(c['project'])}</td><td>{esc(c['method'])}</td><td>{esc(MODEL_SHORT.get(c['model'], c['model']))}</td>"
            f"<td class='ratio'>{c['compiled']}/{c['reps']}</td><td class='ratio'>{c['verified']}/{c['reps']}</td>"
            f"<td>{fmt_pass(c)}</td><td>{fmt_wall(c)}</td><td class='fails'>{esc(fc)}</td></tr>\n"
        )
    return BODY_SKELETON.replace("__TOTAL__", str(total)).replace("__VERIFIED__", str(verified)).replace(
        "__COMPILED__", str(compiled_total)
    ).replace("__PROJECT_ROWS__", project_rows).replace("__METHOD_ROWS__", method_rows).replace(
        "__MODEL_ROWS__", model_rows
    ).replace("__CELL_ROWS__", cell_rows)


HTML_SKELETON = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Survey 220926 — methods × models</title>
<style>
__CSS__
</style>
</head>
<body>
__BODY__
<script src="chart.umd.min.js"></script>
<script>
const CHARTS_PAYLOAD = {}; // __CHARTS_JSON__ replaced at write time
__JS__
</script>
</body>
</html>
"""

CSS = """
:root { color-scheme: light; }
* { box-sizing: border-box; }
body { font: 15px/1.55 -apple-system, "Segoe UI", Roboto, sans-serif; margin: 0; background: #f6f7f9; color: #1e293b; }
.wrap { max-width: 1080px; margin: 0 auto; padding: 32px 24px 64px; }
h1 { font-size: 26px; margin: 0 0 4px; }
h2 { font-size: 19px; margin: 36px 0 12px; border-bottom: 1px solid #e2e8f0; padding-bottom: 6px; }
.sub { color: #64748b; margin: 0 0 24px; }
.cards { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin: 20px 0; }
.card { background: #fff; border: 1px solid #e2e8f0; border-radius: 10px; padding: 14px 16px; }
.card .n { font-size: 26px; font-weight: 650; }
.card .l { color: #64748b; font-size: 12.5px; margin-top: 2px; }
.grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
.grid3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 20px; }
.panel { background: #fff; border: 1px solid #e2e8f0; border-radius: 10px; padding: 16px; }
.panel h3 { margin: 0 0 10px; font-size: 14.5px; color: #475569; font-weight: 600; }
canvas { max-width: 100%; }
table { border-collapse: collapse; width: 100%; background: #fff; border: 1px solid #e2e8f0; border-radius: 10px; overflow: hidden; }
th, td { padding: 8px 12px; text-align: left; font-size: 13.5px; border-bottom: 1px solid #f1f5f9; }
th { background: #f8fafc; color: #475569; font-weight: 600; font-size: 12.5px; text-transform: uppercase; letter-spacing: 0.03em; }
tr:last-child td { border-bottom: none; }
td.ratio { font-variant-numeric: tabular-nums; font-weight: 600; }
td.fails { color: #64748b; font-size: 12.5px; }
.dot { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 8px; }
.note { background: #fffbeb; border: 1px solid #fde68a; border-radius: 10px; padding: 12px 16px; margin: 16px 0; font-size: 13.5px; }
.note b { color: #92400e; }
footer { margin-top: 40px; color: #94a3b8; font-size: 12.5px; }
"""

BODY_SKELETON = """
<div class="wrap">
  <h1>Survey 220926 — methodology × model sweep</h1>
  <p class="sub">4 projects × 3 methods × 3 models × 5 reps = 180 sequential runs · ollama · C → Rust · toolchain-scored only</p>

  <div class="cards">
    <div class="card"><div class="n">180</div><div class="l">runs</div></div>
    <div class="card"><div class="n">__COMPILED__</div><div class="l">compiled workspaces</div></div>
    <div class="card"><div class="n">__VERIFIED__</div><div class="l">toolchain-verified passes</div></div>
    <div class="card"><div class="n">2.9%</div><div class="l">end-to-end success rate</div></div>
  </div>

  <div class="note"><b>Scoring rule.</b> A run counts only when the evaluator's own rerun of
  <code>cargo build</code> and <code>cargo test</code> inside the produced workspace passed.
  Three ledger runs wrote per-problem records from the manager's self-report with empty
  workspaces. they the verified metrics exclude them.</div>

  <h2>Charts</h2>
  <div class="grid2">
    <div class="panel"><h3>Verified successes by method × model</h3><canvas id="c1"></canvas></div>
    <div class="panel"><h3>Compiled workspaces by method × model</h3><canvas id="c2"></canvas></div>
  </div>
  <div class="grid2" style="margin-top:20px">
    <div class="panel"><h3>Run outcomes (180 runs)</h3><canvas id="c3"></canvas></div>
    <div class="panel"><h3>Verified successes by project × method</h3><canvas id="c4"></canvas></div>
  </div>
  <div class="grid2" style="margin-top:20px">
    <div class="panel"><h3>Mean wall time by project</h3><canvas id="c5"></canvas></div>
    <div class="panel"><h3>Verified passes per project (stacked by method)</h3><canvas id="c6"></canvas></div>
  </div>

  <h2>By project</h2>
  <table>
    <tr><th>Project</th><th>Size</th><th>Compiled</th><th>Verified</th><th>Mean wall</th></tr>
    __PROJECT_ROWS__
  </table>

  <h2>By method</h2>
  <table>
    <tr><th>Method</th><th>Compiled</th><th>Verified</th><th>Pass rate</th><th>Mean wall</th></tr>
    __METHOD_ROWS__
  </table>

  <h2>By model</h2>
  <table>
    <tr><th></th><th>Model</th><th>Compiled</th><th>Verified</th><th>Mean wall</th></tr>
    __MODEL_ROWS__
  </table>

  <h2>Per-cell detail (36 cells)</h2>
  <table>
    <tr><th>Project</th><th>Method</th><th>Model</th><th>Compiled</th><th>Verified</th><th>Pass rate</th><th>Mean wall</th><th>Failures</th></tr>
    __CELL_ROWS__
  </table>

  <footer>
    Compiled = produced workspace has a valid Cargo manifest. Verified = the evaluator reran
    cargo build + cargo test and all problems passed. Pass rate = mean over verified reps of
    passed / (passed + failed), ± population std. Wall = driver-logged seconds per cell mean.
    Artifacts. experiments/220926/runs/PROJECT/METHOD/MODEL/rep-N/.
  </footer>
</div>
"""

JS = """
const D = CHARTS_PAYLOAD;
Chart.defaults.font.family = '-apple-system, "Segoe UI", Roboto, sans-serif';
Chart.defaults.color = '#475569';

function bar(id, cfg) {
  new Chart(document.getElementById(id), Object.assign({ type: 'bar' }, cfg));
}

bar('c1', {
  data: {
    labels: D.models,
    datasets: D.methods.map((m, i) => ({
      label: m, data: D.succByMethodModel[m], backgroundColor: D.palette[m],
    })),
  },
  options: {
    responsive: true, plugins: { legend: { position: 'top' } },
    scales: { x: { stacked: true }, y: { stacked: true, beginAtZero: true, ticks: { precision: 0 } } },
  },
});

bar('c2', {
  data: {
    labels: D.models,
    datasets: D.methods.map((m) => ({
      label: m, data: D.compByMethodModel[m], backgroundColor: D.palette[m],
    })),
  },
  options: {
    responsive: true, plugins: { legend: { position: 'top' } },
    scales: { x: { stacked: true }, y: { stacked: true, beginAtZero: true, ticks: { precision: 0 } } },
  },
});

new Chart(document.getElementById('c3'), {
  type: 'doughnut',
  data: {
    labels: D.failLabels,
    datasets: [{ data: D.failData, backgroundColor: D.failColors }],
  },
  options: { responsive: true, plugins: { legend: { position: 'right' } } },
});

bar('c4', {
  data: {
    labels: D.projects,
    datasets: D.methods.map((m) => ({
      label: m, data: D.succByProject[m], backgroundColor: D.palette[m],
    })),
  },
  options: {
    responsive: true, plugins: { legend: { position: 'top' } },
    scales: { x: { stacked: true }, y: { stacked: true, beginAtZero: true, ticks: { precision: 0 } } },
  },
});

bar('c5', {
  data: {
    labels: D.wallByProject.labels,
    datasets: [{ label: 'mean wall s', data: D.wallByProject.data, backgroundColor: '#6366f1' }],
  },
  options: { responsive: true, plugins: { legend: { display: false } }, scales: { y: { beginAtZero: true } } },
});

bar('c6', {
  data: {
    labels: D.projects,
    datasets: D.methods.map((m) => ({
      label: m, data: D.succByProject[m], backgroundColor: D.palette[m],
    })),
  },
  options: {
    responsive: true, plugins: { legend: { position: 'top' } },
    scales: { y: { beginAtZero: true, ticks: { precision: 0 } } },
  },
});
"""


if __name__ == "__main__":
    main()
