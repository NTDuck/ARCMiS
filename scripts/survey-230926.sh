#!/usr/bin/env bash
# Survey 230926 sweep: 7 projects x 3 methods x 8 models x 5 reps against
# the local ollama daemon. One experiment directory per run under
# experiments/230926/runs/<project>/<method>/<model>/rep<N>/, each with
# manifest, config, logs, trace, and the scored result. Finished runs
# are skipped by their .done marker, so an interrupted sweep resumes.
#
# Generous budgets: the configs carry max_turns 120, recode 11 rounds,
# ledger 60/120. The per-run timeout is the only hard ceiling; it logs
# the turn count actually used before the timeout kills a run.
#
# Models switch between blocks, never inside one: --offload unloads the
# resident model before and after every harness run, so a switch block
# always starts from an empty GPU.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO" || exit 1

SURVEY="$REPO/experiments/230926"
LOG="$SURVEY/survey.log"
mkdir -p "$SURVEY"

PROJECTS=(amp ulidgen chtrie murmurhash_c gorilla-paper-encode approxidate xopt)
METHODS=(monolith ledger recode)
# Eight models: the official qwen3.8 27b plus both swift variants, and the
# five other local models that passed the chat and tool-call probes.
# Spark-X2.5 is out: the local llama.cpp build lacks the spark2_5
# architecture and every repackaging fails to load. zen-pro is out: it
# never emits tool calls. The qwen3.8 derivatives jetelain and orcarouter
# are out: not diverse beyond the official and swift variants.
MODELS=(
  "qwen3.8:27b-mtp-q4_K_M"
  "smtek/Swift-Qwen3.8-27B:dflash2"
  "smtek/Swift-Qwen3.8-27B:map-k4v"
  "openbmb/minicpm5-2b:q8_0"
  "granite4.1:3b-bf16"
  "lfm2.5:8b-a1b-bf16"
  "hf.co/bartowski/Altworld_Hemmingway-1-GGUF:q4_K_M"
  "mannix/omnimerge-v6:vision-Q4_K_M"
)
REPS=5
# Generous per-run ceiling. A 27B run on the biggest sample with 120-turn
# budgets can run long; 2h kills only true runaways and the driver logs
# the turns used before the kill.
TIMEOUT=7200

echo "survey 230926 start $(date -Is)" >> "$LOG"

for model in "${MODELS[@]}"; do
  # Cargo rejects ':' and '/' in path segments, so the model tag cannot
  # name the run directory. Sanitize every non-alphanumeric run.
  safe_model="${model//[^a-zA-Z0-9._-]/_}"
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
        # model from the sweep cell. Budgets come from the sample config.
        sed -e "s#^[[:space:]]*dir:[[:space:]].*#  dir: $out/workspace#" \
            -e "s#^[[:space:]]*model:[[:space:]].*#  model: $model#" \
            "$REPO/assets/configs/$project/config.yml" > "$cfg"
        echo "=== $project $method $safe_model rep$rep start $(date -Is)" >> "$LOG"
        start_ns=$(date +%s%N)
        OLLAMA_API_BASE_URL="${OLLAMA_API_BASE_URL:-http://localhost:11434}" \
          timeout "$TIMEOUT" target/release/harness "$cfg" "$out" --method "$method" --offload \
          > "$out/stdout.log" 2> "$out/stderr.log"
        rc=$?
        end_ns=$(date +%s%N)
        wall_s=$(( (end_ns - start_ns) / 1000000000 ))
        # Turns used: the highest model-call turn across every agent in the
        # trace. Tracing colors the output, and the ANSI escapes wrap the
        # `turn` key, so strip them before matching. The rig runner logs
        # `Agent run finished turn=X` per agent; take the max so the
        # report can compare budgets to usage.
        turns=$(sed 's/\x1b\[[0-9;]*m//g' "$out/stdout.log" 2>/dev/null \
          | grep -oE "model call turn=[0-9]+" | grep -oE "[0-9]+" | sort -n | tail -1)
        turns="${turns:-0}"
        # One marker per attempt: a failed cell stays failed on resume
        # instead of retrying a deterministic failure forever.
        touch "$out/.done"
        echo "{\"project\":\"$project\",\"method\":\"$method\",\"model\":\"$safe_model\",\"rep\":$rep,\"rc\":$rc,\"wall_s\":$wall_s,\"turns\":$turns}" >> "$LOG"
        echo "=== $project $method $safe_model rep$rep rc=$rc wall=${wall_s}s turns=$turns $(date -Is)" >> "$LOG"
      done
    done
  done
done
echo "survey 230926 end $(date -Is)" >> "$LOG"
