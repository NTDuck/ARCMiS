#!/usr/bin/env python3
"""Write the manifest for one sweep round (autoopt-v0.3.N naming)."""
import json, subprocess, sys, pathlib, datetime

exp, idx, name, root = sys.argv[1:5]
parents = sorted(
    p.name for p in pathlib.Path(".artifacts/experiments").glob("*v3s*")
    if p.name.endswith(f"v3s{int(idx)-1}-*") or f"v3s{int(idx)-1}-" in p.name
)[-1:]
man = {
    "version": 3,
    "round": f"autoopt-v0.3.{int(idx)+8}",
    "legacy_tag": f"v3s{idx}",
    "harness_id": "mas-sweep-" + datetime.datetime.now(datetime.UTC).strftime("%Y%m%dT%H%M%SZ"),
    "method": "mas",
    "model": "qwen3.8-27b (local ninfer-serve)",
    "git_revision": subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip(),
    "budgets": {"orchestrator_turns": 20, "worker_turns": 40, "max_rounds": 20},
    "config_path": f".artifacts/experiments/{exp}/config.yml",
    "problem_set": root,
    "problem_name": name,
    "hypothesis": pathlib.Path(exp, "hypothesis.txt").read_text().strip() if pathlib.Path(exp, "hypothesis.txt").exists() else "coverage sweep round",
    "parents": parents,
}
pathlib.Path(exp, "manifest.json").write_text(json.dumps(man, indent=2) + "\n")
