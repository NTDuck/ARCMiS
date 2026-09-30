#!/usr/bin/env python3
"""Aggregate the experiment tree and print the Pareto frontier.

Reads .artifacts/experiments/ (or the path given as argv[1]). For every
experiment directory it reads:

  manifest.json          - method, model, budgets, lineage
  result/per_problem.json - per-problem records (toolchain rescored)
  result/aggregate.yml   - harness aggregate (duration_seconds)
  result/aggregate.json  - machine-scored aggregate (run_time_seconds)
  run/ledgers/ and traces/ - defect-key counts (stalled rounds,
  output-cap deaths)

Prints one JSON object with per-candidate aggregates, the parent map,
and the success/frontier ordering. Exits 0 when at least one experiment
scored, 2 when the tree is empty or unreadable.

The frontier keys are pass rate (max), per-problem test count, tests
failed (min), and wall seconds per task (min). A candidate dominates
another when it is not worse on every key and strictly better on one.
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
    # Wall-time metric: result/aggregate.yml carries duration_seconds for
    # harness-reported runs. result/aggregate.json carries
    # run_time_seconds for machine-scored runs. Either one wins. Neither
    # one means unknown.
    wall = read_duration_seconds(exp_dir / "result" / "aggregate.yml")
    done = read_tasks_done(exp_dir / "result" / "aggregate.yml")
    if (agg_path := exp_dir / "result" / "aggregate.json").is_file():
        try:
            agg = json.loads(agg_path.read_text())
            wall = agg.get("run_time_seconds") if wall is None else wall
            done = agg.get("tasks_done", 0) if done is None else done
            cand["tasks_total"] = agg.get("tasks_total")
        except json.JSONDecodeError:
            pass
    if wall is not None:
        cand["wall_seconds"] = wall
        cand["wall_seconds_per_task"] = round(wall / done, 1) if done else None
    if done is not None:
        cand["tasks_done"] = done
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
    cand.update(count_defects(exp_dir))
    return cand


def read_duration_seconds(path: Path) -> float | None:
    """Read duration_seconds from a result/aggregate.yml, when present."""
    value = _yml_top_scalar(path, "duration_seconds")
    if value is None:
        return None
    try:
        return float(value)
    except ValueError:
        return None


def read_tasks_done(path: Path) -> int | None:
    """Read tasks_done from a result/aggregate.yml, when present."""
    value = _yml_top_scalar(path, "tasks_done")
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def _yml_top_scalar(path: Path, key: str) -> str | None:
    """Scan a flat result YAML for one top-level `key: value` scalar.

    Reads only what the harness writes. A YAML dependency is not needed
    for this narrow scan.
    """
    if not path.is_file():
        return None
    for line in path.read_text().splitlines():
        stripped = line.split("#", 1)[0].strip()
        if stripped.startswith(f"{key}:"):
            value = stripped.removeprefix(f"{key}:").strip()
            return value or None
    return None


def count_defects(exp_dir: Path) -> dict:
    """Count harness-defect signals for one experiment.

    Heuristics, documented because the sources are approximate:

    - stalled_rounds: rows in run/ledgers/observations.jsonl whose "kind"
      equals "round_stalled". Missing ledger yields no key (unknown).
    - output_cap_deaths: model_response rows in traces/turns.jsonl whose
      usage.output_tokens equals one of the configured caps AND whose text
      is empty with no tool calls - the signature of a generation run into
      the output ceiling. Approximate on purpose. A response that ends
      exactly at the cap with no payload is the observable artifact. Row
      payloads live under the "fields" envelope in the trace.
    """
    defects: dict = {}
    obs_path = exp_dir / "run" / "ledgers" / "observations.jsonl"
    if obs_path.is_file():
        stalled = 0
        for line in obs_path.read_text().splitlines():
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get("kind") == "round_stalled":
                stalled += 1
        defects["stalled_rounds"] = stalled
    caps = read_output_caps(exp_dir / "config.yml")
    turns_path = exp_dir / "traces" / "turns.jsonl"
    if caps and turns_path.is_file():
        cap_deaths = 0
        for line in turns_path.read_text().splitlines():
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get("event") != "model_response":
                continue
            fields = row.get("fields") or {}
            usage = fields.get("usage") or {}
            tokens = usage.get("output_tokens")
            if tokens not in caps:
                continue
            text = fields.get("text") or ""
            calls = fields.get("tool_calls") or []
            if not text.strip() and not calls:
                cap_deaths += 1
        defects["output_cap_deaths"] = cap_deaths
    return defects


def read_output_caps(path: Path) -> set:
    """Collect the configured output-token caps from config.yml.

    Reads `max_output_tokens: N`, and `role_output_tokens:` entries of the
    forms `role: N` and `- N`. Values below 1024 are not caps and are
    skipped (they are tuning knobs such as think budgets).
    """
    caps: set = set()
    if not path.is_file():
        return caps
    roles_indent = None
    for line in path.read_text().splitlines():
        stripped = line.split("#", 1)[0].rstrip()
        if not stripped.strip():
            continue
        indent = len(stripped) - len(stripped.lstrip())
        text = stripped.strip()
        if roles_indent is not None and indent <= roles_indent:
            roles_indent = None
        if roles_indent is None and text.startswith("role_output_tokens:"):
            roles_indent = indent
            inline = text.split(":", 1)[1].strip()
            if inline.isdigit():
                caps.add(int(inline))
            continue
        if roles_indent is not None:
            m = re.match(r"^(?:-\s+)?[\w-]+:\s*(\d+)$", text) or re.match(r"^-\s*(\d+)$", text)
            if m and int(m.group(1)) >= 1024:
                caps.add(int(m.group(1)))
            continue
        if text.startswith("max_output_tokens:"):
            value = text.split(":", 1)[1].strip()
            if value.isdigit():
                caps.add(int(value))
    return caps


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
