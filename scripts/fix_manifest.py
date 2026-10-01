#!/usr/bin/env python3
"""Backfill hypothesis+parents on a sweep round manifest.

The harness overwrites manifest.json at startup; when a round was
launched without the HARNESS_* environment, this restores the lineage
after the fact. Idempotent: never overwrites a non-empty hypothesis.
"""
import json, sys, pathlib

exp = pathlib.Path(sys.argv[1])
m = json.loads((exp / 'manifest.json').read_text())
if not m.get('hypothesis'):
    hf = exp / 'hypothesis.txt'
    if hf.exists():
        m['hypothesis'] = hf.read_text().strip()
if not m.get('parents'):
    prior = sorted(p.name for p in exp.parent.glob('*v3s*') if p.name < exp.name)
    m['parents'] = prior[-2:]
(exp / 'manifest.json').write_text(json.dumps(m, indent=2) + '\n')
print(f"{exp.name}: hypothesis={'set' if m['hypothesis'] else 'EMPTY'} parents={len(m['parents'])}")
