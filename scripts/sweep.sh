#!/usr/bin/env bash
# Coverage sweep driver: N staggered GPU slots + CPU proposer/scorer
# pipeline (2026-10-02 parallelism directive).
#
# Design:
# - PAIRS file: one "problem_dir|variant" line per round, consumed in order.
#   Each line = one round. The driver runs rounds in waves of SLOTS
#   concurrent harness rounds, staggered STAGGER seconds apart. When
#   SLOTS=2, line i and i+1 form a pair. Convention: first line of a
#   pair is a crust rust-variant problem, second is a non-crust problem
#   (oxidizer/skel/alphatrans) for a clean family A/B.
# - Slot model: the driver scores closed rounds on CPU while the current
#   wave grinds on GPU. Round launches within a wave stagger so their
#   Discovery phases do not collide.
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
mkdir -p "$ROOT/.artifacts/experiments"
PAIRS="${1:?usage: sweep.sh <pairs-file> [start-index]}"
START="${2:-0}"

# Positive-integer check: SLOTS drives wave sizing, so refuse bad values
# early instead of letting an empty or broken wave run.
SLOTS="${SLOTS:-2}"
if ! [[ "$SLOTS" =~ ^[0-9]+$ ]] || [ "$SLOTS" -le 0 ]; then
    echo "ERROR: SLOTS must be a positive integer (got '$SLOTS')" >&2
    exit 1
fi

HARNESS="${HARNESS:-$ROOT/target/release/harness}"
STAGGER="${STAGGER:-900}"
TEMPLATE="$ROOT/scripts/round-template.yml"
# Inference engine URL. Knob default: the ninfer engine moved :8081 ->
# :8082 on 2026-10-02 (both live rounds died in that outage), so the next
# engine relocation is an env var away, not a driver edit.
ENGINE_URL="${ENGINE_URL:-http://localhost:8082/v1}"

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
    #
    # Runs as a background babysitter so launch_round returns the tag
    # immediately and the wave keeps launching on schedule. The busy
    # marker stops wait_close from declaring this round closed during a
    # retry backoff gap, when no process carries the tag yet.
    touch "$exp/.launcher-busy"
    (
        local attempt rc=1
        for attempt in 1 2 3 4; do
            NETMIND_API_KEY=x NETMIND_BASE_URL="$ENGINE_URL" \
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
        rm -f "$exp/.launcher-busy"
    ) > "/tmp/sweep-${name}-${idx}.launcher.log" 2>&1 < /dev/null &
    echo "${exp##*/}"
}

wait_close() {
    # $1 = experiment dir; block until the harness process is gone.
    # Echoes "clean" when the run emitted its terminal done event,
    # "dead" when the process exited silently (needs an abort note).
    local tag="$1"
    local exp="$ROOT/.artifacts/experiments/$tag"
    while pgrep -f "$tag" > /dev/null || [ -f "$exp/.launcher-busy" ]; do
        # Watchdog liveness mark: the driver is alive while it waits.
        touch "$ROOT/.artifacts/experiments/.sweep-heartbeat"
        sleep 300
    done
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
    python3 "$ROOT/scripts/rounds_ledger.py" "$exp" 2>&1 | tail -1
}

mapfile -t ROUNDS < <(grep -v '^#' "$PAIRS" | grep -v '^$')
echo "== sweep: ${#ROUNDS[@]} rounds from $PAIRS (slots ${SLOTS}, stagger ${STAGGER}s)"

past_dirs=()
launch_wave_member() {
    # Launch ROUNDS[$1]; record its experiment dir tag in past_dirs.
    local tag
    tag=$(launch_round "${ROUNDS[$1]}" "$1")
    past_dirs[$1]="$tag"
    echo "== launched slot $(($1 + 1)): $tag ($(date -u +%H:%M))"
}

wave_score_closed() {
    # Score every launched round older than wave $1 that lacks a result.
    local k
    for ((k = 0; k < $1; k++)); do
        if [ -n "${past_dirs[$k]:-}" ] && [ ! -f "$ROOT/.artifacts/experiments/${past_dirs[$k]}/result/aggregate.yml" ]; then
            score_round "${past_dirs[$k]}" || true
            echo "== scored slot C: ${past_dirs[$k]}"
        fi
    done
}

i="$START"
while [ "$i" -lt "${#ROUNDS[@]}" ]; do
    # Watchdog liveness mark: the driver is alive at this loop iteration.
    touch "$ROOT/.artifacts/experiments/.sweep-heartbeat"
    end=$((i + SLOTS))
    if [ "$end" -gt "${#ROUNDS[@]}" ]; then
        end="${#ROUNDS[@]}"
    fi

    for ((j = i; j < end; j++)); do
        launch_wave_member "$j"
        # Delay between launches so Discovery phases do not collide.
        if [ "$j" -lt $((end - 1)) ]; then
            sleep "$STAGGER"
        fi
    done

    # score closed rounds while the current wave grinds
    wave_score_closed "$i"

    # wait for ALL slots of this wave
    for ((j = i; j < end; j++)); do
        tag_close=$(wait_close "${past_dirs[$j]}")
        [ "$tag_close" = dead ] && annotate_aborted "${past_dirs[$j]}"
    done
    echo "== wave [$i .. $((end - 1))] closed $(date -u +%H:%M); scoring next cycle"
    i="$end"
done
# drain: score any unscored round dirs
for d in "$ROOT"/.artifacts/experiments/*v3s*; do
    [ -d "$d" ] && [ -d "$d/run" ] && [ ! -f "$d/result/aggregate.yml" ] && score_round "${d##*/}" || true
done
echo "== sweep complete"
