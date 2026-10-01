#!/usr/bin/env python3
"""Regenerate .artifacts/experiments/ROUNDS.yaml from the experiment tree.

One ledger entry per protocol round (v3r1..v3rN plus the v2r1 stray).
Run after every round close. The file is an output, the script is the
durable artifact.

Schema (one entry per round):

    round: v3rN                    # round tag parsed from the dir name
    candidate_id: <dir name>       # experiment directory
    parents: [...]                 # manifest.json parents
    hypothesis: <one line>         # manifest.json hypothesis
    defect_class_targeted: <slug>  # keyword map from the dir slug, else none
    metrics:
      wall_seconds: N              # result/aggregate.yml duration_seconds,
                                   # else result/aggregate.json run_time_seconds
      compile: true|false|unknown  # per_problem.json success with stage=evaluate
      tests_passed: N              # sum over result/per_problem.json
      tests_failed: N
      stalled_rounds: N            # run/ledgers/observations.jsonl kind=round_stalled
      max_turns_deaths: N          # run/ledgers/failures.jsonl root_cause MaxTurnsError
      output_cap_deaths: N         # heuristic: model_response rows at an exact
                                   # configured cap with empty text, no tool calls
                                   # (traces/turns.jsonl, "fields" envelope)
    delta_vs_parent:               # vs first-listed parent, when measurable
      tests_passed: +/-N
      wall_seconds: +/-N
    regression_flags: [...]        # wall regression >20%, test regression
    verdict: SOLVED|NOT CLEARED|ABORTED|NOT SCORED
    artifacts: {manifest, events, traces, result}   # present paths or null

Ground rules: every value comes from an on-disk artifact. Missing data is
`unknown`, never guessed. Known gaps, kept honest instead of patched:

- Manifests of v3r2..v3r7 carry empty parents and hypothesis, so deltas
  and regression flags stay unknown even though SUMMARY.md documents the
  lineage. Parsing SUMMARY.md would make regeneration brittle.
- v3r1 has no run/ ledger, so stalled_rounds and max_turns_deaths stay
  unknown there.
- Dir twins at 20260930T081500Z (broken v3r1 relaunch without a
  top-level manifest, and the v2r1 abort) - a round tag resolves to the
  dir with a parsable manifest.json. The generator skips the broken
  twin and lists it in a comment.
"""
import json
import re
import sys
from pathlib import Path

TAG_RE = re.compile(r"(?:^|[^a-z])(v[23][rs]\d+)(?:-|$)")

# Rename scheme (2026-10-01 directive): protocol versions map to
# autoopt-v0.N.x candidate names. On-disk directory names are timestamped
# history and stay; this mapping applies to the metadata layer only
# (ROUNDS.yaml round fields, SUMMARY headings, future manifest ids).
PROTOCOL_NAME = {
    1: "autoopt-v0.1",
    2: "autoopt-v0.2",
    3: "autoopt-v0.3",
}


def autoopt_name(tag: str) -> str:
    """Round tag v3r8 / v3s0 -> autoopt-v0.3.8 / autoopt-v0.3.8.

    Round and sweep tags share one candidate-numbering space (the sweep
    continues the campaign), so `s` maps like `r`.
    """
    protocol = int(tag[1])
    number = int(re.search(r"[rs](\d+)", tag).group(1))
    return f"{PROTOCOL_NAME.get(protocol, 'autoopt-v0.' + str(protocol))}.{number}"


HYPOTHESIS_DEFECT_KEYWORDS = [
    ("output cap", "output-cap"),
    ("output-cap", "output-cap"),
    ("stall", "stall-breaker"),
    ("provider", "provider-shape"),
    ("read", "read-discipline"),
    ("gate", "class2-echo"),
]
SLUG_DEFECT_KEYWORDS = [
    ("repair16384", "output-cap"),
    ("orchout16384", "output-cap"),
    ("replangate", "stall-breaker"),
    ("blockedgate", "class2-echo"),
]
WALL_REGRESSION_RATIO = 0.2


