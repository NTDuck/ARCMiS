#!/usr/bin/env bash
# Score one benchmark cell's produced workspace: compile + test pass rate.
#
# Reads the raw run under $BENCH_SCORE_ROOT (or --run) — the .artifacts tree
# the driver left behind — inspects workspace/target for a target-language
# build, runs the test command, and writes a scoring record to --out
# (result.yml). Independent of the harness and of the driver: safe to re-run
# offline against an existing .artifacts tree.
#
# Scored record fields:
#   score_status        tests_green | tests_failed | compile_failed | timeout | no_workspace
#   compilation_status  ok | failed | error
#   tests_pass / tests_fail / test_pass_rate   (test_pass_rate: null when no tests ran)
#
# Test commands per target language (system toolchains, per mise.toml):
#   rust        cargo build; cargo test
#   python      compileall; pytest when test files exist (.venv-bench python)
#   javascript  node --check per file
#
# Usage: BENCH_SCORE_ROOT=.artifacts/... bench/score-run.sh \
#            --tool crust --src-lang c --dst-lang rust \
#            --logs logs/ --out result.yml
#        bench/score-run.sh --rescore benchmarks/<commit>   # re-score every cell
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

LOGDIR="logs"
OUT=""
TOOL=""
SRC_LANG=""
DST_LANG=""
RESCORE_COMMIT=""
PYBIN=".venv-bench/bin/python"
[ -x "$PYBIN" ] || PYBIN="python3"

while [ $# -gt 0 ]; do
    case "$1" in
        --run) BENCH_SCORE_ROOT="$2"; shift 2 ;;
        --logs) LOGDIR="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        --tool) TOOL="$2"; shift 2 ;;
        --src-lang) SRC_LANG="$2"; shift 2 ;;
        --dst-lang) DST_LANG="$2"; shift 2 ;;
        --rescore) RESCORE_COMMIT="$2"; shift 2 ;;
        *) echo "unknown arg $1" >&2; exit 1 ;;
    esac
done

# Score one workspace directory. Sets: score_status, compilation_status,
# tests_pass, tests_fail. The dst language decides the toolchain; the tool
# name is irrelevant (a produced oxidizer/crust workspace is always Rust).
score_workspace() {
    ws="$1" # workspace dir containing target/ (or the project root)
    mkdir -p "$LOGDIR"
    : > "$PWD_BUILD" # fresh logs per scoring pass; counts are summed below
    : > "$PWD_TEST"
    tests_pass=0
    tests_fail=0
    score_status="no_workspace"
    compilation_status="error"
    if [ ! -d "$ws" ]; then
        return 0
    fi
    case "$DST_LANG" in
        rust)
            if [ -f "$ws/Cargo.toml" ]; then
                root="$ws"
            elif [ -f "$ws/target/Cargo.toml" ]; then
                root="$ws/target"
            else
                score_status="no_workspace"
                return 0
            fi
            if (cd "$root" && cargo build >>"$PWD_BUILD" 2>&1); then
                compilation_status="ok"
            else
                compilation_status="failed"
                score_status="compile_failed"
                return 0
            fi
            if (cd "$root" && cargo test >>"$PWD_TEST" 2>&1); then
                score_status="tests_green"
            else
                score_status="tests_failed"
            fi
            tests_pass=$(grep -hoP '\d+(?= passed)' "$PWD_TEST" 2>/dev/null | awk '{s+=$1} END{print s+0}')
            tests_fail=$(grep -hoP '\d+(?= failed)' "$PWD_TEST" 2>/dev/null | awk '{s+=$1} END{print s+0}')
            ;;
        python)
            if [ -z "$(find "$ws" -name '*.py' -print -quit 2>/dev/null)" ]; then
                score_status="no_workspace"
                return 0
            fi
            if (cd "$ws" && "$PYBIN" -m compileall -q . >>"$PWD_BUILD" 2>&1); then
                compilation_status="ok"
            else
                compilation_status="failed"
                score_status="compile_failed"
                return 0
            fi
            if find "$ws" \( -name 'test_*.py' -o -name '*_test.py' \) | grep -q .; then
                if (cd "$ws" && "$PYBIN" -m pytest --tb=no -q . >>"$PWD_TEST" 2>&1); then
                    score_status="tests_green"
                else
                    score_status="tests_failed"
                fi
                tests_pass=$(grep -hoP '\d+(?= passed)' "$PWD_TEST" 2>/dev/null | awk '{s+=$1} END{print s+0}')
                tests_fail=$(grep -hoP '\d+(?= failed)' "$PWD_TEST" 2>/dev/null | awk '{s+=$1} END{print s+0}')
            else
                score_status="tests_green" # compile-only cell: no runnable suite
            fi
            ;;
        javascript)
            if [ -z "$(find "$ws" -name '*.js' -not -path '*/node_modules/*' -print -quit 2>/dev/null)" ]; then
                score_status="no_workspace"
                return 0
            fi
            ok=0
            while IFS= read -r f; do
                node --check "$f" >>"$PWD_BUILD" 2>&1 || ok=1
            done < <(find "$ws" -name '*.js' -not -path '*/node_modules/*')
            if [ "$ok" = 0 ]; then
                compilation_status="ok"
                score_status="tests_green" # syntax-check only: no runnable suite
            else
                compilation_status="failed"
                score_status="compile_failed"
            fi
            ;;
        *)
            # No toolchain for this target: score the run, not the code.
            compilation_status="unknown"
            score_status="tests_green"
            ;;
    esac
}

