#!/usr/bin/env python3
"""Append a single-variable override block under the mas: key of a config."""
import sys, pathlib

cfg_path, ov_path = sys.argv[1:3]
cfg = pathlib.Path(cfg_path).read_text()
ov = pathlib.Path(ov_path).read_text().strip()
# insert right after the first 'mas:' line, indented one level
lines = cfg.splitlines(keepends=True)
for i, line in enumerate(lines):
    if line.rstrip("\n") == "mas:":
        indent = "  "
        block = "".join(indent + l + "\n" for l in ov.splitlines())
        lines.insert(i + 1, block)
        break
else:
    sys.exit("no mas: key in config")
pathlib.Path(cfg_path).write_text("".join(lines))
