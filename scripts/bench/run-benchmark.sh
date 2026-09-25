#!/usr/bin/env bash
# ARCMiS MAS benchmark driver over the ReCodeAgent tool_projects dataset.
#
# One cell = (family, project, source_lang -> target_lang). For each cell the
# driver writes a dedicated config, runs the harness binary, scores the
# produced workspace (compile status + test pass rate), and persists:
#
#   benchmarks/{commit}/{family}/{project}/{src}2{dst}/result.yml
#   benchmarks/{commit}/{family}/{project}/{src}2{dst}/logs/
#   plus the raw run artifacts under {output_dir}/ (.ARCMiS, run/, workspace/)
#
# Resumable: a cell whose result.yml already exists is skipped unless --force.
#
# Usage: scripts/bench/run-benchmark.sh [--family NAME]... [--project NAME]...
#        [--limit N] [--force] [--timeout SECS]
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1

FAMILIES=()
PROJECTS=()
LIMIT=0
FORCE=0
TIMEOUT=5400
while [ $# -gt 0 ]; do
    case "$1" in
        --family) FAMILIES+=("$2"); shift 2 ;;
        --project) PROJECTS+=("$2"); shift 2 ;;
        --limit) LIMIT="$2"; shift 2 ;;
        --force) FORCE=1; shift ;;
        --timeout) TIMEOUT="$2"; shift 2 ;;
        *) echo "unknown arg $1"; exit 1 ;;
    esac
done

COMMIT=$(git rev-parse --short=12 HEAD)
BENCH_ROOT="benchmarks/$COMMIT"
mkdir -p "$BENCH_ROOT"

DATASET="assets/ReCodeAgent/data/tool_projects"

# Per-family scoring commands, run inside the PRODUCED workspace. The scorer
# returns "pass=<n> fail=<m>" on stdout; compile status comes from the exit
# path. Families whose reference suites carry no runnable tests score
# compile-only.
score_rust() {
    cargo build 2>>"$LOGDIR/build.log" || return 1
    cargo test 2>>"$LOGDIR/test.log" || return 2
}
score_go() { :; } # produced Rust crate still scored as rust
score_python() {
    /home/ayin/projs/ARCMiS/.venv-bench/bin/python -m compileall -q . 2>>"$LOGDIR/build.log" || return 1
    if find . -name "test_*.py" -o -name "*_test.py" | grep -q .; then
        /home/ayin/projs/ARCMiS/.venv-bench/bin/python -m pytest --tb=no -q 2>>"$LOGDIR/test.log" || return 2
    fi
}
score_javascript() {
    ok=0
    while IFS= read -r f; do
        node --check "$f" >>"$LOGDIR/build.log" 2>&1 || ok=1
    done < <(find . -name "*.js" -not -path "./node_modules/*")
    [ "$ok" = 0 ] || return 1
}

family_test_lang() {
    case "$1" in
        crust) echo "rust" ;;
        oxidizer) echo "rust" ;;
        alphatrans) echo "python" ;;
        skel) echo "multi" ;;
    esac
}

