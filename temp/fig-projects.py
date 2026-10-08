#!/usr/bin/env python3
"""Attempted-projects figure: per-problem maxima over scored sweep rounds.

Data: ROUNDS.yaml. Problem name = text between the last '-' and the trailing
'-sweep' of candidate_id. Problems with zero numeric wall_seconds (in-flight
or never-scored) are excluded and printed to stdout. One horizontal bar per
problem, sorted by max wall minutes descending, log-scaled X from 8 to 900
minutes, colored by best verdict. Right of each bar two aligned text columns
report compile rate and best test pass rate.

Output: temp/attempted-projects.png
"""
from pathlib import Path

import yaml

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

ROOT = Path(__file__).resolve().parent.parent
ROUNDS = ROOT / ".artifacts/experiments/ROUNDS.yaml"
OUT = Path(__file__).resolve().parent / "attempted-projects.png"

INK = "#1F497D"
BLUE = "#4F81BD"
GREEN = "#9BBB59"
RED = "#C0504D"
GRAY = "#999999"

VERDICT_COLORS = {"SOLVED": GREEN, "NOT CLEARED": RED, "ABORTED": GRAY}


def problem_of(cid: str) -> str:
    core = cid[: cid.rfind("-sweep")]
    return core[core.rfind("-") + 1:]


def main():
    data = yaml.safe_load(ROUNDS.read_text())
    rows = data["rounds"]
    sweep_rows = [
        r for r in rows
        if r.get("protocol") != "v2"
        and r.get("round") != "autoopt-v0.2.1"
        and r["candidate_id"].endswith("-sweep")
    ]

    groups = {}
    for r in sweep_rows:
        groups.setdefault(problem_of(r["candidate_id"]), []).append(r)

    skipped = []
    stats = []
    for name, rs in groups.items():
        walls = [
            r["metrics"]["wall_seconds"] for r in rs
            if isinstance(r["metrics"].get("wall_seconds"), (int, float))
        ]
        if not walls:
            skipped.append(name)
            continue
        max_wall_min = max(walls) / 60
        comp = [
            r["metrics"]["compile"] for r in rs
            if r["metrics"].get("compile") in (True, False)
        ]
        compile_rate = (
            sum(1 for c in comp if c) / len(comp) if comp else None
        )
        rates = []
        for r in rs:
            m = r["metrics"]
            if (
                isinstance(m.get("tests_passed"), (int, float))
                and isinstance(m.get("tests_failed"), (int, float))
                and (m["tests_passed"] + m["tests_failed"]) > 0
            ):
                rates.append(m["tests_passed"] / (m["tests_passed"] + m["tests_failed"]))
        best_pass = max(rates) if rates else None
        verdicts = {r["verdict"] for r in rs}
        if "SOLVED" in verdicts:
            verdict = "SOLVED"
        elif "NOT CLEARED" in verdicts:
            verdict = "NOT CLEARED"
        else:
            verdict = "ABORTED"
        stats.append((name, max_wall_min, compile_rate, best_pass, verdict))

    for name in skipped:
        print(f"skipped (zero numeric wall_seconds): {name}")

    stats.sort(key=lambda t: t[1], reverse=True)

    fig, ax = plt.subplots(figsize=(13, 9), dpi=200)
    names = [s[0] for s in stats]
    n = len(stats)
    ax.set_xlim(8, 900)
    ax.set_xscale("log")
    ax.set_ylim(-0.9, n - 0.1)
    bar_h = 0.62

    for i, (name, wall_min, comp_rate, pass_rate, verdict) in enumerate(stats):
        y = n - 1 - i
        ax.barh(y, wall_min, height=0.62, color=VERDICT_COLORS[verdict],
                edgecolor="white", linewidth=0.8, zorder=3)
        # Two aligned text columns right of each bar: on a log axis the
        # column anchor is a fixed fraction of the bar's visual span, so the
        # columns align across bars.
        ax.text(1.06 * wall_min, y + 0.14,
                "-" if comp_rate is None else f"compile {comp_rate * 100:.0f}%",
                ha="left", va="center", fontsize=10, color=INK, zorder=4)
        ax.text(1.06 * wall_min, y - 0.20,
                "-" if pass_rate is None else f"tests {pass_rate * 100:.0f}%",
                ha="left", va="center", fontsize=10, color=INK, zorder=4)

    ax.set_yticks(range(n))
    ax.set_yticklabels(names[::-1], fontsize=11.5, color=INK)
    ax.set_xlabel("max wall time per problem (minutes, log scale)",
                  fontsize=12, color=INK)
    ax.set_title("Attempted projects: max wall time, compile rate, test pass rate",
                 fontsize=17, fontweight="bold", color=INK, pad=14)
    ax.tick_params(axis="x", labelsize=11, colors=INK)
    ax.grid(axis="x", color="#DDDDDD", linewidth=0.7, zorder=0)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    for spine in ("left", "bottom"):
        ax.spines[spine].set_color(GRAY)

    handles = [
        plt.Rectangle((0, 0), 1, 1, facecolor=GREEN, edgecolor="white"),
        plt.Rectangle((0, 0), 1, 1, facecolor=RED, edgecolor="white"),
        plt.Rectangle((0, 0), 1, 1, facecolor=GRAY, edgecolor="white"),
    ]
    ax.legend(handles, ["SOLVED", "NOT CLEARED", "ABORTED"],
              loc="lower right", fontsize=11.5, frameon=False,
              labelcolor=INK, title="verdict (best over rounds)",
              title_fontsize=11.5)

    fig.text(0.995, 0.008,
             f"per-problem maxima over scored rounds; ROUNDS.yaml, n={len(sweep_rows)} rows",
             ha="right", va="bottom", fontsize=10, color=GRAY)

    fig.tight_layout(rect=(0, 0.03, 1, 1))
    fig.savefig(OUT, dpi=200, bbox_inches="tight", facecolor="white")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
