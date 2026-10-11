#!/usr/bin/env python3
"""Write the manifest for one sweep round (autoopt-v0.3.N naming).

The hypothesis is mandatory (ADR 0020): pass $HYPOTHESIS or write
hypothesis.txt in the experiment dir before calling. Parents are the
newest prior round dirs of this sweep lineage.
"""
import json, os, subprocess, sys, pathlib, datetime

exp, idx, name, root = sys.argv[1:5]
hypothesis = os.environ.get("HYPOTHESIS", "").strip()
hf = pathlib.Path(exp, "hypothesis.txt")
if not hypothesis and hf.exists():
    hypothesis = hf.read_text().strip()
if not hypothesis:
    sys.exit(f"manifest requires a hypothesis: set $HYPOTHESIS or {hf}")

here = pathlib.Path(exp).name
prior = sorted(p.name for p in pathlib.Path(".artifacts/experiments").glob("*v3s*")
               if p.name < here)
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
    "hypothesis": hypothesis,
    "parents": prior[-2:],
}
pathlib.Path(exp, "manifest.json").write_text(json.dumps(man, indent=2) + "\n")
