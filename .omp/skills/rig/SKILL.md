---
name: rig
description: "Operating knowledge for the rig Rust LLM framework (0xPlaygrounds/rig) — route to exactly one sub-skill before writing any rig code. Use when building agents, tools, RAG/vector search, structured extraction, multi-agent topologies, conversation memory, or provider integrations with rig."
---

# rig — Rust LLM Framework (0xPlaygrounds/rig)

Router entry point. Follows the AREX-Skill progressive-disclosure model: load this page, pick ONE sub-skill, load only that. Never load all sub-skills at once.

## Provenance

- Upstream: `0xPlaygrounds/rig`, commit `6828097ce102fbb4e26d3aeadd50201cc65c4150` (2026-09-14).
- rig ships breaking changes between minor versions ([upstream README](https://github.com/0xPlaygrounds/rig) carries a standing dragon warning). Sub-skill claims are valid against the pinned commit; re-ground before writing code against a newer rig version.

## What rig is

Rust library for LLM-powered applications: 20+ model providers under one interface, 10+ vector stores, completion + embedding workflows, transcription/image/audio generation, WASM-portable core. `rig-core` holds provider-neutral contracts; `rig-agent` holds the classic agent runtime (builder, prompt/streaming traits, hooks, extraction, `AgentRun` state machine); the root `rig` facade re-exports both — most code depends only on `rig` (`upstream/README.md`, `upstream/crates/rig-agent/README.md`).

## Sub-Skills (load exactly one)

| Sub-skill | Use when the task touches |
| --- | --- |
| [`agents`](sub-skills/agents/SKILL.md) | Agent builder, prompting, streaming, multi-turn, max turns, run stepping, hooks |
| [`tools`](sub-skills/tools/SKILL.md) | Tool trait, tool registration, dynamic tools, manual tool calls, approvals |
| [`rag-vector`](sub-skills/rag-vector/SKILL.md) | Embeddings, vector store trait, store integrations, rerank, retrieval |
| [`extraction`](sub-skills/extraction/SKILL.md) | Typed struct extraction from LLM output, multi-extract |
| [`multi-agent`](sub-skills/multi-agent/SKILL.md) | Orchestrator/routing/parallelization/chaining/evaluator-optimizer/debate topologies |
| [`memory`](sub-skills/memory/SKILL.md) | Conversation memory traits, in-memory backend, history-shaping policies |
| [`providers`](sub-skills/providers/SKILL.md) | Provider clients, model constants, facade features, transcription/image/audio, candle local models |

## Anti-Hallucination Gate (binding, see .omp/rules/skills.md)

Before writing rig code: read the chosen sub-skill; every rig symbol in your diff must appear in that sub-skill's Ground Truth **or** be grep-verified in the rig checkout pinned by its Provenance section. Unverified symbol → stop, ground, then write.
