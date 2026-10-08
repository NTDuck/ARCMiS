#!/usr/bin/env python3
"""Phase state machine figure: compact two-lane horizontal slide graphic.

Lane 1: ten small rounded boxes, one per methodology phase (state.rs Phase
enum), forward arrows in ink #1F497D, Done emphasized. Lane 2: dashed curved
regression arcs in green #9BBB59 dipping under lane 1 without touching its
text, labeled with the five observed regression classes. Caption bottom
right.

Output: temp/state-machine.png
"""
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch

OUT = Path(__file__).resolve().parent / "state-machine.png"

INK = "#1F497D"
BLUE = "#4F81BD"
GREEN = "#9BBB59"


def main():
    fig, ax = plt.subplots(figsize=(14, 2.8), dpi=200)
    ax.set_xlim(0, 14)
    ax.set_ylim(0, 2.8)
    ax.axis("off")

    phases = ["Preflight", "Discovery", "Contract", "Planning", "Pilot",
              "Migration", "Integration", "Hardening", "FinalValidation",
              "Done"]
    n = len(phases)
    box_w, box_h = 1.16, 0.62
    lane_y = 2.08
    x0, step = 0.62, 1.345
    centers = [x0 + i * step for i in range(n)]

    for i, (name, cx) in enumerate(zip(phases, centers)):
        emphasized = name == "Done"
        ax.add_patch(FancyBboxPatch(
            (cx - box_w / 2, lane_y - box_h / 2), box_w, box_h,
            boxstyle="round,pad=0.02,rounding_size=0.09",
            linewidth=2.1 if emphasized else 1.4,
            edgecolor=INK, facecolor=INK if emphasized else "white",
            zorder=3,
        ))
        ax.text(cx, lane_y, name, ha="center", va="center",
                fontsize=9.8, fontweight="bold",
                color="white" if emphasized else INK, zorder=4)
        if i < n - 1:
            ax.add_patch(FancyArrowPatch(
                (cx + box_w / 2 + 0.015, lane_y),
                (centers[i + 1] - box_w / 2 - 0.015, lane_y),
                arrowstyle="-|>", mutation_scale=11,
                linewidth=1.5, color=INK, zorder=2,
                shrinkA=0, shrinkB=0,
            ))

    # Regression arcs dipping below lane 1 (lane 2), never touching box text.
    # Labels sit on two fixed rows so they never collide with each other.
    regressions = [
        (4, 1, "Pilot to Discovery", 0),
        (5, 1, "Migration to Discovery", 1),
        (6, 5, "Integration to Migration", 0),
        (7, 6, "Hardening to Integration", 1),
        (8, 6, "FinalValidation to Integration", 2),
    ]
    for src, dst, label, slot in regressions:
        xs, xd = centers[src], centers[dst]
        ax.add_patch(FancyArrowPatch(
            (xs, lane_y - box_h / 2 - 0.04),
            (xd, lane_y - box_h / 2 - 0.04),
            arrowstyle="-|>", mutation_scale=11,
            linewidth=1.5, color=GREEN, linestyle="--",
            connectionstyle=f"arc3,rad={-0.42 - 0.05 * abs(src - dst)}",
            zorder=2, shrinkA=0, shrinkB=0,
        ))
        mid = (xs + xd) / 2
        label_y = 0.62 if slot == 0 else 0.42
        label_x = mid
        if label == "Hardening to Integration":
            label_x, label_y = 10.4, 0.55
        elif label == "FinalValidation to Integration":
            label_x, label_y = 13.3, 0.62
        ax.text(label_x, label_y, label, ha="center", va="top",
                fontsize=7.6, color=GREEN, style="italic", zorder=3)

    # Caption bottom right, below the label rows.
    ax.text(13.85, 0.02,
            "forward = phase exit condition earned; regressions on failure",
            ha="right", va="bottom", fontsize=9.5, color=INK, style="italic")

    fig.savefig(OUT, dpi=200, bbox_inches="tight", facecolor="white")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
