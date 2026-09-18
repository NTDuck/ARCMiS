---
name: rig-extraction
description: Extract typed structs from LLM output with rig — Extract derive/impl, multi-extract, RAG-grounded extraction
---

# Extraction (typed structured output)

## Scope

Typed extraction: turn free text into a value of a schema-described type —
making a type extractable, running an `Extractor`, multi-extract, RAG-grounded
extraction, error surface. Agents/tools/vector stores are separate sub-skills.

## Ground Truth

- Extractor lives in `rig-agent`: `rig_agent::extractor::{Extractor, ExtractorBuilder}`
  (`upstream/crates/rig-agent/src/extractor.rs`, `upstream/crates/rig-agent/src/lib.rs`).
  Construction sugar is `AgentClientExt::extractor::<T>(model)`, blanket-implemented for
  every `rig_core::client::completion::CompletionClient`; `use ::rig::prelude::*;` brings
  both traits into scope (`upstream/crates/rig-agent/src/client.rs`).
- Target type must implement `::serde::Deserialize`, `::serde::Serialize`, and
  `::schemars::JsonSchema` — all three derive, no extractor trait to impl
  (`upstream/crates/rig-agent/src/extractor.rs:5-6`).
- Builder setters: `.retries(u64)`, `.append_preamble(&str)` (appends, never replaces the
  fixed extraction preamble), `.context(&str)`, `.dynamic_context(samples, index)`,
  `.additional_params(json)`, `.max_tokens(u64)`, `.tool_choice(ToolChoice)`,
  `.add_hook(hook)` (`upstream/crates/rig-agent/src/extractor.rs`).
- `Extractor::extract(text)` returns a `TypedRun<T>` (`#[must_use]`) with per-run tuning
  `.history(..)`, `.using_model(..)`, `.retries(..)`, `.max_turns(..)`; `.await` yields
  `TypedPromptResponse<T>` with public fields `output: T`, `usage: Usage`,
  `completion_calls: Vec<CompletionCall>`, `memory_append: Option<MemoryAppend>`
  (`upstream/crates/rig-agent/src/agent/typed.rs`).
- Mechanism: each extract is a one-turn run in `OutputMode::Tool` with a synthetic `submit`
  tool whose arguments are the value; `ToolChoice::Required` is forced
  (`upstream/crates/rig-agent/src/extractor.rs:98-105,148-163`).
- Retry semantics: an attempt fails on run error, empty output, or unparseable output;
  retries restart from scratch and `usage` accumulates across billed attempts
  (`upstream/crates/rig-agent/src/agent/typed.rs:99-105,342-399`).
- Errors: `StructuredOutputError::{PromptError(Box<PromptError>),
  DeserializationError(serde_json::Error), EmptyResponse}`
  (`upstream/crates/rig-agent/src/completion.rs:61-73`). `Usage` fields: `input_tokens`,
  `output_tokens`, `total_tokens`, `cached_input_tokens`,
  `cache_creation_input_tokens`, `tool_use_prompt_tokens`
  (`upstream/crates/rig-core/src/completion/request.rs:573-602`).

## Workflow

1. Derive the trio and tag `Option<T>` fields with `#[schemars(required)]` so the
   generated JSON schema lists them as required (upstream examples always do this,
   e.g. `upstream/examples/extractor/src/main.rs:11-19`):
```rust
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::serde::Serialize, ::schemars::JsonSchema)]
struct Person {
    #[schemars(required)]
    first_name: Option<String>,
    #[schemars(required)]
    age: Option<u8>,
}
```

2. Build from a client (any provider with `CompletionClient`; `use ::rig::prelude::*;`):

```rust
let client = ::rig::providers::openai::Client::from_env()?;
let extractor = client
    .extractor::<Person>(::rig::providers::openai::GPT_4O)
    .retries(2)
    .build();

let person = extractor.extract("John Doe is a 30 year old doctor.").await?.output;
let response = extractor.extract("Jane Smith is a data scientist.").await?;
// response.usage.total_tokens aggregates across every attempt.
```

