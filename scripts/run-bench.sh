#!/usr/bin/env bash
# 5x3 benchmark: monolith, ledger, recode on the same task and model.
# Runs run sequentially. Workspace dirs sit under .artifacts/bench/<m>/rep<N>/
# with a Cargo.toml inside, so each rep's workspace is excluded from the
# ARCMiS workspace by a target/ nested-workspace guard: harness runs from the
# repo root, and cargo only errors when CWD is inside the nested root. The
# harness runs cargo inside the workspace dir (bash tool roots there), so the
# nested workspace is fine.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO" || exit 1
BENCH="$REPO/.artifacts/bench"
METHODS=(monolith ledger recode)
declare -A CONFIGS=(
  [monolith]=assets/configs/GildedRose-Refactoring-Kata/config.yml
  [ledger]=assets/configs/ledger-method/config.yml
  [recode]=assets/configs/ReCodeAgent-method/config.yml
)
mkdir -p "$BENCH"
echo "bench start $(date -Is)" >> "$BENCH/bench.log"
for method in "${METHODS[@]}"; do
  for rep in 1 2 3 4 5; do
    out="$BENCH/$method/rep$rep"
    rm -rf "$out"
    mkdir -p "$out"
    cfg="$out/config.yml"
    sed "s#^[[:space:]]*dir:[[:space:]].*#  dir: $out/workspace#" "$REPO/${CONFIGS[$method]}" > "$cfg"
    echo "=== $method rep$rep start $(date -Is)" >> "$BENCH/bench.log"
    start_ns=$(date +%s%N)
    OLLAMA_API_BASE_URL="${OLLAMA_API_BASE_URL:-http://localhost:11434}" \
      timeout 3600 cargo run --quiet -p harness --manifest-path "$REPO/Cargo.toml" -- "$cfg" \
      > "$out/stdout.log" 2> "$out/stderr.log"
    rc=$?
    end_ns=$(date +%s%N)
    wall_s=$(( (end_ns - start_ns) / 1000000000 ))
    echo "{\"method\":\"$method\",\"rep\":$rep,\"rc\":$rc,\"wall_s\":$wall_s}" >> "$BENCH/bench.log"
    echo "=== $method rep$rep rc=$rc wall=${wall_s}s $(date -Is)" >> "$BENCH/bench.log"
  done
done
echo "bench end $(date -Is)" >> "$BENCH/bench.log"
