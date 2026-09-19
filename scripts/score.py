#!/usr/bin/env python3
"""Score one bench run from its workspace.

Reads the harness result yaml when present. Otherwise runs the recorded
toolchain (build + test) directly in the workspace and parses the counts.
Prints one JSON object per workspace on stdout.
"""
import json
import re
import subprocess
import sys
from pathlib import Path


def result_yaml(ws: Path):
    d = ws / ".ARCMiS" / "result"
    if not d.is_dir():
        return None
    ymls = sorted(d.glob("*.yml"))
    if not ymls:
        return None
    return ymls[-1]


def parse_yml(text: str) -> dict:
    # Tiny subset: keys with scalar values plus steps list entries.
    out = {}
    steps = []
    for line in text.splitlines():
        m = re.match(r"^(\w+): (.*)$", line)
        if m and m.group(1) not in ("steps",):
            out[m.group(1)] = m.group(2)
    return out


def run_tests(ws: Path):
    r = subprocess.run(
        ["cargo", "test"], cwd=ws, capture_output=True, text=True, timeout=600
    )
    text = r.stdout + r.stderr
    passed = sum(int(m) for m in re.findall(r"(\d+) passed", text))
    failed = sum(int(m) for m in re.findall(r"(\d+) failed", text))
    return r.returncode, passed, failed


def score(ws: Path) -> dict:
    rec = {"workspace": str(ws)}
    y = result_yaml(ws)
    if y:
        rec["result_yaml"] = str(y)
        rec.update(parse_yml(y.read_text()))
    build = subprocess.run(
        ["cargo", "build"], cwd=ws, capture_output=True, text=True, timeout=600
    )
    rec["build_rc"] = build.returncode
    if build.returncode == 0:
        rc, p, f = run_tests(ws)
        rec["test_rc"] = rc
        rec["tests_passed"] = p
        rec["tests_failed"] = f
        rec["compiles"] = True
        rec["all_tests_pass"] = rc == 0
    else:
        rec["compiles"] = False
        rec["all_tests_pass"] = False
        rec["tests_passed"] = 0
        rec["tests_failed"] = 0
    return rec


def main():
    root = Path(sys.argv[1])
    for ws in sorted(root.glob("*/rep*/workspace")):
        if not (ws / "Cargo.toml").exists():
            print(json.dumps({"workspace": str(ws), "compiles": False, "empty": True}))
            continue
        print(json.dumps(score(ws)))


if __name__ == "__main__":
    main()
