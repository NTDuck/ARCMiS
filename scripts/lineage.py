#!/usr/bin/env python3
"""Patch lineage fields into one experiment manifest.

Usage: lineage.py <manifest.json> <parents> <hypothesis>
  parents:    comma-separated experiment ids, empty for none
  hypothesis: free text, empty for none

The harness writes the manifest before the run; the driver patches the
lineage afterwards because the proposer, not the harness, owns lineage.
"""
import json
import sys
from pathlib import Path


def main():
    path = Path(sys.argv[1])
    parents_raw = sys.argv[2] if len(sys.argv) > 2 else ""
    hypothesis = sys.argv[3] if len(sys.argv) > 3 else ""
    manifest = json.loads(path.read_text())
    manifest["parents"] = [p.strip() for p in parents_raw.split(",") if p.strip()]
    manifest["hypothesis"] = hypothesis
    path.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
