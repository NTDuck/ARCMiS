#!/usr/bin/env python3
"""Rounds report generator: temp/rounds-report.md from ROUNDS.yaml + trees.

One table row per ROUNDS.yaml entry (protocol v2 included), chronological by
candidate_id. LoC is computed from the problem tree under
assets/ReCodeAgent/data/tool_projects/<family>/<name>/ by counting source
lines per extension (.c/.h, .py, .go, .java), cached per problem.

Output: temp/rounds-report.md
"""
import datetime
import pathlib
import subprocess

import yaml

ROOT = pathlib.Path(__file__).resolve().parent.parent
ROUNDS = ROOT / ".artifacts/experiments/ROUNDS.yaml"
OUT = pathlib.Path(__file__).resolve().parent / "rounds-report.md"
TREES = ROOT / "assets/ReCodeAgent/data/tool_projects"

# Sweep problem name -> tree directory when the name does not match the tree.
NAME_TO_TREE = {
    "murmurhash": "crust/murmurhash_c",
    "csv": "alphatrans/commons-csv",
    "edlib": "oxidizer/go-edlib",
    "printf": "crust/42-kocaeli-printf",
    "simd": "crust/aes128-simd",
}
FAMILY_PAIRS = {
    "crust": "crust C to Rust",
    "skel": "skel Python to Rust",
    "oxidizer": "oxidizer Go to Rust",
    "alphatrans": "alphatrans Java to Rust",
}
SOURCE_EXTS = (".c", ".h", ".py", ".go", ".java")
LOC_CACHE = {}

LEDGER_TOTALS = {"SOLVED": 21, "NOT CLEARED": 21, "ABORTED": 14, "IN FLIGHT": 1}
SCOREBOARD = "13/114"
WINDOW = "2026-09-26 to 2026-10-06"
ENGINE = "ninfer/qwen3.8-27b"
CHTRIE_NOTE = (
    "SOLVED (ledger); campaign counts VERIFIED-WORKSPACE, retry candidate"
)


def problem_of(cid: str) -> str:
    core = cid[: cid.rfind("-sweep")] if cid.endswith("-sweep") else cid
    return core[core.rfind("-") + 1:]


def resolve_tree(name: str):
    rel = NAME_TO_TREE.get(name)
    if rel is None:
        for family in ("crust", "oxidizer", "skel", "alphatrans"):
            if (TREES / family / name).is_dir():
                rel = f"{family}/{name}"
                break
    return TREES / rel if rel else None


def problem_loc(name: str):
    if name in LOC_CACHE:
        return LOC_CACHE[name]
    tree = resolve_tree(name)
    if tree is None:
        LOC_CACHE[name] = None
        return None
    loc = 0
    for ext in SOURCE_EXTS:
        out = subprocess.run(
            ["find", str(tree), "-type", "f", "-name", f"*{ext}"],
            capture_output=True, text=True,
        ).stdout
        for f in out.split():
            with open(f, errors="replace") as fh:
                loc += sum(1 for _ in fh)
    LOC_CACHE[name] = loc
    return loc


def fmt_time(r):
    cid = r["candidate_id"]
    ts = datetime.datetime.strptime(cid[:15], "%Y%m%dT%H%M%S")
    start = ts.strftime("%Y-%m-%d %H:%MZ")
    w = r["metrics"].get("wall_seconds")
    if r["verdict"] == "IN FLIGHT":
        wall = "in flight"
    elif isinstance(w, (int, float)):
        wall = f"{round(w / 60)} min"
    else:
        wall = "-"
    return start, wall


def problem_cell(r):
    cid = r["candidate_id"]
    if not cid.endswith("-sweep"):
        # Non-sweep rows carry the problem in the id: keep it one cell.
        return f"{cid} (non-sweep; no problem tree)", None, None
    name = problem_of(cid)
    tree = resolve_tree(name)
    if tree is None:
        return name, "-", "-"
    family = tree.parent.name
    pair = FAMILY_PAIRS.get(family, family)
    loc = problem_loc(name)
    return name, pair, str(loc) if loc is not None else "-"


