#!/usr/bin/env bash
# ARCMiS benchmark driver over the ReCodeAgent tool_projects dataset.
#
# One cell = (tool, project, source_lang -> target_lang). For each cell the
# driver writes a per-run config from assets/configs/bench/template.yml
# (generous ceilings: failure must come from genuine completion, never from
# budget starvation), runs the harness binary under a wall-clock timeout,
# scores the produced workspace, and persists into:
#
#   benchmarks/{commit}/{tool}/{project}/{src}2{dst}/
#     result.yml            scoring record (score-run.sh format)
#     events.jsonl          harness event log
#     traces/turns.jsonl    per-model-call trace
#     logs/                 harness stdout/stderr + score logs
#
# The raw run (workspace, run/ blackboard, config) stays under
# .artifacts/bench/{tool}/{project}/{src}2{dst}/ (gitignored) so scoring can
# be re-run offline with bench/score-run.sh --rescore.
#
# Resumable: a cell whose result.yml already exists is skipped unless
# --force. Interrupted runs just restart the driver; completed cells skip.
#
# Usage: bench/run-benchmarks.sh [--tool NAME]... [--project NAME]...
#        [--limit N] [--force] [--timeout SECS] [--commit HASH] [--dry-run]
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

TOOLS=()
PROJECTS=()
LIMIT=0
FORCE=0
TIMEOUT="${BENCH_TIMEOUT:-3600}"
COMMIT_PIN="${BENCH_COMMIT:-}"
while [ $# -gt 0 ]; do
    case "$1" in
        --tool) TOOLS+=("$2"); shift 2 ;;
        --project) PROJECTS+=("$2"); shift 2 ;;
        --limit) LIMIT="$2"; shift 2 ;;
        --force) FORCE=1; shift ;;
        --timeout) TIMEOUT="$2"; shift 2 ;;
        --commit) COMMIT_PIN="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        *) echo "unknown arg $1; expected --tool/--project/--limit/--force/--timeout/--commit/--dry-run" >&2; exit 1 ;;
    esac
done

# Commit dir for results. --commit pins it; default is HEAD. Pin when
# restarting mid-sweep after result commits moved HEAD, so the driver
# resumes the existing cell set instead of starting a fresh dir.
if [ -z "$COMMIT_PIN" ]; then
    COMMIT=$(git rev-parse --short=12 HEAD)
else
    COMMIT="$COMMIT_PIN"
fi
BENCH_ROOT="benchmarks/$COMMIT"
DATASET="assets/ReCodeAgent/data/tool_projects"
TEMPLATE="assets/configs/bench/template.yml"
RAW_ROOT=".artifacts/bench"
mkdir -p "$BENCH_ROOT"

# Source->target pair per (tool, project, dir) from the dataset layout. One
# cell per project: the src-language dir is the run's source root; the
# sibling dst-language dir is ReCodeAgent's reference, not our input.
emit_cells() {
    for tool_dir in "$DATASET"/*/; do
        tool=$(basename "$tool_dir")
        if [ ${#TOOLS[@]} -gt 0 ] && ! printf '%s\n' "${TOOLS[@]}" | grep -qx "$tool"; then
            continue
        fi
        for proj_dir in "$tool_dir"*/; do
            proj=$(basename "$proj_dir")
            if [ ${#PROJECTS[@]} -gt 0 ] && ! printf '%s\n' "${PROJECTS[@]}" | grep -qx "$proj"; then
                continue
            fi
            for lang_dir in "$proj_dir"*/; do
                lang=$(basename "$lang_dir")
                case "$tool:$lang" in
                    crust:c) src=c; dst=rust ;;
                    crust:rust) continue ;; # reference dir; the c->rust cell covers it
                    oxidizer:go) src=go; dst=rust ;;
                    oxidizer:rust) continue ;;
                    alphatrans:java) src=java; dst=python ;;
                    alphatrans:python) continue ;;
                    skel:python) src=javascript; dst=python ;;
                    skel:javascript) src=python; dst=javascript ;;
                    *) continue ;;
                esac
                echo "$tool $proj $src $dst"
            done
        done
    done
}

if [ "${DRY_RUN:-0}" = 1 ]; then
    emit_cells
    exit 0
fi