def find_round_dirs(root: Path) -> dict:
    """Map round tag -> primary experiment dir.

    A tag with several candidate dirs resolves to the one with a parsable
    top-level manifest.json (latest timestamp wins). The harness skips
    broken twins and reports them for the header comment.
    """
    rounds: dict = {}
    skipped: list = []
    for exp_dir in sorted(root.iterdir()):
        if not exp_dir.is_dir():
            continue
        m = TAG_RE.search(exp_dir.name)
        if not m:
            continue
        tag = m.group(1)
        if not (exp_dir / "manifest.json").is_file():
            skipped.append(exp_dir.name)
            continue
        try:
            json.loads((exp_dir / "manifest.json").read_text())
        except json.JSONDecodeError:
            skipped.append(exp_dir.name)
            continue
        if tag in rounds:
            skipped.append(exp_dir.name)
            continue
        rounds[tag] = exp_dir
    return rounds, skipped


def round_tag_order(tag: str):
    protocol = int(tag[1])
    number = int(re.search(r"[rs](\d+)", tag).group(1))
    return (protocol, number)


def read_json(path: Path):
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return None


def read_yml_scalar(path: Path, key: str):
    if not path.is_file():
        return None
    for line in path.read_text().splitlines():
        stripped = line.split("#", 1)[0].strip()
        if stripped.startswith(f"{key}:"):
            return stripped.removeprefix(f"{key}:").strip() or None
    return None


def read_output_caps(path: Path) -> set:
    """Collect configured output-token caps (max_output_tokens plus the
    role_output_tokens block). Values below 1024 are tuning knobs."""
    caps: set = set()
    if not path.is_file():
        return caps
    roles_indent = None
    for line in path.read_text().splitlines():
        stripped = line.split("#", 1)[0]
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


def count_ledger_rows(path: Path, kind: str) -> int | None:
    if not path.is_file():
        return None
    n = 0
    for line in path.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("kind") == kind:
            n += 1
    return n


def count_max_turns_deaths(path: Path) -> int | None:
    if not path.is_file():
        return None
    n = 0
    for line in path.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "MaxTurnsError" in (row.get("root_cause") or ""):
            n += 1
    return n


def count_output_cap_deaths(exp_dir: Path, caps: set) -> int | None:
    turns_path = exp_dir / "traces" / "turns.jsonl"
    if not caps or not turns_path.is_file():
        return None
    n = 0
    for line in turns_path.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("event") != "model_response":
            continue
        fields = row.get("fields") or {}
        usage = fields.get("usage") or {}
        if usage.get("output_tokens") not in caps:
            continue
        text = fields.get("text") or ""
        calls = fields.get("tool_calls") or []
        if not text.strip() and not calls:
            n += 1
    return n


def problem_metrics(exp_dir: Path) -> dict:
    out = {"compile": "unknown", "tests_passed": "unknown", "tests_failed": "unknown"}
    records = read_json(exp_dir / "result" / "per_problem.json")
    if not isinstance(records, list):
        return out
    passed = sum(r.get("tests_passed", 0) for r in records)
    failed = sum(r.get("tests_failed", 0) for r in records)
    out["tests_passed"] = passed
    out["tests_failed"] = failed
    out["compile"] = any(r.get("success") and r.get("stage") == "evaluate" for r in records)
    return out


def wall_seconds(exp_dir: Path):
    value = read_yml_scalar(exp_dir / "result" / "aggregate.yml", "duration_seconds")
    if value is not None:
        try:
            return float(value)
        except ValueError:
            pass
    agg = read_json(exp_dir / "result" / "aggregate.json")
    if isinstance(agg, dict) and agg.get("run_time_seconds") is not None:
        return float(agg["run_time_seconds"])
    return "unknown"


