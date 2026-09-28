#!/usr/bin/env python3
"""Toolchain rescore for one experiment directory.

The MAS v1 harness (post a2e41dd) writes manifest.json, traces/turns.jsonl,
and result/aggregate.yml, but no result/per_problem.json. The frontier
aggregator reads per_problem.json to compute pass rate and test counts.
This script runs the toolchain in the workspace and writes the missing
record. It is the scoring step the contract expects ("result/ after
scoring"); the harness exit code reports only the agents' build verdict.

Usage: rescore.py <experiment-dir>
"""
import json
import re
import subprocess
import sys
from pathlib import Path


def run(cmd, cwd, timeout=600):
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout + "\n" + r.stderr
    except subprocess.TimeoutExpired:
        return 124, "timeout"


def parse_test_counts(text):
    p = sum(int(m) for m in re.findall(r"(\d+) passed", text))
    f = sum(int(m) for m in re.findall(r"(\d+) failed", text))
    return p, f


def find_project_root(workspace: Path, target_language: str) -> Path:
    target = workspace / "target"
    if target.is_dir():
        if target_language == "rust" and (target / "Cargo.toml").is_file():
            return target
        if target_language == "python" and any(target.rglob("pyproject.toml")):
            return target
        if target_language == "javascript" and any(target.rglob("package.json")):
            return target
    return workspace


def rescore(exp_dir: Path) -> dict:
    manifest = json.loads((exp_dir / "manifest.json").read_text())
    target_language = manifest.get("target_language", "rust")
    workspace = exp_dir / "workspace"
    # A run without a translated tree must fail here, not fall through to
    # running the test command in the caller's own repo root (which once
    # scored the harness's own suite as the workspace's).
    if not (workspace / "target").is_dir():
        print(
            f"rescore: {exp_dir}: no workspace/target translated tree; nothing to score",
            file=sys.stderr,
        )
        sys.exit(1)
    if not workspace.is_dir():
        return {"problem": "workspace", "success": False, "stage": "setup",
                "tests_passed": 0, "tests_failed": 0, "detail": "workspace missing"}
    test_command = ""
    cfg_path = exp_dir / "config.yml"
    if cfg_path.is_file():
        for line in cfg_path.read_text().splitlines():
            m = re.match(r"^\s*test_command:\s*(.+?)\s*$", line)
            if m:
                test_command = m.group(1).strip()
                break

    project_root = find_project_root(workspace, target_language)
    record = {"problem": "workspace", "success": False, "stage": "evaluate",
              "tests_passed": 0, "tests_failed": 0, "detail": ""}

    if target_language == "rust":
        rc, out = run(["cargo", "build"], cwd=project_root, timeout=900)
        if rc != 0:
            record["stage"] = "translate"
            record["detail"] = out[-400:]
            return record
        if test_command:
            rc, out = run(["sh", "-c", test_command], cwd=project_root, timeout=900)
        else:
            rc, out = run(["cargo", "test"], cwd=project_root, timeout=900)
    else:
        if test_command:
            rc, out = run(["sh", "-c", test_command], cwd=project_root, timeout=900)
        else:
            record["stage"] = "validate"
            record["detail"] = "no test_command configured"
            return record

    p, f = parse_test_counts(out)
    record["tests_passed"] = p
    record["tests_failed"] = f
    record["success"] = rc == 0 and f == 0
    if rc != 0:
        record["stage"] = "validate"
    record["detail"] = out[-400:]
    return record


def main():
    exp_dir = Path(sys.argv[1])
    record = rescore(exp_dir)
    result_dir = exp_dir / "result"
    result_dir.mkdir(parents=True, exist_ok=True)
    (result_dir / "per_problem.json").write_text(json.dumps([record], indent=2) + "\n")
    print(json.dumps(record))


if __name__ == "__main__":
    main()
