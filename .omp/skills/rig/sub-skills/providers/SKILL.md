---
name: rig-providers
description: Use rig's unified provider interface — 26 providers behind one Provider trait, capability traits (completion, embeddings, transcription, image, audio, rerank), model constants, from_env clients, facade feature gating, and local models via candle.
---

# Rig Providers

## Scope

Client construction, model constants, the capability-trait architecture, and
non-chat capabilities (transcription, image generation, audio generation) for
rig's built-in providers. Facade companion crates (bedrock, vertexai,
gemini-grpc, candle) and agent building are cross-referenced only.

## Ground Truth

- 26 provider modules live in `crates/rig-core/src/providers/mod.rs` —
  anthropic, azure, chatgpt, cohere, copilot, deepseek, doubleword, gemini,
  groq, huggingface, hyperbolic, llamacpp, minimax, mira, mistral, moonshot,
  ollama, openai, openrouter, perplexity, together, venice, voyageai, xai,
  xiaomimimo, zai; each exports a `Client` plus model types.
- Architecture (`upstream/crates/rig-core/src/client/mod.rs`): one
  `Provider` trait plus one `Has*` trait per capability — `HasCompletion`,
  `HasEmbeddings`, `HasRerank`, `HasTranscription`, `HasModelListing`,
  `HasImageGeneration`, `HasAudioGeneration` — over generic
  `Client<P, H = BoxedHttpClient>`; a capability a provider lacks is a trait
  it does not implement, so the missing method is a compile error.
- Model constants are `pub const ...: &str` per provider, re-exported at the
  provider root: `openai::GPT_5_2`, `openai::GPT_5_6`,
  `anthropic::completion::CLAUDE_SONNET_4_6`, `gemini::GEMINI_2_5_FLASH_IMAGE`,
  `groq::WHISPER_LARGE_V3`, `openai::WHISPER_1`, `xai::GROK_3`,
  `mistral::transcription::VOXTRAL_MINI`.
- Client construction: the facade's default `reqwest` feature brings
  `DefaultTransportClient` (`upstream/crates/rig-reqwest/src/client.rs`),
  supplying `Client::new(key)`, `Client::from_env()`, and
  `Client::builder().api_key(..).build()`. Without a transport,
  `Client::new_with(key, http)` / `.http_client(..)` is required
  (`upstream/crates/rig-core/src/client/mod.rs`). Env vars are per provider,
  read by `Client::from_env_api_key`: `OPENAI_API_KEY` (+ optional
  `OPENAI_BASE_URL`), `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `GROQ_API_KEY`,
  `MISTRAL_API_KEY`, `COHERE_API_KEY`, `DEEPSEEK_API_KEY`, `XAI_API_KEY`.
- The unified model interface is `CompletionModel`
  (`upstream/crates/rig-core/src/completion/request.rs`): `completion()`,
  `stream()`, plus `completion_request(prompt)`. OpenAI ships Responses
  (default) and Completions APIs; switch with `.completions_api()` /
  `.responses_api()` (`upstream/crates/rig-core/src/providers/openai/client.rs`).
  OpenAI-compatible providers (deepseek, groq, ...) reuse
  `GenericCompletionModel` via an `OpenAICompatibleProvider` impl
  (`upstream/crates/rig-core/src/providers/deepseek.rs`).
- Facade feature gating (`upstream/Cargo.toml` `[features]`): one feature per
  companion crate — `bedrock` → `rig::bedrock`, `candle` → `rig::candle`,
  `gemini-grpc` → `rig::gemini_grpc`, `vertexai` → `rig::vertexai`, plus
  vector stores (`lancedb`, `qdrant`, ...). `image` and `audio` gate
  `rig-core`'s `image_generation` / `audio_generation` modules (default off);
  `transcription` needs no feature.
- Local inference via candle: `rig::candle` (`upstream/crates/rig-candle`)
  exposes `CandleModel::from_gguf(ModelData { config, tokenizer, weights })`
  (`upstream/crates/rig-candle/src/model.rs`); GGUF Llama/SmolLM2 and
  tool-capable Qwen3.

## Workflow

1. Client from env, agent builder, raw completion — facade + prelude; the
   raw path is `CompletionModel::completion` under `AgentClientExt::agent`:

```rust
use ::rig::prelude::*;
use ::rig::providers::{self, openai};

let client = providers::openai::Client::from_env()?;
let agent = client
    .agent(openai::GPT_5_2)
    .preamble("You are a comedian.")
    .build();
let response = agent.prompt("Entertain me!").await?;

// raw path: client.completion_model(openai::GPT_5_2) then
// .completion_request(..).preamble(..).build() -> model.completion(request)
```

2. OpenAI Completions API fallback (example `openai_agent_completions_api_otel`):

```rust
let agent = providers::openai::Client::from_env()?
    .completion_model(openai::GPT_4O)
    .completions_api()
    .into_agent_builder()
    .preamble("You are a helpful assistant")
    .build();
```

3. Transcription (no feature gate; example `transcription`):

```rust
let response = client
    .transcription_model(openai::WHISPER_1)
    .transcription_request()
    .load_file(file_path)?
    .send()
    .await?; // response.text
```

4. Image generation — requires facade `image` feature (example
   `gemini_nanobanana_image_generation`):

```rust
let response = client
    .image_generation_model(gemini::GEMINI_2_5_FLASH_IMAGE)
    .image_generation_request()
    .prompt("a yellow banana")
    .width(512)
    .height(512)
    .send()
    .await?; // response.image: Vec<u8>
```

5. Audio generation — requires facade `audio` feature; builder takes
   `.text(..)`, `.voice(..)`, `.speed(..)`; `response.audio: Vec<u8>`
   (`upstream/crates/rig-core/src/audio_generation.rs`).

6. Local models with candle — `rig = { features = ["candle"] }`
   (example `candle_local`):

```rust
let model = ::rig::candle::CandleModel::from_gguf(::rig::candle::ModelData {
    config: ::std::fs::read(model_dir.join("config.json"))?,
    tokenizer: ::std::fs::read(model_dir.join("tokenizer.json"))?,
    weights: ::std::fs::read(model_dir.join("model.gguf"))?,
})?;
```

## Pitfalls

- `image_generation_model` / `audio_generation_model` fail to resolve unless
  the facade `image` / `audio` features are enabled — transcription is
  always on, the other two are not.
- Default OpenAI client is the Responses API; openai-compatible endpoints
  that reject top-level `instructions` need `.completions_api()`.
- `from_env()` reads the provider's specific variable (`GEMINI_API_KEY`, not
  `GOOGLE_API_KEY`); base-URL overrides like `OPENAI_BASE_URL` are optional.
- Capabilities are per-provider: compile error, not runtime error, when a
  provider lacks a `Has*` impl (e.g. no `HasImageGeneration` on deepseek).
- Provider-native response shapes stay reachable through inherent
  `raw_completion` / `raw_transcription` / `raw_image_generation` /
  `raw_audio_generation` methods; the normalized types drop those fields.
- Upstream ships breaking changes — verify every symbol against the pinned
  commit before relying on it.

## Verify

- `cargo check` a snippet against the pinned revision with the features it
  claims (e.g. `--features image`).
- Grep the constant: `rg "pub const GPT_5_2" upstream/crates/rig-core/src/providers/openai/`.
- Capability presence: grep `impl Has<ImageGeneration|AudioGeneration> for <Provider>`.

## Provenance

- Repo: 0xPlaygrounds/rig, commit 6828097, 2026-09-14.
- Upstream ships breaking changes; claims above are pinned to that commit.