def derive_verdict(exp_dir: Path) -> str:
    if (exp_dir / "result" / "abort-note.txt").is_file():
        return "ABORTED"
    records = read_json(exp_dir / "result" / "per_problem.json")
    if isinstance(records, list):
        return "SOLVED" if any(r.get("success") for r in records) else "NOT CLEARED"
    agg = read_json(exp_dir / "result" / "aggregate.json")
    if read_yml_scalar(exp_dir / "result" / "aggregate.yml", "harness_id") or isinstance(agg, dict):
        return "NOT CLEARED"
    return "NOT SCORED"


def defect_class_from_slug(exp_dir: Path, manifest: dict) -> str:
    """Defect class from the manifest hypothesis keywords, else the dir
    slug. The hypothesis keyword that occurs earliest in the text wins
    (the opening clause states the target). Falls back to "unknown" when
    neither hypothesis nor slug carries a signal."""
    hypothesis = (manifest.get("hypothesis") or "").lower()
    best = None
    for keyword, defect in HYPOTHESIS_DEFECT_KEYWORDS:
        index = hypothesis.find(keyword)
        if index >= 0 and (best is None or index < best[0]):
            best = (index, defect)
    if best is not None:
        return best[1]
    for keyword, defect in SLUG_DEFECT_KEYWORDS:
        if keyword in exp_dir.name:
            return defect
    return "unknown"


def delta_vs_parent(exp_dir: Path, rounds: dict, metrics: dict) -> dict:
    manifest = read_json(exp_dir / "manifest.json") or {}
    parents = manifest.get("parents") or []
    if not parents:
        return "unknown"
    parent_dir = None
    for candidate in rounds.values():
        if candidate.name == parents[0] or candidate.name.endswith(parents[0]) or parents[0] in candidate.name:
            parent_dir = candidate
            break
    if parent_dir is None:
        return "unknown"
    parent_metrics = problem_metrics(parent_dir)
    parent_wall = wall_seconds(parent_dir)
    delta = {}
    if metrics["tests_passed"] != "unknown" and parent_metrics["tests_passed"] != "unknown":
        delta["tests_passed"] = metrics["tests_passed"] - parent_metrics["tests_passed"]
    else:
        delta["tests_passed"] = "unknown"
    if metrics["wall_seconds"] != "unknown" and parent_wall != "unknown":
        delta["wall_seconds"] = metrics["wall_seconds"] - parent_wall
    else:
        delta["wall_seconds"] = "unknown"
    return delta


def regression_flags(exp_dir: Path, rounds: dict, metrics: dict) -> list | str:
    manifest = read_json(exp_dir / "manifest.json") or {}
    parents = manifest.get("parents") or []
    flags: list = []
    if not parents:
        return "unknown"
    parent_dir = None
    for candidate in rounds.values():
        if candidate.name == parents[0] or candidate.name.endswith(parents[0]) or parents[0] in candidate.name:
            parent_dir = candidate
            break
    if parent_dir is None:
        return "unknown"
    parent_metrics = problem_metrics(parent_dir)
    parent_wall = wall_seconds(parent_dir)
    if (
        metrics["tests_passed"] != "unknown"
        and parent_metrics["tests_passed"] != "unknown"
        and metrics["tests_passed"] < parent_metrics["tests_passed"]
    ):
        flags.append("test-regression")
    if (
        metrics["wall_seconds"] != "unknown"
        and parent_wall != "unknown"
        and parent_wall > 0
        and (metrics["wall_seconds"] - parent_wall) / parent_wall > WALL_REGRESSION_RATIO
    ):
        flags.append("wall-regression-over-20-percent")
    return flags


def artifacts_entry(exp_dir: Path) -> dict:
    def present(rel: str):
        return rel if (exp_dir / rel).exists() else None

    return {
        "manifest": present("manifest.json"),
        "events": present("events.jsonl"),
        "traces": present("traces/turns.jsonl"),
        "result": "result" if (exp_dir / "result").is_dir() else None,
    }


