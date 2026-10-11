#!/usr/bin/env bash
# Smoke-compare the fine-tuned model against the base model on a handful of
# prompts sampled from the validation split. Qualitative gate only - the real
# metric is the ARCMiS benchmark pass rate.
#
# Usage: bash qlora/scripts/eval_smoke.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

val=qlora/datasets/recodeagent/val.jsonl
out=qlora/runs/eval-smoke.md
[[ -s "$val" ]] || { echo "missing $val - build the dataset first" >&2; exit 1; }

mkdir -p qlora/runs
{
  echo "# QLoRA eval smoke - $(date -u +%FT%TZ)"
  echo
  echo "Base vs fine-tuned on up to 5 validation prompts."
  echo
  python3 - "$val" <<'PY'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1])][:5]
for row in rows:
    user = next(m["content"] for m in row["messages"] if m["role"] == "user")
    print("##", row["id"])
    print("### prompt")
    print(user[:1200])
    print()
PY
} > "$out"

for model in recodeagent-sft; do
  echo "Polling ollama for $model (create it first: qlora/scripts/merge_and_export.sh)" >&2
  if ollama show "$model" >/dev/null 2>&1; then
    {
      echo "## $model answers"
      python3 - "$val" <<'PY' | while IFS= read -r prompt; do
        ollama run "$model" "$prompt" 2>/dev/null || echo "(generation failed)"
        echo
      done
import json, sys
for line in open(sys.argv[1]).readlines()[:5]:
    row = json.loads(line)
    user = next(m["content"] for m in row["messages"] if m["role"] == "user")
    print(user[:1200].replace("\n", " "))
PY
      echo
    } >> "$out"
  else
    echo "## $model not present - skipped" >> "$out"
  fi
done

echo "wrote $out"
