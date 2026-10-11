#!/usr/bin/env python3
"""Deterministic bottleneck scan over ARCMiS sweep experiments (ADR 0028).

Scans every `.artifacts/experiments/*v3s*` directory and ranks rounds by
measured bottleneck signals: repairer dispatches, stalled rounds, max-turns
and output-cap deaths, escalations, context-length failures, empty model
responses, source size vs context window, and translator output-token share.
Recorded `jev_triage` / `jev_round_triage` observation rows are joined when
present; rounds without them are labeled `uninstrumented`. Every unknown is
reported as `unknown`, never guessed.

Usage: python3 scripts/bottlenecks.py [--root DIR] [--json]
"""

import argparse
import json
import sys
from pathlib import Path

# Rough token estimate for source bytes (chars ~= 4 bytes per token for code).
BYTES_PER_TOKEN = 4


def read_jsonl(path):
    """Read a JSONL file; a missing file yields no rows."""
    try:
        with open(path, encoding="utf-8") as handle:
            return [json.loads(line) for line in handle if line.strip()]
    except FileNotFoundError:
        return []
    except (json.JSONDecodeError, OSError) as error:
        print(f"warn: {path}: {error}", file=sys.stderr)
        return []


def read_yml_scalars(path):
    """Flat yml scalar reader for the harness aggregate (top-level keys)."""
    fields = {}
    try:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                stripped = line.split("#")[0].strip()
                if ":" in stripped and not stripped.startswith((" ", "-", "\t")):
                    key, _, value = stripped.partition(":")
                    fields[key.strip()] = value.strip().strip('"')
    except FileNotFoundError:
        pass
    except OSError as error:
        print(f"warn: {path}: {error}", file=sys.stderr)
    return fields


def count_repair_dispatches(decisions):
    """Repairer dispatch rows in the decisions ledger."""
    return sum(
        1
        for row in decisions
        if row.get("action") == "delegate"
        and (row.get("detail") or {}).get("role") == "repairer"
    )


def trace_stats(traces, cap_tokens):
    """Model-response stats from the trace: totals, empties, per-agent output."""
    total_output = 0
    empty_responses = 0
    agent_output = {}
    agent_calls = {}
    for row in traces:
        if row.get("event") != "model_response":
            continue
        fields = row.get("fields") or {}
        agent = fields.get("agent") or "unknown"
        agent_calls[agent] = agent_calls.get(agent, 0) + 1
        usage = fields.get("usage") or {}
        output = usage.get("output_tokens")
        if isinstance(output, (int, float)):
            total_output += output
            agent_output[agent] = agent_output.get(agent, 0) + output
        text = fields.get("text") or ""
        tool_calls = fields.get("tool_calls") or []
        if not text.strip() and not tool_calls:
            empty_responses += 1
    translator_share = (
        f"{100.0 * agent_output.get('translator', 0) / total_output:.1f}%"
        if total_output
        else "unknown"
    )
    capped = sum(
        1
        for row in traces
        if row.get("event") == "model_response"
        and ((row.get("fields") or {}).get("usage") or {}).get("output_tokens")
        and isinstance(cap_tokens, int)
        and cap_tokens > 0
        and ((row.get("fields") or {}).get("usage") or {}).get("output_tokens") >= cap_tokens
    )
    return {
        "model_responses": sum(1 for row in traces if row.get("event") == "model_response"),
        "empty_responses": empty_responses,
        "translator_output_share": translator_share,
        "cap_hits": capped,
        "agent_calls": agent_calls,
    }


def source_bytes(workspace):
    """Total size of the translated source tree, 0 when absent."""
    root = workspace / "source"
    if not root.is_dir():
        root = workspace
    total = 0
    for path in root.rglob("*"):
        try:
            if path.is_file() and ".git" not in path.parts:
                total += path.stat().st_size
        except OSError:
            continue
    return total


def verdict_from_ledgers(exp_dir):
    """Recorded triage observation rows, joined when present."""
    observations = read_jsonl(exp_dir / "run" / "ledgers" / "observations.jsonl")
    dispatch = [row for row in observations if row.get("kind") == "jev_triage"]
    round_rows = [row for row in observations if row.get("kind") == "jev_round_triage"]
    if not dispatch and not round_rows:
        return "uninstrumented", []
    actions = [row.get("detail", {}).get("action") for row in dispatch]
    return f"dispatch_rows={len(dispatch)} round_rows={len(round_rows)} actions={actions or 'unknown'}", actions


