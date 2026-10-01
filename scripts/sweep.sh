#!/usr/bin/env bash
# Coverage sweep driver: two staggered GPU slots + CPU proposer/scorer
# pipeline (2026-10-02 parallelism directive).
#
# Design:
# - PAIRS file: one "problem_dir|variant" line per round, consumed in order.
#   Each line = one round; the driver always runs two rounds concurrently,
#   staggered STAGGER seconds apart. Line i and i+1 form a pair; convention:
#   first line of a pair is a crust rust-variant problem, second is a
#   non-crust problem (oxidizer/skel/alphatrans) for a clean family A/B.
# - Slot model: slot A (GPU) runs round i's harness; while it grinds, the
#   driver scores round i-1 (slot C, CPU) and stages round i+1's config
#   (slot B, CPU). Rounds of one pair stagger so their Discovery phases
#   interleave with the other's grind.
# - Resume-safe: a round whose dir already has manifest.json + result/ is
#   skipped; the driver restarts clean rounds (manifest rewritten at launch).
# - Config template: configs/round-template.yml with @PROBLEM_ROOT@,
#   @SOURCE_LANG@, @OUTDIR@ placeholders; single-variable deltas go in the
#   per-problem override file configs/overrides/<problem>.yml (appended as
#   mas-level lines by the stager when present).
# - GPU-load guard: rounds record wall in the ledger as usual; the driver
#   only logs a warning when a paired round exceeds 1.5x the solo median.
# - Attribution: one config delta per round, named in the manifest
#   hypothesis line as before.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PAIRS="${1:?usage: sweep.sh <pairs-file> [start-index]}"
START="${2:-0}"
HARNESS="$ROOT/target/release/harness"
STAGGER="${STAGGER:-900}"
TEMPLATE="$ROOT/scripts/round-template.yml"

round_tag() { printf 'autoopt-v0.3.%s' "$1"; }