run_cell() {
    tool="$1"; proj="$2"; src="$3"; dst="$4"
    cell_dir="$BENCH_ROOT/$tool/$proj/${src}2${dst}"
    result_yml="$cell_dir/result.yml"
    if [ -f "$result_yml" ] && [ "$FORCE" = 0 ]; then
        echo "SKIP $tool/$proj/${src}2${dst} (already scored)"
        return 0
    fi

    # Raw run location; wiped so a rerun cannot inherit a populated workspace.
    output_dir="$RAW_ROOT/$tool/$proj/${src}2${dst}"
    rm -rf "$output_dir"
    mkdir -p "$output_dir"
    source_root="$DATASET/$tool/$proj/$src"
    config_path="$output_dir/config.yml"
    sed -e "s|__OUTPUT_DIR__|$output_dir|g" \
        -e "s|__SOURCE_ROOT__|$source_root|g" \
        -e "s|__TEST_COMMAND__|__|g" \
        -e "s|__SRC_LANG__|$src|g" \
        -e "s|__DST_LANG__|$dst|g" \
        "$TEMPLATE" > "$config_path"

    echo "RUN $tool/$proj $src->$dst (timeout ${TIMEOUT}s)"
    started=$(date +%s)
    timeout "$TIMEOUT" cargo run --release -p harness -- --config "$config_path" \
        > "$output_dir/harness-stdout.log" 2> "$output_dir/harness-stderr.log" || true
    ended=$(date +%s)
    duration=$((ended - started))

    # Persist the committed cell: scoring record + the harness's own
    # artifacts, never the migrated source trees (score-run.sh --rescore
    # reads the workspace back from $output_dir).
    mkdir -p "$cell_dir/logs"
    cp "$config_path" "$cell_dir/logs/config.yml"
    cp "$output_dir/harness-stdout.log" "$output_dir/harness-stderr.log" "$cell_dir/logs/"
    for f in events.jsonl manifest.json state.json tasks.json; do
        [ -f "$output_dir/run/$f" ] && cp "$output_dir/run/$f" "$cell_dir/"
    done
    [ -f "$output_dir/events.jsonl" ] && cp "$output_dir/events.jsonl" "$cell_dir/"
    if [ -f "$output_dir/traces/turns.jsonl" ]; then
        mkdir -p "$cell_dir/traces"
        cp "$output_dir/traces/turns.jsonl" "$cell_dir/traces/"
    fi
    if [ -f "$output_dir/result/aggregate.yml" ]; then
        mkdir -p "$cell_dir/result"
        cp "$output_dir/result/aggregate.yml" "$cell_dir/result/"
    fi

    # Score: separate helper so it can be re-run offline.
    BENCH_SCORE_ROOT="$output_dir" bash "$(dirname "$0")/score-run.sh" \
        --tool "$tool" --src-lang "$src" --dst-lang "$dst" --logs "$cell_dir/logs" \
        --out "$result_yml"
    # Enrich the scored record with cell identity and run metadata.
    python3 - "$result_yml" "$cell_dir" "$tool" "$proj" "$src" "$dst" "$COMMIT" "$output_dir" "$duration" <<'PYEOF'
import sys

path, cell_dir, tool, proj, src, dst, commit, output_dir, duration = sys.argv[1:10]
text = open(path).read()
header = (
    f"# One benchmark cell: {tool}/{proj} {src}->{dst}\n"
    f"tool: {tool}\n"
    f"project: {proj}\n"
    f"source_language: {src}\n"
    f"target_language: {dst}\n"
    f"commit: {commit}\n"
    f"output_dir: {output_dir}\n"
    f"duration_seconds: {duration}\n"
)
open(path, "w").write(header + text)
PYEOF
    echo "DONE $tool/$proj $src->$dst dur=${duration}s -> $result_yml"
}

cells=$(emit_cells)
total=$(echo "$cells" | grep -c . || true)
echo "Sweep $COMMIT: $total cells, timeout ${TIMEOUT}s, results in $BENCH_ROOT"
index=0
while read -r tool proj src dst; do
    [ -n "$tool" ] || continue
    index=$((index + 1))
    if [ "$LIMIT" -gt 0 ] && [ "$index" -gt "$LIMIT" ]; then break; fi
    echo "=== [$index/$total] $tool/$proj $src->$dst"
    run_cell "$tool" "$proj" "$src" "$dst"
done <<< "$cells"
echo "ALL CELLS COMPLETE ($total total)"
