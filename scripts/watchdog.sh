#!/usr/bin/env bash
# Stall watchdog for the GPU coverage sweep.
#
# Kills harness rounds with no file progress for STALL_AGE and relaunches
# the sweep driver when it dies silently. Never touches a round younger
# than STALL_AGE (manifest mtime grace) and never kills by a short name
# prefix: the pgrep pattern is always the full experiment dir name.
#
# Usage: watchdog.sh <pairs-file>
# Production default root is the repo above this script. Set
# WATCHDOG_ROOT for a sandbox test.
#
# Stall classes covered (2026-10-02 campaign post-mortem):
# 1. Silent harness death without a done event -> wait_close in the
#    driver already writes the abort note. Nothing to do here.
# 2. Live-but-stuck harness (zero progress > STALL_AGE, v3s20/v3s21) ->
#    kill + abort note, so the driver can close the round.
# 3. Driver death (nohup sweep.sh gone, "everything stops forever") ->
#    relaunch the driver at the smallest unfinished round index.
set -u

# Tunables: override via environment. No inline magic numbers.
CHECK_INTERVAL="${CHECK_INTERVAL:-300}"
STALL_AGE="${STALL_AGE:-2700}"
DRIVER_STALE="${DRIVER_STALE:-900}"

ROOT="${WATCHDOG_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
PAIRS="${1:?usage: watchdog.sh <pairs-file>}"
EXPS="$ROOT/.artifacts/experiments"
HEARTBEAT="$EXPS/.sweep-heartbeat"
WATCH_HEARTBEAT="$EXPS/.watchdog-heartbeat"
LOG="$EXPS/watchdog.log"

log() { printf '%s %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*" >> "$LOG"; }

# Newest mtime across a round's progress files. Empty when none exist.
last_progress() {
    local exp="$1" newest=0 f m
    for f in "$exp"/events.jsonl "$exp"/traces/turns.jsonl \
             "$exp"/run/ledgers/*.jsonl "$exp"/run/state.json; do
        [ -f "$f" ] || continue
        m=$(stat -c %Y "$f") || continue
        [ "$m" -gt "$newest" ] && newest=$m
    done
    [ "$newest" -gt 0 ] && echo "$newest"
}

# "unfinished" when the round dir lacks aggregate.yml and abort-note.txt.
round_unfinished() {
    local exp="$1"
    [ -f "$exp/result/aggregate.yml" ] && return 1
    [ -f "$exp/result/abort-note.txt" ] && return 1
    # per_problem.json means the round reached the scoring stage. The
    # scorer stopped early on it (VERIFIED-WORKSPACE rows in SUMMARY.md,
    # v3s8/v3s17). Do not treat a scored round as stalled.
    [ -f "$exp/result/per_problem.json" ] && return 1
    return 0
}

# Smallest pair index whose round dir is missing or unfinished.
smallest_unfinished_idx() {
    local idx=0 line name
    while IFS= read -r line; do
        case "$line" in '#'*) continue ;; esac
        [ -n "$line" ] || continue
        name="${line##*|}"
        local d
        d=$(ls -dt "$EXPS"/*v3s${idx}-* 2>/dev/null | head -1)
        if [ -z "$d" ] || round_unfinished "$d"; then
            echo "$idx"
            return 0
        fi
        idx=$((idx + 1))
    done < <(grep -v '^$' "$PAIRS")
    return 1
}

# True when the watchdog may and should kill this round now.
stalled_round() {
    local exp="$1" tag="$2" now lp
    [ -f "$exp/manifest.json" ] || return 1
    round_unfinished "$exp" || return 1
    # A done event means the round closed and only awaits scoring.
    # Never judge a closed round as stalled.
    if [ -f "$exp/events.jsonl" ] && grep -q '"event":"done"' "$exp/events.jsonl" 2>/dev/null; then
        return 1
    fi
    # Grace: never judge a round younger than STALL_AGE.
    now=$(date +%s)
    [ $((now - $(stat -c %Y "$exp/manifest.json"))) -lt "$STALL_AGE" ] && return 1
    lp=$(last_progress "$exp") || return 1
    [ -z "$lp" ] && return 1
    [ $((now - lp)) -gt "$STALL_AGE" ] || return 1
    LAST_PROGRESS_AGE=$((now - lp))
    return 0
}

check_stalls() {
    local exp tag
    for exp in "$EXPS"/*v3s*; do
        [ -d "$exp" ] || continue
        tag="${exp##*/}"
        if stalled_round "$exp" "$tag"; then
            if pgrep -f "$tag" > /dev/null; then
                pkill -f "$tag"
                # Give the harness up to 10s to exit before logging.
                local waited=0
                while pgrep -f "$tag" > /dev/null && [ "$waited" -lt 10 ]; do
                    sleep 1
                    waited=$((waited + 1))
                done
                mkdir -p "$exp/result"
                printf 'killed by watchdog: no file progress for %ss (stall age %ss); last progress age %ss\n' \
                    "$((LAST_PROGRESS_AGE))" "$STALL_AGE" "$LAST_PROGRESS_AGE" \
                    > "$exp/result/abort-note.txt"
                log "killed stalled harness $tag: no progress for ${LAST_PROGRESS_AGE}s; wrote result/abort-note.txt"
            else
                # Dead but unannotated: write the note so the ledger
                # records ABORTED instead of stale counters.
                mkdir -p "$exp/result"
                printf 'harness process gone without a done event; no file progress for %ss (stall age %ss)\n' \
                    "$LAST_PROGRESS_AGE" "$STALL_AGE" \
                    > "$exp/result/abort-note.txt"
                log "annotated dead round $tag: no progress for ${LAST_PROGRESS_AGE}s; wrote result/abort-note.txt"
            fi
        fi
    done
}

# True when a harness process for this watchdog root is alive. Match the
# root path in the command line, not a bare round tag: a production
# harness must not block a sandbox test (and vice versa).
harness_alive() {
    pgrep -f "$EXPS" > /dev/null
}

check_driver() {
    # Fresh driver heartbeat: nothing to do.
    [ -f "$HEARTBEAT" ] && [ $(( $(date +%s) - $(stat -c %Y "$HEARTBEAT") )) -lt "$DRIVER_STALE" ] && return 0
    # Any live harness process means the driver may be between rounds.
    # Never relaunch under it.
    harness_alive && return 0
    local idx
    idx=$(smallest_unfinished_idx) || return 0
    log "driver dead (heartbeat stale > ${DRIVER_STALE}s) and no harness alive; relaunching sweep.sh at index $idx"
    nohup bash "$ROOT/scripts/sweep.sh" "$PAIRS" "$idx" \
        > "/tmp/sweep-driver-$(date -u +%H%M%S).log" 2>&1 &
    log "relaunched sweep.sh at index $idx (pid $!)"
}

while true; do
    touch "$WATCH_HEARTBEAT"
    check_stalls
    check_driver
    sleep "$CHECK_INTERVAL"
done