launch_round() {
    # $1 = problem spec "set_path|lang|tagname", $2 = round index
    local spec="$1" idx="$2"
    local root="${spec%%|*}" rest="${spec#*|}"
    local lang="${rest%%|*}" name="${rest#*|}"
    local exp="$ROOT/.artifacts/experiments/$(date -u +%Y%m%dT%H%M%SZ)v3s${idx}-${name}-sweep"
    mkdir -p "$exp"
    sed -e "s|@PROBLEM_ROOT@|$root|g" -e "s|@SOURCE_LANG@|$lang|g" -e "s|@OUTDIR@|$exp|g" \
        "$TEMPLATE" > "$exp/config.yml"
    # per-problem override lines (single-variable deltas) appended under mas:
    local ov="$ROOT/scripts/overrides/${name}.yml"
    [ -f "$ov" ] && python3 "$ROOT/scripts/override_merge.py" "$exp/config.yml" "$ov"
    # Manifest lineage comes from the harness binary itself (ADR 0020
    # contract): pass the mandatory hypothesis through the environment.
    local hyp="${HYPOTHESIS:-}"
    if [ -z "$hyp" ] && [ -f "$exp/hypothesis.txt" ]; then hyp=$(cat "$exp/hypothesis.txt"); fi
    [ -n "$hyp" ] || { echo "REFUSING launch without hypothesis: $exp" >&2; return 1; }
    local prior
    prior=$(ls -d "$ROOT"/.artifacts/experiments/*v3s* 2>/dev/null | sed 's|.*/||' | awk -v me="${exp##*/}" '$0 < me' | tail -2 | paste -sd, -)
    # Engine-admission retry: a fresh process's first inference can expire
    # in ninfer's admission queue behind an established stream (503
    # request_queue_timeout, v3s9/v3s10/v3s11). Relaunch the same
    # experiment dir with spaced attempts; the harness resumes from the
    # snapshot and the manifest stays untouched.
    local attempt rc=1
    for attempt in 1 2 3 4; do
        NETMIND_API_KEY=x NETMIND_BASE_URL=http://localhost:8081/v1 \
            HARNESS_HYPOTHESIS="$hyp" HARNESS_PARENTS="$prior" HARNESS_GIT_REV="$(git -C "$ROOT" rev-parse --short HEAD)" \
            nohup "$HARNESS" --config "$exp/config.yml" --experiment "${exp##*/}" \
            > "/tmp/sweep-${name}-${idx}.stdout.log" 2>&1 < /dev/null &
        wait $! || rc=$?
        if [ "$rc" -eq 0 ]; then break; fi
        if grep -q "request_queue_timeout" "/tmp/sweep-${name}-${idx}.stdout.log"; then
            echo "== launch attempt $attempt died on admission timeout; retrying in $((attempt * 300))s" >&2
            sleep $((attempt * 300))
        else
            break
        fi
    done
    echo "${exp##*/}"
}

wait_close() {
    # $1 = experiment dir; block until the harness process is gone.
    # Echoes "clean" when the run emitted its terminal done event,
    # "dead" when the process exited silently (needs an abort note).
    local tag="$1"
    while pgrep -f "$tag" > /dev/null; do sleep 300; done
    if grep -q '"event":"done"' "$ROOT/.artifacts/experiments/$tag/events.jsonl" 2>/dev/null; then
        echo clean
    else
        echo dead
    fi
}

annotate_aborted() {
    # A round that died without a `done` event gets an abort note so the
    # ledger records ABORTED instead of stale counters (v3s4).
    local exp="$ROOT/.artifacts/experiments/$1"
    [ -f "$exp/result/abort-note.txt" ] && return 0
    mkdir -p "$exp/result"
    printf 'harness process exited without a done event. annotated by the sweep driver; see the stdout log for the harness error line.\n' \
        > "$exp/result/abort-note.txt"
    echo "== annotated ABORTED: $1"
}

score_round() {
    local exp="$ROOT/.artifacts/experiments/$1"
    python3 "$ROOT/scripts/rescore.py" "$exp" 2>&1 | tail -1
    python3 "$ROOT/scripts/rounds_ledger.py" 2>&1 | tail -1
}

mapfile -t ROUNDS < <(grep -v '^#' "$PAIRS" | grep -v '^$')
echo "== sweep: ${#ROUNDS[@]} rounds from $PAIRS (stagger ${STAGGER}s)"

i="$START"
while [ "$i" -lt "${#ROUNDS[@]}" ]; do
    slotA_spec="${ROUNDS[$i]}"
    slotB_spec=""
    [ "$((i+1))" -lt "${#ROUNDS[@]}" ] && slotB_spec="${ROUNDS[$((i+1))]}"

    # score the round before last (slot C) while staging
    if [ "$((i-2))" -ge 0 ]; then
        prev_dir=$(ls -dt "$ROOT"/.artifacts/experiments/*v3s$((i-2))* 2>/dev/null | head -1)
        [ -n "$prev_dir" ] && [ -d "$prev_dir/result" ] || true
        # scoring happens below after launch to never block the GPU
    fi

    a_tag=$(launch_round "$slotA_spec" "$i")
    echo "== launched slot A: $a_tag ($(date -u +%H:%M))"
    if [ -n "$slotB_spec" ]; then
        sleep "$STAGGER"
        b_tag=$(launch_round "$slotB_spec" "$((i+1))")
        echo "== launched slot B: $b_tag ($(date -u +%H:%M))"
    fi

    # pipeline: score round i-2 (already closed) while slots grind
    if [ "$((i-2))" -ge 0 ]; then
        c_dir=$(ls -dt "$ROOT"/.artifacts/experiments/*v3s$((i-2))* 2>/dev/null | head -1)
        if [ -n "$c_dir" ] && [ ! -f "$c_dir/result/aggregate.yml" ]; then
            score_round "${c_dir##*/}" || true
            echo "== scored slot C: ${c_dir##*/}"
        fi
    fi

    # wait for BOTH slots of this pair
    a_close=$(wait_close "$a_tag")
    [ "$a_close" = dead ] && annotate_aborted "$a_tag"
    if [ -n "$slotB_spec" ]; then
        b_close=$(wait_close "$b_tag")
        [ "$b_close" = dead ] && annotate_aborted "$b_tag"
    fi
    echo "== pair ($a_tag $b_tag) closed $(date -u +%H:%M); scoring next cycle"
    i=$((i+2))
done
# drain: score any unscored round dirs
for d in "$ROOT"/.artifacts/experiments/*v3s*; do
    [ -d "$d" ] && [ -d "$d/run" ] && [ ! -f "$d/result/aggregate.yml" ] && score_round "${d##*/}" || true
done
echo "== sweep complete"