def build_entry(tag: str, exp_dir: Path, rounds: dict) -> dict:
    manifest = read_json(exp_dir / "manifest.json") or {}
    caps = read_output_caps(exp_dir / "config.yml")
    metrics = problem_metrics(exp_dir)
    metrics["wall_seconds"] = wall_seconds(exp_dir)
    metrics["stalled_rounds"] = count_ledger_rows(
        exp_dir / "run" / "ledgers" / "observations.jsonl", "round_stalled"
    )
    metrics["max_turns_deaths"] = count_max_turns_deaths(
        exp_dir / "run" / "ledgers" / "failures.jsonl"
    )
    metrics["output_cap_deaths"] = count_output_cap_deaths(exp_dir, caps)
    return {
        "round": autoopt_name(tag),
        "legacy_tag": tag,
        "protocol": f"v{tag[1]}",
        "candidate_id": exp_dir.name,
        "parents": manifest.get("parents", []),
        "hypothesis": manifest.get("hypothesis") or "unknown",
        "defect_class_targeted": defect_class_from_slug(exp_dir, manifest),
        "metrics": metrics,
        "delta_vs_parent": delta_vs_parent(exp_dir, rounds, metrics),
        "regression_flags": regression_flags(exp_dir, rounds, metrics),
        "verdict": derive_verdict(exp_dir),
        "artifacts": artifacts_entry(exp_dir),
    }


def fmt(value) -> str:
    if value is None or value == "unknown":
        return "unknown"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, float) and value.is_integer():
        return str(int(value))
    return json.dumps(value)


def render_yaml(rounds_out: list, skipped: list) -> str:
    lines = [
        "# ROUNDS.yaml - per-round traceability ledger (generated).",
        "# Regenerate with: python3 scripts/rounds_ledger.py",
        "# Schema and ground rules are documented in scripts/rounds_ledger.py.",
    ]
    if skipped:
        lines.append("# Skipped dirs (no parsable top-level manifest.json): " + ", ".join(skipped))
    lines.append("rounds:")
    for entry in rounds_out:
        lines.append(f"  - round: {entry['round']}")
        lines.append(f"    protocol: {entry['protocol']}")
        lines.append(f"    candidate_id: {entry['candidate_id']}")
        lines.append(f"    parents: {json.dumps(entry['parents'])}")
        lines.append(f"    hypothesis: {json.dumps(entry['hypothesis'])}")
        lines.append(f"    defect_class_targeted: {entry['defect_class_targeted']}")
        lines.append("    metrics:")
        for key in (
            "wall_seconds",
            "compile",
            "tests_passed",
            "tests_failed",
            "stalled_rounds",
            "max_turns_deaths",
            "output_cap_deaths",
        ):
            lines.append(f"      {key}: {fmt(entry['metrics'][key])}")
        delta = entry["delta_vs_parent"]
        if delta == "unknown":
            lines.append("    delta_vs_parent: unknown")
        else:
            lines.append("    delta_vs_parent:")
            lines.append(f"      tests_passed: {fmt(delta['tests_passed'])}")
            lines.append(f"      wall_seconds: {fmt(delta['wall_seconds'])}")
        flags = entry["regression_flags"]
        if flags == "unknown":
            lines.append("    regression_flags: unknown")
        else:
            lines.append(
                "    regression_flags: " + (json.dumps(flags) if flags else "[]")
            )
        lines.append(f"    verdict: {entry['verdict']}")
        lines.append("    artifacts:")
        for key in ("manifest", "events", "traces", "result"):
            value = entry["artifacts"][key]
            lines.append(f"      {key}: {json.dumps(value) if value is not None else 'null'}")
    return "\n".join(lines) + "\n"


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".artifacts/experiments")
    out_path = root / "ROUNDS.yaml"
    rounds, skipped = find_round_dirs(root)
    ordered = sorted(rounds, key=round_tag_order)
    entries = [build_entry(tag, rounds[tag], rounds) for tag in ordered]
    out_path.write_text(render_yaml(entries, skipped))
    print(f"wrote {out_path} with {len(entries)} rounds")


if __name__ == "__main__":
    main()