def scan_experiment(exp_dir):
    """One experiment -> one row of measured signals."""
    decisions = read_jsonl(exp_dir / "run" / "ledgers" / "decisions.jsonl")
    failures = read_jsonl(exp_dir / "run" / "ledgers" / "failures.jsonl")
    observations = read_jsonl(exp_dir / "run" / "ledgers" / "observations.jsonl")
    traces = read_jsonl(exp_dir / "traces" / "turns.jsonl")

    config = {}
    config_path = exp_dir / "config.yml"
    try:
        config = read_yml_scalars(config_path)
        num_ctx = None
        max_output = None
        with open(config_path, encoding="utf-8") as handle:
            for line in handle:
                stripped = line.split("#")[0].strip()
                if stripped.startswith("num_ctx:"):
                    num_ctx = int(stripped.partition(":")[2] or 0)
                if stripped.startswith("max_output_tokens:"):
                    max_output = int(stripped.partition(":")[2] or 0)
    except (OSError, ValueError):
        pass

    causes = [row.get("root_cause") or "" for row in failures]
    stalled = sum(1 for row in observations if row.get("kind") == "round_stalled")
    aggregate = read_yml_scalars(exp_dir / "result" / "aggregate.yml")
    stats = trace_stats(traces, max_output)
    src = source_bytes(exp_dir / "workspace")
    risk = (
        f"source~{src // BYTES_PER_TOKEN}tok > num_ctx {num_ctx}"
        if num_ctx and src // BYTES_PER_TOKEN > num_ctx
        else "no"
        if num_ctx
        else "unknown"
    )
    triage, _actions = verdict_from_ledgers(exp_dir)

    return {
        "dir": exp_dir.name,
        "completed": aggregate.get("completed", "unknown"),
        "final_phase": aggregate.get("final_phase", "unknown") or "unknown",
        "rounds": aggregate.get("rounds", "unknown"),
        "wall_s": aggregate.get("duration_seconds", "unknown"),
        "repair_dispatches": count_repair_dispatches(decisions),
        "stalled_rounds": stalled,
        "max_turns_deaths": sum(1 for cause in causes if "MaxTurnsError" in cause),
        "output_cap_deaths": sum(1 for cause in causes if "finish_reason=Length" in cause),
        "escalations": sum(1 for row in decisions if row.get("action") == "escalate"),
        "context_length": sum(1 for cause in causes if "context_length_exceeded" in cause),
        "empty_responses": stats["empty_responses"],
        "model_responses": stats["model_responses"],
        "translator_share": stats["translator_output_share"],
        "source_vs_ctx": risk,
        "triage": triage,
    }


SIGNALS = [
    "repair_dispatches",
    "stalled_rounds",
    "max_turns_deaths",
    "output_cap_deaths",
    "escalations",
    "context_length",
    "empty_responses",
]


def rank_key(row):
    """Rank by total measured bottleneck signal count, then wall seconds."""
    total = 0
    for signal in SIGNALS:
        value = row.get(signal)
        total += value if isinstance(value, int) else 0
    wall = row.get("wall_s")
    wall = wall if isinstance(wall, str) and wall.isdigit() else 0
    return (-total, -int(wall))


def hypothesis_seeds(row):
    """One seed line per dominant signal for the ranked round."""
    seeds = []
    if row["repair_dispatches"] >= 10:
        seeds.append(
            f"{row['dir']}: repair loop dominated ({row['repair_dispatches']} repairer dispatches, "
            f"wall {row['wall_s']}s) - cap the repairer budget or tighten diagnosis text"
        )
    if row["output_cap_deaths"] >= 3:
        seeds.append(
            f"{row['dir']}: output-cap deaths {row['output_cap_deaths']} - raise max_output_tokens or "
            f"force compact answers (translator output share {row['translator_share']})"
        )
    if row["context_length"] >= 1:
        seeds.append(
            f"{row['dir']}: {row['context_length']} context_length_exceeded events - snapcompact or "
            f"source-size risk {row['source_vs_ctx']}"
        )
    if row["max_turns_deaths"] >= 3:
        seeds.append(
            f"{row['dir']}: {row['max_turns_deaths']} max-turns deaths - turn budget vs tool-loop check"
        )
    if isinstance(row["empty_responses"], int) and row["empty_responses"] >= 10 and row["model_responses"]:
        seeds.append(
            f"{row['dir']}: {row['empty_responses']}/{row['model_responses']} empty model responses - wasted turns"
        )
    if row["stalled_rounds"] >= 5:
        seeds.append(f"{row['dir']}: {row['stalled_rounds']} stalled rounds - stagnation, not capability")
    return seeds


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".artifacts/experiments", help="experiments root")
    parser.add_argument("--json", action="store_true", help="emit JSON rows instead of a table")
    args = parser.parse_args()

    dirs = sorted(Path(args.root).glob("*v3s*"))
    dirs = [path for path in dirs if (path / "run" / "ledgers").is_dir()]
    rows = [scan_experiment(path) for path in dirs]
    rows.sort(key=rank_key)

    if args.json:
        print(json.dumps(rows, indent=1))
        return

    header = (
        f"{'experiment':42} {'done':5} {'phase':11} {'rnd':3} {'wall':6} {'rep':3} {'stl':3} "
        f"{'mtD':3} {'capD':3} {'esc':3} {'ctx':3} {'empty':>9} {'transl':>7} {'src/ctx':>20} triage"
    )
    print(header)
    print("-" * len(header))
    for row in rows:
        empty = f"{row['empty_responses']}/{row['model_responses']}" if row["model_responses"] else "unknown"
        print(
            f"{row['dir']:42.42} {str(row['completed']):5.5} {row['final_phase']:11.11} "
            f"{str(row['rounds']):3.3} {str(row['wall_s']):6.6} {row['repair_dispatches']:3} "
            f"{row['stalled_rounds']:3} {row['max_turns_deaths']:3} {row['output_cap_deaths']:3} "
            f"{row['escalations']:3} {row['context_length']:3} {empty:>9} {row['translator_share']:>7} "
            f"{row['source_vs_ctx']:>20} {row['triage']}"
        )
    print("\n# hypothesis seeds (ranked rounds first)")
    seen = 0
    for row in rows:
        for seed in hypothesis_seeds(row):
            print(seed)
            seen += 1
        if seen >= 12:
            break
    if not seen:
        print("# none: no round crossed a seed threshold")


if __name__ == "__main__":
    main()