3. Multi-extract: fan out with `::futures::try_join!` over extractors and `.buffered(n)`
   over inputs. `TypedRun` is `IntoFuture`, so each call needs `.into_future()` inside
   `try_join!` (`upstream/examples/multi_extract/src/main.rs:60-82`):
```rust
use ::futures::stream::{StreamExt, TryStreamExt};
use ::std::future::IntoFuture;

let responses: Vec<String> = ::futures::stream::iter(inputs)
    .map(|text| {
        let (names, topics) = ::futures::try_join!(
            names_extractor.extract(text).into_future(),
            topics_extractor.extract(text).into_future(),
        )?;
        ::anyhow::Ok(names.output.names.join(", "))
    })
    .buffered(4)
    .try_collect()
    .await?;
```

4. RAG-grounded extraction: derive `::rig::Embed` with `#[embed]` on searchable fields,
   build an `InMemoryVectorStore` via `EmbeddingsBuilder`, then attach
   `.dynamic_context(samples, index)` — retrieved docs are injected as context for every
   extraction (`upstream/examples/gemini_extractor_with_rag/src/main.rs:14-104`):

```rust
#[derive(Embed, Serialize, Clone, Debug, Default)]
struct Question {
    #[embed]
    text: String,
}

let index = InMemoryVectorStore::from_documents(embeddings).index(embedding_model);
let rag_extractor = client
    .extractor::<Answers>("gemini-2.5-flash")
    .append_preamble("Answer the questions from the provided background.")
    .dynamic_context(3, index)
    .build();
```

## Pitfalls

- `StructuredOutputError` and `PromptError` are non-exhaustive — matches need a
  wildcard arm (`upstream/crates/rig-core/CHANGELOG.md`).
- `EmptyResponse` means the model never called the `submit` tool; bump retries and/or
  use a stronger model. Only the final error surfaces, not per-attempt ones.
- Forgetting `#[schemars(required)]` on `Option` fields leaves them out of the schema's
  required list, so models routinely drop them.
- A `TypedRun` is `#[must_use]` — dropping it without `.await` does nothing.
- `submit` is a real tool call, so the provider/model must support tool calling;
  extraction always runs in output-tool mode (one turn, `ToolChoice::Required`).
- Gemini: extractors pin `OutputMode::Tool` and cannot read from Gemini `cached_content`;
  context documents are fine (`upstream/crates/rig-core/src/providers/gemini/completion.rs:133-138`).
- `.append_preamble` never replaces the fixed extraction preamble; use it to add
  field-level instructions only.

## Verify

1. `cargo check` your crate: the derive trio and builder bounds are compile-time.
2. Smoke run: extract a known sentence, assert `.output` fields and
   `response.usage.total_tokens > 0`.
3. Force a failure path and confirm you get `StructuredOutputError::EmptyResponse` or
   `PromptError` after retries are spent.

## Reference Examples (upstream `examples/`)

Runnable, idiomatic usage of this sub-skill's API surface. Captured from
upstream `main` at commit `9b94481` (2026-09-17) — newer than the Ground
Truth pin. Treat example APIs as the current idiom and re-verify against
your rig version.

| Example | Demonstrates |
| --- | --- |
| `extractor` | Typed extraction plus usage metadata from `TypedPromptResponse`. |
| `sentiment_classifier` | Smallest extractable type: enum classification. |
| `multi_extract` | `try_join!` fan-out plus `.buffered(n)` over inputs. |
| `gemini_extractor_with_rag` | `dynamic_context`-grounded extraction on Gemini. Also under rag-vector. |
| `agent_autonomous` | Extractor loop feeding its own output back until a stop condition. |

## Provenance

- Repo: `0xPlaygrounds/rig`, commit `6828097` (2026-09-14).
- Upstream ships breaking changes between versions; re-ground every symbol against the
  pinned commit before reuse.
