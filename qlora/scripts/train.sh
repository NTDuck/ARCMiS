#!/usr/bin/env bash
# Fine-tune Swift-Qwen3.8-27B on ReCodeAgent traces with axolotl QLoRA.
# Prereqs (one-time, outside this script): axolotl installed, dataset built
# (extract_traces.py + build_dataset.py), HF model cached.
#
# Usage: bash qlora/scripts/train.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

train_file=qlora/datasets/recodeagent/train.jsonl
val_file=qlora/datasets/recodeagent/val.jsonl
config=qlora/configs/recodeagent-qlora.yml

[[ -s "$train_file" ]] || { echo "missing $train_file - run extract+build first" >&2; exit 1; }
[[ -s "$val_file" ]] || { echo "missing $val_file - run extract+build first" >&2; exit 1; }

mkdir -p qlora/runs
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True

axolotl train "$config" "$@"
