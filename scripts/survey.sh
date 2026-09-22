#!/usr/bin/env bash
# Survey sweep: 4 projects x 3 methods x 3 models x 5 reps, all against
# the local ollama daemon. One experiment directory per run under
# experiments/220926/runs/<project>/<method>/<model>/rep<N>/, each with
# manifest, config, logs, trace, and the scored result. Finished runs
# are skipped by their per_problem.json or .done marker, so an
# interrupted sweep resumes where it stopped.
#
# Models switch between blocks, never inside one: --offload unloads
# the resident model before and after every harness run, so a switch
# block always starts from an empty GPU.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO" || exit 1

SURVEY="$REPO/experiments/220926"
LOG="$SURVEY/survey.log"
mkdir -p "$SURVEY"

PROJECTS=(fft totp cjson expr)
METHODS=(monolith ledger recode)
# Three ollama models: one small 2B, two 27B fine-tunes. Spark-X2.5 is
# out: its spark2_5 GGUF architecture fails to load in the local daemon.
MODELS=(
  "openbmb/minicpm5-2b:q8_0"
  "qwen3.8:27b-mtp-q4_K_M"
  "smtek/Swift-Qwen3.8-27B:dflash2"
)
# The ledger manager loops over its own task list, the recode pipeline
# runs four agents, the monolith runs two; the recode config carries 40
# turns for the four-phase pipeline, the others 14. One budget per
# method, read from its GildedRose config, applied to every project.
declare -A TURNS=([monolith]=14 [ledger]=14 [recode]=40)
REPS=5
TIMEOUT=1800

echo "survey start $(date -Is)" >> "$LOG"

for model in "${MODELS[@]}"; do
  safe_model="${model//\//_}"
  for project in "${PROJECTS[@]}"; do
    for method in "${METHODS[@]}"; do
      for rep in $(seq 1 "$REPS"); do
        out="$SURVEY/runs/$project/$method/$safe_model/rep$rep"
        # Resume marker: the run scored and copied its result.
        if [ -f "$out/result/per_problem.json" ] || [ -f "$out/.done" ]; then
          echo "skip $project $method $safe_model rep$rep (done)" >> "$LOG"
          continue
        fi
        rm -rf "$out"
        mkdir -p "$out"
        cfg="$out/config.yml"
        # Render the per-run config: workspace inside the run dir, the
        # model and turn budget from the sweep cell.
        sed -e "s#^[[:space:]]*dir:[[:space:]].*#  dir: $out/workspace#" \
            -e "s#^[[:space:]]*model:[[:space:]].*#  model: $model#" \
            -e "s#^[[:space:]]*max_turns:[[:space:]].*#  max_turns: ${TURNS[$method]}#" \
            "$REPO/assets/configs/$project/config.yml" > "$cfg"
        echo "=== $project $method $safe_model rep$rep start $(date -Is)" >> "$LOG"
        start_ns=$(date +%s%N)
        OLLAMA_API_BASE_URL="${OLLAMA_API_BASE_URL:-http://localhost:11434}" \
          timeout "$TIMEOUT" target/debug/harness "$cfg" "$out" --method "$method" --offload \
          > "$out/stdout.log" 2> "$out/stderr.log"
        rc=$?
        end_ns=$(date +%s%N)
        wall_s=$(( (end_ns - start_ns) / 1000000000 ))
        # One marker per attempt: a failed cell stays failed on resume
        # instead of retrying a deterministic failure forever.
        touch "$out/.done"
        echo "{\"project\":\"$project\",\"method\":\"$method\",\"model\":\"$safe_model\",\"rep\":$rep,\"rc\":$rc,\"wall_s\":$wall_s}" >> "$LOG"
        echo "=== $project $method $safe_model rep$rep rc=$rc wall=${wall_s}s $(date -Is)" >> "$LOG"
      done
    done
  done
done
echo "survey end $(date -Is)" >> "$LOG"