PWD_BUILD="$(pwd)/build.log"
PWD_TEST="$(pwd)/test.log"

if [ -n "$RESCORE_COMMIT" ]; then
    # Offline re-score: every cell whose raw run still exists gets its
    # result.yml scoring fields refreshed (identity fields preserved).
    count=0
    while IFS= read -r -d '' result; do
        cell_dir=$(dirname "$result")
        record=$(python3 - "$result" <<'PYEOF'
import re, sys

# Flat result.yml: key: value lines; values are safe identifiers/paths/numbers.
for line in open(sys.argv[1]):
    key, _, value = line.strip().partition(":")
    if key in ("tool", "project", "source_language", "target_language", "output_dir") and value.strip():
        value = value.strip().replace("'", "'\\''")
        print(f"{key}='{value}'")
PYEOF
        ) || { echo "SKIP unparseable $result"; continue; }
        eval "$record"
        [ -n "${output_dir:-}" ] || { echo "SKIP no output_dir $result"; continue; }
        DST_LANG="$target_language" # toolchain follows the cell's target language
        [ -d "$output_dir/workspace" ] || { echo "SKIP raw run gone $result"; continue; }
        LOGDIR="$cell_dir/logs"
        mkdir -p "$LOGDIR"
        PWD_BUILD="$(cd "$LOGDIR" && pwd)/build.log"
        PWD_TEST="$(cd "$LOGDIR" && pwd)/test.log"
        score_workspace "$output_dir/workspace"
        python3 - "$result" "$score_status" "$compilation_status" "$tests_pass" "$tests_fail" <<'PYEOF'
import sys

# Rewrite the scoring fields in place, preserving identity fields and order.
path, status, compile_status, passed, failed = sys.argv[1:6]
lines = open(path).read().splitlines()
fields = {
    "score_status": status,
    "compilation_status": compile_status,
    "tests_pass": str(int(passed or 0)),
    "tests_fail": str(int(failed or 0)),
}
seen = set()
out = []
for line in lines:
    key = line.split(":", 1)[0].strip()
    if key in fields:
        if key in seen:
            continue
        seen.add(key)
        out.append(f"{key}: {fields[key]}")
    else:
        out.append(line)
for key, value in fields.items():
    if key not in seen:
        out.append(f"{key}: {value}")
total = int(fields["tests_pass"]) + int(fields["tests_fail"])
rate = str(int(fields["tests_pass"]) * 100 // total) if total else "null"
if any(line.startswith("test_pass_rate:") for line in out):
    out = [f"test_pass_rate: {rate}" if line.startswith("test_pass_rate:") else line for line in out]
else:
    out.append(f"test_pass_rate: {rate}")
open(path, "w").write("\n".join(out) + "\n")
PYEOF
        count=$((count + 1))
        echo "RESCORE $output_dir -> $score_status"
    # mindepth 4: tool/proj/cell/result.yml — the sweep-level result.yml is
    # the aggregator's output, not a cell.
    done < <(find "benchmarks/$RESCORE_COMMIT" -mindepth 4 -name result.yml -print0)
    echo "Rescored $count cells under benchmarks/$RESCORE_COMMIT"
    exit 0
fi

# Single-run scoring. Defaults from the environment the driver exports.
: "${BENCH_SCORE_ROOT:?set BENCH_SCORE_ROOT or pass --run}"
: "${DST_LANG:?pass --dst-lang}"
mkdir -p "$LOGDIR"
PWD_BUILD="$(cd "$LOGDIR" && pwd)/build.log"
PWD_TEST="$(cd "$LOGDIR" && pwd)/test.log"
score_workspace "$BENCH_SCORE_ROOT/workspace"
: "${OUT:?pass --out or --rescore}"
total=$((tests_pass + tests_fail))
if [ "$total" -gt 0 ]; then
    rate=$((tests_pass * 100 / total))
else
    rate="null"
fi
cat > "$OUT" <<EOF
score_status: $score_status
compilation_status: $compilation_status
tests_pass: $tests_pass
tests_fail: $tests_fail
test_pass_rate: $rate
EOF
echo "SCORE $BENCH_SCORE_ROOT status=$score_status pass=$tests_pass fail=$tests_fail"
