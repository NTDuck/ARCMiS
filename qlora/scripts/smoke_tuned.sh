#!/usr/bin/env bash
# Verify the fine-tuned model is servable before a benchmark sweep. One
# trivial prompt; fails with a clear fix when the model is not registered.
# Run after merge_and_export.sh, before bench/run-benchmarks.sh.
set -euo pipefail

model="${1:-recodeagent-sft}"

if ! ollama list >/dev/null 2>&1; then
    echo "ollama daemon unreachable; start it with 'ollama serve'." >&2
    exit 1
fi

if ! ollama list | awk '{print $1}' | grep -qx "$model"; then
    echo "model '$model' is not registered with ollama." >&2
    echo "Fix: bash qlora/scripts/merge_and_export.sh   # runs 'ollama create $model'" >&2
    exit 1
fi

if ! reply=$(ollama run "$model" "Reply with the single word: ready" 2>&1); then
    echo "ollama run '$model' failed:" >&2
    echo "$reply" >&2
    exit 1
fi

echo "$model" is servable
echo "$reply"
