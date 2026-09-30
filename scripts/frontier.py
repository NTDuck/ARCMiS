#!/usr/bin/env python3
"""Aggregate the experiment tree and print the Pareto frontier.

Reads .artifacts/experiments/ (or the path given as argv[1]). For every
experiment directory it reads:

  manifest.json          - method, model, budgets, lineage
  result/per_problem.json - per-problem records (toolchain rescored)
  result/*.yml           - the aggregate result yaml, when present

Prints one JSON object with per-candidate aggregates, the parent map,
and the success/frontier ordering. Exits 0 when at least one experiment
scored, 2 when the tree is empty or unreadable.

The frontier keys are pass rate (max), per-problem test count, and wall
time from experiments.log when present. A candidate dominates another
when it is not worse on every key and strictly better on one.
"""
import json
import re
import sys
from pathlib import Path


def load_candidate(exp_dir: Path) -> dict | None:
    manifest_path = exp_dir / "manifest.json"
    if not manifest_path.is_file():
        return None
    try:
        manifest = json.loads(manifest_path.read_text())
    except json.JSONDecodeError:
        return None
    cand = {
        "candidate": manifest.get("candidate_id", exp_dir.name),
        "method": manifest.get("method", "unknown"),
        "model": manifest.get("model", "unknown"),
        "parents": manifest.get("parents", []),
        "hypothesis": manifest.get("hypothesis", ""),
        "git_revision": manifest.get("git_revision", "unknown"),
    }
    # Wall-time metric: run wall seconds from result/aggregate.json, with
    # per-task wall (run wall / completed tasks) as the cost key.
    agg_path = exp_dir / "result" / "aggregate.json"
    if agg_path.is_file():
        try:
            agg = json.loads(agg_path.read_text())
            wall = agg.get("run_time_seconds")
            done = agg.get("tasks_done", 0)
            if wall is not None:
                cand["wall_seconds"] = wall
                cand["wall_seconds_per_task"] = round(wall / done, 1) if done else None
            cand["tasks_done"] = done
            cand["tasks_total"] = agg.get("tasks_total")
        except json.JSONDecodeError:
            pass
    problems_path = exp_dir / "result" / "per_problem.json"
    if problems_path.is_file():
        try:
            records = json.loads(problems_path.read_text())
        except json.JSONDecodeError:
            records = []
        cand["problems"] = len(records)
        cand["problems_passed"] = sum(1 for r in records if r.get("success"))
        cand["tests_passed"] = sum(r.get("tests_passed", 0) for r in records)
        cand["tests_failed"] = sum(r.get("tests_failed", 0) for r in records)
        stages = [r.get("stage") for r in records if not r.get("success")]
        cand["failure_stages"] = sorted(set(stages))
        cand["pass_rate"] = (
            cand["problems_passed"] / cand["problems"] if cand["problems"] else 0.0
        )
    else:
        cand["problems"] = 0
        cand["problems_passed"] = 0
        cand["tests_passed"] = 0
        cand["tests_failed"] = 0
        cand["failure_stages"] = []
        cand["pass_rate"] = 0.0
    return cand


def dominates(a: dict, b: dict) -> bool:
    keys_ge = [
        a["pass_rate"] >= b["pass_rate"],
        a["tests_passed"] >= b["tests_passed"],
        a["tests_failed"] <= b["tests_failed"],
    ]
    keys_gt = [
        a["pass_rate"] > b["pass_rate"],
        a["tests_passed"] > b["tests_passed"],
        a["tests_failed"] < b["tests_failed"],
    ]
    # Time is a first-class metric: lower is better. Only comparable when
    # both candidates recorded it.
    if a.get("wall_seconds_per_task") is not None and b.get("wall_seconds_per_task") is not None:
        keys_ge.append(a["wall_seconds_per_task"] <= b["wall_seconds_per_task"])
        keys_gt.append(a["wall_seconds_per_task"] < b["wall_seconds_per_task"])
    return all(keys_ge) and any(keys_gt)
def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".artifacts/experiments")
    candidates = []
    for exp_dir in sorted(root.iterdir()) if root.is_dir() else []:
        if not exp_dir.is_dir():
            continue
        cand = load_candidate(exp_dir)
        if cand:
            candidates.append(cand)
    if not candidates:
        print(json.dumps({"error": "no scored experiments", "root": str(root)}))
        sys.exit(2)
    frontier = [c for c in candidates if not any(dominates(o, c) for o in candidates if o is not c)]
    out = {
        "root": str(root),
        "n": len(candidates),
        "candidates": candidates,
        "frontier": [c["candidate"] for c in frontier],
    }
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
