# QLoRA fine-tuning on ReCodeAgent traces

Fine-tune `ukisai/Swift-Qwen3.8-27b` (the sweep model family) on the
ReCodeAgent paper's published experiment traces (Claude-CLI `stream-json`
logs from Zenodo DOI 10.5281/zenodo.21842351), then serve the merged model
through ollama for A/B benchmarking against the base model.

## Layout

```
qlora/
  configs/recodeagent-qlora.yml   axolotl QLoRA config (4-bit NF4, LoRA r32)
  scripts/extract_traces.py       results.zip -> messages.jsonl (chat turns)
  scripts/build_dataset.py        messages.jsonl -> train/val.jsonl (axolotl)
  scripts/train.sh                axolotl train invocation
  scripts/merge_and_export.sh     adapters -> merged fp16 -> Q4_K_M GGUF -> ollama
  scripts/eval_smoke.sh           qualitative base-vs-tuned smoke report
  datasets/                       large artifacts, gitignored
  runs/                           training runs, gitignored
```

## Prerequisites

- axolotl installed (separate setup window) with CUDA bf16 support.
- `huggingface-cli` authenticated if the base repo needs it (it is public).
- ~20 GB disk for the zip, dataset, merged fp16 weights, and GGUF.
- The traces zip: `assets/ReCodeAgent/data/results.zip` (1.45 GB, already
  downloaded; md5 `5df332d2a1477ec30f719dd7d0ff2470`).

## Workflow

```bash
# 1. Extract traces (reads the zip in place; no unzip needed).
python3 qlora/scripts/extract_traces.py \
    --results-dir assets/ReCodeAgent/data/results.zip \
    --out-dir qlora/datasets/recodeagent

# 2. Build the axolotl dataset (dedup, length filter, 98/2 session split).
python3 qlora/scripts/build_dataset.py \
    --messages qlora/datasets/recodeagent/messages.jsonl \
    --out-dir qlora/datasets/recodeagent

# 3. Train (after axolotl is installed).
bash qlora/scripts/train.sh

# 4. Merge adapters, quantize Q4_K_M, register in ollama as recodeagent-sft.
bash qlora/scripts/merge_and_export.sh

# 5. Qualitative smoke check.
bash qlora/scripts/eval_smoke.sh
```

## Trace provenance

ReCodeAgent runs Claude CLI with `--output-format stream-json`; every
intercepted event is logged at DEBUG as `Captured JSON: {...}` under
`results/recodeagent_translations/logs/{agent}/{session}.log` inside the
zip. `extract_traces.py` reassembles those events into model-call turns
(system init -> assistant tool calls -> user tool results -> result) and
emits two views: full `messages.jsonl` transcripts and flattened
`sft.jsonl` pairs. Only the validated `recodeagent_translations` tree is
used; the paper's other artifacts (ablations, cost) are not training data.

## Notes

- `build_dataset.py` marks tool traffic with `[TOOL_CALL name]` /
  `[TOOL_RESULT id]` text markers inside user turns, so any chat template
  renders the data without provider-specific tool roles.
- Dedup keys on the normalized final assistant text: ReCodeAgent logs
  contain repeated boilerplate (summaries, template text) that would
  otherwise dominate small datasets.
- Sequence cap: 96k chars (~24k tokens) per row at build time,
  `sequence_len: 4096` at train time with `sample_packing`. Tune
  `--max-chars` if you raise the training sequence length.
- The config targets one 24 GB GPU (3090). On OOM: lower
  `sequence_len` to 2048 or drop `micro_batch_size` (already 1) and raise
  `gradient_accumulation_steps` to keep the effective batch at 16.
