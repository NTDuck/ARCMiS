#!/usr/bin/env python3
"""MAS agent hierarchy figure (ADR 0026): three static tiers plus advisory.

Tier 1: orchestrator on top. Tier 2: the three tool-free leads, each drawn
with its turn budget. Tier 3: nine specialists, one rounded box per agent,
exactly one incoming lead edge each. fleet-analyst sits to the side,
reachable only through a dashed advisory edge from the orchestrator. Inside
the migration team the translate/validate/test loop is annotated with the
repairer re-entry, labeled "translation collective".

Palette: title/text #1F497D, blue #4F81BD, green #9BBB59, gray #999999
(red #C0504D reserved for negative results, unused here). White background,
200 DPI.

Output: temp/agent-hierarchy.png
"""
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch

OUT = Path(__file__).resolve().parent / "agent-hierarchy.png"

INK = "#1F497D"
BLUE = "#4F81BD"
GREEN = "#9BBB59"
GRAY = "#999999"


def rounded_box(ax, x, y, w, h, title, sub="", edge=BLUE, title_size=15,
                sub_size=11.5, dashed=False, lw=1.8):
    """One agent = one rounded box, centered on (x, y)."""
    ax.add_patch(FancyBboxPatch(
        (x - w / 2, y - h / 2), w, h,
        boxstyle="round,pad=0.02,rounding_size=0.10",
        linewidth=lw, edgecolor=edge, facecolor="white",
        linestyle="--" if dashed else "-", zorder=3,
    ))
    if sub:
        ax.text(x, y + h * 0.21, title, ha="center", va="center",
                fontsize=title_size, fontweight="bold", color=INK, zorder=4)
        ax.text(x, y - h * 0.24, sub, ha="center", va="center",
                fontsize=sub_size, color=INK, zorder=4, linespacing=1.3)
    else:
        ax.text(x, y, title, ha="center", va="center",
                fontsize=title_size, fontweight="bold", color=INK, zorder=4)


def arrow(ax, x0, y0, x1, y1, color=BLUE, dashed=False, lw=1.6, rad=0.0):
    ax.add_patch(FancyArrowPatch(
        (x0, y0), (x1, y1),
        arrowstyle="-|>", mutation_scale=15,
        linewidth=lw, color=color, linestyle="--" if dashed else "-",
        connectionstyle=f"arc3,rad={rad}", zorder=2,
        shrinkA=3, shrinkB=3,
    ))