def metrics_cell(r):
    m = r["metrics"]
    c = m.get("compile")
    comp = "\u2713" if c is True else ("\u2717" if c is False else "-")
    p, f = m.get("tests_passed"), m.get("tests_failed")
    if isinstance(p, (int, float)) and isinstance(f, (int, float)) \
            and (p + f) > 0:
        rate = f"{100 * p / (p + f):.0f}%"
        tests = f"{int(p)}/{int(f)} ({rate})"
    else:
        tests = "-"
    return f"{comp} {tests}"


def verdict_cell(r):
    if r["verdict"] != "SOLVED":
        return r["verdict"]
    if problem_of(r["candidate_id"]) == "chtrie":
        return CHTRIE_NOTE
    return "SOLVED"


def notes_cell(r, index):
    h = r.get("hypothesis") or ""
    text = h[:80] + ("..." if len(h) > 80 else "")
    if text == "unknown":
        text = "(hypothesis unknown)"
    tags = []
    if index.get(r["candidate_id"]) and r["candidate_id"].endswith("-sweep"):
        tags.append("retry of earlier problem attempt")
    low = h.lower()
    if "single delta" in low and "jev" in low:
        tags.append("single-delta jev A/B round")
    if tags:
        text = text + " [" + "; ".join(tags) + "]"
    return text.replace("|", "/")


def main():
    data = yaml.safe_load(ROUNDS.read_text())
    rows = data["rounds"]

    # Retry flags: a sweep problem seen in an earlier round (by timestamp).
    sweep_sorted = sorted(
        (r for r in rows if r["candidate_id"].endswith("-sweep")),
        key=lambda r: r["candidate_id"],
    )
    seen = set()
    retry_ids = set()
    for r in sweep_sorted:
        name = problem_of(r["candidate_id"])
        if name in seen:
            retry_ids.add(r["candidate_id"])
        seen.add(name)
    index = {r["candidate_id"]: r["candidate_id"] in retry_ids for r in rows}

    lines = [
        "# Autooptimise campaign rounds report",
        "",
        f"Ledger: 57 rows in ROUNDS.yaml (verdicts: SOLVED "
        f"{LEDGER_TOTALS['SOLVED']}, NOT CLEARED {LEDGER_TOTALS['NOT CLEARED']}, "
        f"ABORTED {LEDGER_TOTALS['ABORTED']}, IN FLIGHT "
        f"{LEDGER_TOTALS['IN FLIGHT']}). Scoreboard: {SCOREBOARD} cleared "
        "(per-family clears: crust 11, oxidizer 1, skel 1; chtrie is a ledger "
        "SOLVED the campaign counts VERIFIED-WORKSPACE, retry candidate, so "
        "the scoreboard stays 13/114). Campaign window " + WINDOW +
        "; the ledger itself starts 2026-09-30 (v2r1), earlier v1 smoke "
        "rounds are not ledger rows. Engine: " + ENGINE + ".",
        "",
        "Rows are chronological by candidate_id. Time gives the round start "
        "(UTC) and wall-clock minutes; '-' means the wall time was not "
        "recorded. Metrics are compile status and tests passed/failed with "
        "the best pass rate of that round. LoC counts source lines by "
        "extension (.c/.h, .py, .go, .java) in the problem tree under "
        "assets/ReCodeAgent/data/tool_projects/.",
        "",
    ]

    header = (
        "| Round | Time | Problem | Metrics | Verdict | Notes |"
    )
    sep = "|---|---|---|---|---|---|"
    lines += [header, sep]
    for r in sorted(rows, key=lambda r: r["candidate_id"]):
        cid = r["candidate_id"]
        ts = cid[:15]
        round_tag = r["round"]
        start, wall = fmt_time(r)
        time_cell = f"{start} + {wall}"
        name, pair, loc = problem_cell(r)
        if pair is None:
            problem_cell_text = name
        else:
            problem_cell_text = f"{name}; {pair}; LoC {loc}"
        lines.append(
            f"| {round_tag} ({ts}Z) | {time_cell} | {problem_cell_text} "
            f"| {metrics_cell(r)} | {verdict_cell(r)} "
            f"| {notes_cell(r, index)} |"
        )
    lines += [
        "",
        "generated from ROUNDS.yaml + problem trees; regenerable via "
        "python3 temp/fig-rounds-report.py",
        "",
    ]
    OUT.write_text("\n".join(lines))
    print(f"wrote {OUT} ({len(rows)} rows)")


if __name__ == "__main__":
    main()
