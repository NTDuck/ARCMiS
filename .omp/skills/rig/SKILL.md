---
name: rig
description: "Operating knowledge for the rig Rust LLM framework (0xPlaygrounds/rig) — the references/ corpus is the reference. Taken verbatim from upstream main. Read the closest example before you write rig code. Use when building agents, tools, RAG/vector search, structured extraction, multi-agent topologies, conversation memory, or provider integrations with rig."
---

# rig — Rust LLM Framework (0xPlaygrounds/rig)

This skill holds **no distilled ground truth**. The reference material is the
upstream `examples/` corpus, copied **verbatim** under `references/` — every
file byte-identical to the upstream commit pinned below. When upstream text
says "See source", the source is here: `references/<name>/src/main.rs`.

## Provenance

- Corpus: `0xPlaygrounds/rig` `examples/`, copied here as `references/`, commit
  [`9b94481`](https://github.com/0xPlaygrounds/rig/commit/9b944819541e8191fe065f62de913802eb221bb1)
  (2026-09-17, upstream `main` HEAD at capture).
- rig ships breaking changes between minor versions (upstream README carries a
  standing dragon warning). Treat example APIs as the current upstream idiom.
  Re-capture before writing code against a different rig version.
- Refresh: copy `examples/` fresh from upstream into `references/`, byte-for-byte,
  and update this commit pin. No other file in this skill carries API claims.

## Layout

```
SKILL.md            ← this file (navigation only)
references/         ← verbatim upstream corpus (from upstream examples/)
  README.md         ← upstream's index: one row per example, run commands
  <name>/           ← one self-contained cargo package per directory
    src/main.rs     ← the example
    Cargo.toml
  documents/        ← shared sample PDFs (used by pdf_agent)
  otel/             ← shared OTel collector config (Dockerfile + config.yaml)
```

65 packages. `discord_bot` is workspace-excluded upstream and carries its own
lockfile. Every other package belongs to the upstream workspace.

## How to use

1. **Find the closest example.** Start from
   [`references/README.md`](references/README.md) — the upstream index table. If
   its description says "See source", read the source. It is local.
2. **Read the whole example before writing anything.** Examples are small
   (usually one `main.rs`). They show current idiom for builders, streaming,
   hooks, extraction, stores, and provider wiring.
3. **Copy the pattern, not the code.** Adapt providers/models/keys. Verify each
   symbol against this corpus (grep under `references/`) or against your rig
   version (`cargo tree`, docs.rs). The compiler is the cheapest verifier.
4. **Never write a rig symbol from memory.** No symbol in your diff may exist
   only in a model prior. Ground it here, or verify it in the rig version that
   you compile against. Unverified symbol → stop, ground, then write.

## Cheat sheet: task → examples

Navigation aid only. The rows mirror `references/README.md`. The source of
truth for what an example does is the example itself.

| Task | Read first |
| --- | --- |
| Smallest agent | `agent` |
| Agent + runtime tools | `agent_with_tools` |
| Streaming | `agent_stream_chat` |
| Multi-turn with history | `multi_turn_agent`, `multi_turn_agent_extended` |
| Turn budget | `agent_with_default_max_turns` |
| Hand-driven run loop (sans-IO) | `agent_run_stepping` |
| Durable human approval (serialize/resume) | `agent_with_durable_approval` |
| Interactive human approval | `agent_with_human_in_the_loop` |
| Policy-based approval (allow-list) | `agent_with_approval_policy` |
| Hooks (lifecycle, retry, request patch) | `request_hook`, `agent_with_retry_hook`, `force_tool_first_turn` |
| Custom HTTP client / middleware | `reqwest_middleware`, `http_middleware` |
| OTel tracing | `agent_with_tools_otel`, `openai_streaming_with_tools_otel`, `openai_agent_completions_api_otel` (config in `references/otel/`) |
| Per-call usage from a stream | `openai_streaming_per_call_usage` |
| Enum-dispatch over providers | `enum_dispatch` |
| Provider-specific (Gemini) flows | `gemini_*` |
| Local models (candle) | `candle_local`, `candle_wasm_chat` |
| MCP via rmcp | `rmcp_example` |
| Typed extraction | `extractor`, `sentiment_classifier` |
| Multi-extract fan-out | `multi_extract` |
| Embeddings + vector search | `vector_search`, `vector_search_ollama`, `vector_search_cohere`, `cohere_image_embeddings` |
| RAG pipelines | `rag`, `chain`, `rag_ollama`, `pdf_agent`, `gemini_extractor_with_rag` |
| Dynamic tool retrieval (RAG) | `rag_dynamic_tools`, `rag_dynamic_tools_multi_turn` |
| Custom vector store | `custom_vector_store` |
| Conversation memory | `agent_with_memory`, `agent_with_memory_streaming` |
| Multi-agent topologies | `multi_agent`, `agent_orchestrator`, `agent_routing`, `agent_parallelization`, `agent_prompt_chaining`, `agent_evaluator_optimizer`, `debate` |
| Autonomous / agentic loops | `agent_autonomous`, `complex_agentic_loop_claude`, `reasoning_loop` |
| Agent-as-tool | `agent_with_agent_tool`, `agent_with_echochambers` |
| Tool result handling | `tool_result_outcomes`, `manual_tool_calls` |
| Context loading | `agent_with_context`, `agent_with_loaders` |
| Transcription / image / video | `transcription`, `gemini_nanobanana_image_generation`, `gemini_video_understanding` |
| Discord bot | `discord_bot` (own workspace, run with `--manifest-path`) |
| No-tokio agent | `agent_no_tokio` |

## Refresh procedure

```sh
# from a rig checkout (or a sparse clone at the pin):
rm -rf .omp/skills/rig/references
cp -r <rig>/examples .omp/skills/rig/references
# then update the Provenance commit pin in this file
```

Verify with `diff -r` against the upstream commit. Byte-identical, or it is not
"as-is".