def main():
    fig, ax = plt.subplots(figsize=(16, 9.4), dpi=200)
    ax.set_xlim(0, 16.5)
    ax.set_ylim(0, 9.4)
    ax.axis("off")

    ax.text(8.0, 8.60, "MAS agent hierarchy (ADR 0026): three static tiers",
            ha="center", va="center", fontsize=21, fontweight="bold", color=INK)

    # Tier band labels on the left edge
    for y, name in ((7.40, "TIER 1"), (5.50, "TIER 2"), (2.75, "TIER 3")):
        ax.text(0.08, y, name, ha="left", va="center", fontsize=11,
                fontweight="bold", color=GRAY)

    # Tier 1: orchestrator
    rounded_box(ax, 6.3, 7.40, 4.5, 1.15, "orchestrator",
                "owns the plan, delegates via the task graph; fanout 3",
                edge=INK, lw=2.2, title_size=16, sub_size=11)

    # Advisory fleet-analyst: dashed edge straight from orchestrator
    rounded_box(ax, 14.0, 7.40, 2.55, 1.15, "fleet-analyst",
                "recommends model\npromotion/demotion",
                edge=GRAY, dashed=True, title_size=13, sub_size=10)
    arrow(ax, 8.55, 7.40, 12.70, 7.40, color=GRAY, dashed=True, lw=1.6)
    ax.text(10.62, 7.71, "advisory, tier 1", ha="center", va="center",
            fontsize=10.5, color=GRAY, style="italic")

    # Tier 2: tool-free leads, DECISION text with turn budgets
    LEAD_Y = 5.50
    leads = {
        "migration-lead": 2.9,
        "discovery-lead": 7.6,
        "integration-lead": 12.4,
    }
    for name, x in leads.items():
        turns = 16 if name == "migration-lead" else 10
        rounded_box(ax, x, LEAD_Y, 2.9, 1.05, name,
                    f"DECISION: delegate MEMBER, turns {turns}",
                    edge=BLUE, title_size=13.5, sub_size=10.5)
        arrow(ax, 6.3, 6.82, x, LEAD_Y + 0.55, color=INK, lw=1.8)

    # Tier 3: one box per specialist, one incoming lead edge each. Two
    # staggered rows keep every box and description readable.
    TOP_Y = 3.45
    BOT_Y = 2.05
    BOX_H = 1.15
    # Row A: validator, architect, planner, critic (4 boxes)
    row_a = [
        (3.4, "validator", "judges one batch;\nread-only"),
        (7.3, "architect", "writes the\nmigration brief"),
        (11.6, "planner", "produces\ndependency batches"),
        (15.0, "critic", "end-of-run adversarial\nreview; read-only"),
    ]
    # Row B: translator, tester, repairer, analyst, failure-analyst (5 boxes)
    row_b = [
        (1.25, "translator", "writes target modules\nfor one batch"),
        (4.30, "tester", "adds tests; runs\nthe test command"),
        (6.75, "repairer", "fixes one diagnosed\nfailure"),
        (9.15, "analyst", "reads the codebase;\nwrites the source map"),
        (13.3, "failure-analyst", "classifies\none failure"),
    ]
    for x, name, desc in row_a:
        rounded_box(ax, x, TOP_Y, 2.1, BOX_H, name, desc,
                    edge=BLUE, title_size=12, sub_size=9.3)
    for x, name, desc in row_b:
        rounded_box(ax, x, BOT_Y, 2.3, BOX_H, name, desc,
                    edge=BLUE, title_size=12, sub_size=9.3)

    # One incoming lead edge per specialist
    lead_of = {
        "translator": "migration-lead", "validator": "migration-lead",
        "tester": "migration-lead", "repairer": "migration-lead",
        "analyst": "discovery-lead", "architect": "discovery-lead",
        "planner": "integration-lead", "failure-analyst": "integration-lead",
        "critic": "integration-lead",
    }
    for x, name, _ in row_a:
        lx = leads[lead_of[name]]
        arrow(ax, lx + 0.9, LEAD_Y - 0.55, x, TOP_Y + BOX_H / 2,
              color=BLUE, lw=1.5, rad=0.08)
    for x, name, _ in row_b:
        lx = leads[lead_of[name]]
        arrow(ax, lx, LEAD_Y - 0.55, x, BOT_Y + BOX_H / 2,
              color=BLUE, lw=1.5, rad=0.06)

    # Translation collective: translator -> validator -> tester with the
    # repairer re-entry. All four sit in the migration team columns.
    def center(name):
        for x, n, _ in row_a + row_b:
            if n == name:
                return x
        raise KeyError(name)

    t_x, v_x = center("translator"), center("validator")
    s_x, r_x = center("tester"), center("repairer")
    t_bot, v_bot = BOT_Y - BOX_H / 2, TOP_Y - BOX_H / 2
    # translator -> validator (up to row A), validator -> tester (down), and
    # tester -> repairer forward along row B, repairer re-entry back up.
    arrow(ax, t_x + 0.3, BOT_Y + BOX_H / 2, v_x - 0.3, v_bot,
          color=GREEN, lw=2.0, rad=0.20)
    arrow(ax, v_x + 0.3, v_bot, s_x - 0.3, BOT_Y + BOX_H / 2,
          color=GREEN, lw=2.0, rad=-0.15)
    arrow(ax, s_x + 1.15, BOT_Y + BOX_H / 2 + 0.04, r_x - 1.15,
          BOT_Y + BOX_H / 2 + 0.04, color=GREEN, lw=2.0, rad=-0.30)
    ax.text(2.85, 0.90, "translation collective:\n"
            "translator \u2192 validator \u2192 tester, repairer re-entry",
            ha="center", va="center", fontsize=12, color=GREEN,
            fontweight="bold", linespacing=1.4)

    # Legend (bottom right, clear of the collective caption on the left)
    y = 0.32
    ax.plot([9.35, 9.85], [y, y], color=INK, lw=1.8)
    ax.text(9.95, y, "tier-1 delegation", fontsize=10.5, color=INK, va="center")
    ax.plot([12.75, 13.25], [y, y], color=BLUE, lw=1.8)
    ax.text(13.35, y, "lead \u2192 member", fontsize=10.5, color=INK, va="center")
    ax.plot([9.35, 9.85], [y - 0.34, y - 0.34], color=GREEN, lw=2.0)
    ax.text(9.95, y - 0.34, "collective loop", fontsize=10.5, color=INK, va="center")
    ax.plot([12.75, 13.25], [y - 0.34, y - 0.34], color=GRAY, lw=1.6, linestyle="--")
    ax.text(13.35, y - 0.34, "advisory (no team)", fontsize=10.5, color=INK, va="center")

    fig.savefig(OUT, dpi=200, bbox_inches="tight", pad_inches=0.25,
                facecolor="white")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
