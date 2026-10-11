#!/usr/bin/env bash
# Merge the trained QLoRA adapters into full weights and export a GGUF for
# ollama. Run after train.sh completes.
#
# Usage: bash qlora/scripts/merge_and_export.sh [run_dir]
set -euo pipefail
cd "$(dirname "$0")/../.."

run_dir="${1:-qlora/runs/recodeagent-sft}"
merged=qlora/runs/recodeagent-sft-merged
gguf_dir=qlora/runs/gguf

[[ -d "$run_dir" ]] || { echo "missing run dir $run_dir" >&2; exit 1; }
mkdir -p "$merged" "$gguf_dir"

# 1. Merge LoRA adapters into the base weights (fp16 master).
axolotl merge-lora "$run_dir" --lora-model-dir "$run_dir" --output-dir "$merged"

# 2. Quantize to Q4_K_M GGUF (the quant the sweep already serves).
python3 -m llama_cpp.convert \
    "$merged" --outfile "$gguf_dir/recodeagent-sft-Q4_K_M.gguf" \
    --outtype q4_k_m || {
    echo "llama_cpp.convert unavailable - use llama-quantize on an f16 GGUF:" >&2
    echo "  python3 -m llama_cpp.convert \"$merged\" --outtype f16 && llama-quantize p8 f16 q4_k_m" >&2
    exit 1
}

# 3. Register with ollama for A/B eval against the base model.
cat > "$gguf_dir/Modelfile" <<EOF
FROM $gguf_dir/recodeagent-sft-Q4_K_M.gguf
PARAMETER num_ctx 65536
EOF
ollama create recodeagent-sft -f "$gguf_dir/Modelfile"
ollama list | grep recodeagent-sft