# Source->target pair list per family from the dataset layout.
emit_cells() {
    for family in crust oxidizer alphatrans skel; do
        [ ${#FAMILIES[@]} -gt 0 ] || true
        if [ ${#FAMILIES[@]} -gt 0 ] && ! printf '%s\n' "${FAMILIES[@]}" | grep -qx "$family"; then
            continue
        fi
        for proj_dir in "$DATASET/$family"/*/; do
            proj=$(basename "$proj_dir")
            if [ ${#PROJECTS[@]} -gt 0 ] && ! printf '%s\n' "${PROJECTS[@]}" | grep -qx "$proj"; then
                continue
            fi
            for lang_dir in "$proj_dir"*/; do
                lang=$(basename "$lang_dir")
                case "$family:$lang" in
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
                echo "$family $proj $src $dst"
            done
        done
    done
}

if [ "${DRY_RUN:-0}" = 1 ]; then
    emit_cells
    exit 0
fi

run_cell() {
    family="$1"; proj="$2"; src="$3"; dst="$4"
    cell_dir="$BENCH_ROOT/$family/$proj/${src}2${dst}"
    result_yml="$cell_dir/result.yml"
    if [ -f "$result_yml" ] && [ "$FORCE" = 0 ]; then
        echo "SKIP $family/$proj/${src}2${dst} (already scored)"
        return 0
    fi
    mkdir -p "$cell_dir/logs"
    LOGDIR="$(cd "$cell_dir/logs" && pwd)"

    # Source root: the SOURCE language dir of the pair.
    case "$family:$src" in
        crust:c) source_root="$DATASET/crust/$proj/c" ;;
        oxidizer:go) source_root="$DATASET/oxidizer/$proj/go" ;;
        alphatrans:java) source_root="$DATASET/alphatrans/$proj/java" ;;
        skel:javascript) source_root="$DATASET/skel/$proj/python" ;;
        skel:python) source_root="$DATASET/skel/$proj/javascript" ;;
        *) source_root="$DATASET/$family/$proj/$src" ;;
    esac

    output_dir=".artifacts/bench/$family/$proj/${src}2${dst}"
    rm -rf "$output_dir"
    mkdir -p "$(dirname "$output_dir")"

    test_command="cargo test"
    case "$dst" in
        python) test_command="/home/ayin/projs/ARCMiS/.venv-bench/bin/python -m pytest --tb=no -q" ;;
        javascript) test_command="node --check" ;;
    esac

    config_path="$cell_dir/config.yml"
    sed -e "s|__OUTPUT_DIR__|$output_dir|g" \
        -e "s|__SOURCE_ROOT__|$source_root|g" \
        -e "s|__TEST_COMMAND__|$test_command|g" \
        -e "s|__SRC_LANG__|$src|g" \
        -e "s|__DST_LANG__|$dst|g" \
        assets/configs/bench/template.yml > "$config_path"

    echo "RUN $family/$proj $src->$dst"
    started=$(date +%s)
    timeout "$TIMEOUT" cargo run -p harness --release -- --config "$config_path" \
        > "$LOGDIR/harness-stdout.log" 2> "$LOGDIR/harness-stderr.log"
    run_exit=$?
    ended=$(date +%s)
    duration=$((ended - started))

    compile="error"; tests_pass=0; tests_fail=0; score_status="harness_failed"
    workspace="$output_dir/workspace"
    if [ $run_exit -eq 124 ]; then
        score_status="timeout"
    elif [ -d "$workspace" ]; then
        (
            cd "$workspace" || exit 99
            case "$dst" in
                rust) score_rust ;;
                python) score_python ;;
                javascript) score_javascript ;;
                *) score_python ;;
            esac
        ) > "$LOGDIR/score.log" 2>&1
        score_exit=$?
        case $score_exit in
            0) compile="ok"; score_status="tests_green" ;;
            1) compile="failed"; score_status="compile_failed" ;;
            2) compile="ok"; score_status="tests_failed" ;;
            99) compile="error"; score_status="no_workspace" ;;
        esac
        # Parse pass/fail counts from the cargo/pytest logs.
        tests_pass=$(grep -hoE "[0-9]+ passed" "$LOGDIR/test.log" 2>/dev/null | head -1 | grep -oE "[0-9]+")
        tests_fail=$(grep -hoE "[0-9]+ failed" "$LOGDIR/test.log" 2>/dev/null | head -1 | grep -oE "[0-9]+")
        tests_pass=${tests_pass:-0}; tests_fail=${tests_fail:-0}
    fi

    # Pull the aggregate the harness wrote (rounds, delegations, stop reason).
    aggregate="$output_dir/run/aggregate.yml"
    [ -f "$aggregate" ] || aggregate="$output_dir/result/aggregate.yml"

    {
        echo "# One benchmark cell: $family/$proj $src->$dst"
        echo "family: $family"
        echo "project: $proj"
        echo "source_language: $src"
        echo "target_language: $dst"
        echo "commit: $COMMIT"
        echo "config: $config_path"
        echo "output_dir: $output_dir"
        echo "duration_seconds: $duration"
        echo "harness_exit: $run_exit"
        echo "score_status: $score_status"
        echo "compilation_status: $compile"
        echo "tests_pass: $tests_pass"
        echo "tests_fail: $tests_fail"
        if [ -n "$tests_pass" ] && [ -n "$tests_fail" ]; then
            total=$((tests_pass + tests_fail))
            if [ "$total" -gt 0 ]; then
                echo "test_pass_rate: $((tests_pass * 100 / total))"
            else
                echo "test_pass_rate: null"
            fi
        else
            echo "test_pass_rate: null"
        fi
        if [ -f "$aggregate" ]; then
            echo "aggregate: |"
            sed 's/^/  /' "$aggregate"
        fi
    } > "$result_yml"
    echo "DONE $family/$proj $src->$dst status=$score_status dur=${duration}s"
}

cells=$(emit_cells)
total=$(echo "$cells" | grep -c .)
index=0
consecutive_failures=0
MAX_CONSECUTIVE_FAILURES="${MAX_CONSECUTIVE_FAILURES:-3}"
while read -r family proj src dst; do
    [ -n "$family" ] || continue
    index=$((index + 1))
    if [ "$LIMIT" -gt 0 ] && [ "$index" -gt "$LIMIT" ]; then break; fi
    run_cell "$family" "$proj" "$src" "$dst"
    status=$(grep -oP '(?<=^score_status: ).*' "$BENCH_ROOT/$family/$proj/${src}2${dst}/result.yml" 2>/dev/null || echo "harness_failed")
    if [ "$status" = "harness_failed" ] || [ "$status" = "compile_failed" ]; then
        consecutive_failures=$((consecutive_failures + 1))
        echo "FAILURE $consecutive_failures/$MAX_CONSECUTIVE_FAILURES at $family/$proj ($status)"
        if [ "$consecutive_failures" -ge "$MAX_CONSECUTIVE_FAILURES" ]; then
            {
                echo "Sweep aborted after $consecutive_failures consecutive failed cells."
                echo "Last failing cell: $family/$proj $src->$dst (status=$status)"
                echo "Diagnosis hints:"
                tail -20 "$BENCH_ROOT/$family/$proj/${src}2${dst}/logs/harness-stderr.log" 2>/dev/null
            } > "$BENCH_ROOT/ABORT.md"
            echo "ABORT: $consecutive_failures consecutive failures; see $BENCH_ROOT/ABORT.md"
            exit 2
        fi
    else
        consecutive_failures=0
    fi
done <<< "$cells"
echo "ALL CELLS COMPLETE ($total total)"
