---
name: rig-rag-vector
description: Build rig RAG pipelines — EmbeddingsBuilder, vector store traits, in-memory index, 12 store feature flags, rerank, dynamic tool retrieval.
---

# rig RAG & Vector Search

Embedding documents, storing them in a vector store, querying an index,
attaching retrieval to an agent (dynamic context / retrieved tools), reranking,
and implementing a custom store backend. Not for agent basics or extraction.

## Ground Truth

- `VectorStoreIndex` (query trait): `type Filter: SearchFilter`; `top_n<T:
  DeserializeOwned>(VectorSearchRequest<Self::Filter>) -> Vec<(f64, String, T)>`
  and `top_n_ids(..) -> Vec<(f64, String)>` (score, id, doc — best first)
  `upstream/crates/rig-core/src/vector_store/mod.rs:133-149`. Companion
  `InsertDocuments::insert_documents(Vec<(Doc, Vec<Embedding>)>)` — every
  document must carry ≥1 embedding, implementors do not guard it
  (`upstream/crates/rig-core/src/vector_store/mod.rs:115-130`).
- `VectorSearchRequest::builder()` — `.query(..)` and `.samples(u64)` required
  (typestate); `.threshold(f64)`, `.filter(f)`, `.additional_params(json)`
  optional. `upstream/crates/rig-core/src/vector_store/request.rs:445-508`
- `SearchFilter` trait: `eq`, `gt`, `lt`, `and`, `or` (tagless final — call
  methods directly, inference picks the backend type). Canonical `Filter<V>`
  enum has the same variants
  (`upstream/crates/rig-core/src/vector_store/request.rs:117-125,284-293`).
- Every JSON-valued-filter `VectorStoreIndex` blanket-implements `PortableTool`
  (tool name `search_vector_store`, args = the request, output =
  `Vec<VectorStoreOutput>{score, id, document}`) — expose an index to the model
  as a tool for free. `upstream/crates/rig-core/src/vector_store/mod.rs:161-212`
- Embedding models: `EmbeddingsClient::embedding_model(model)` /
  `embedding_model_with_ndims(model, ndims)` on every provider client.
  `upstream/crates/rig-core/src/client/embeddings.rs:14-45`
- `EmbeddingModel` trait: `max_documents()`, `ndims()`, `embed_texts_response`
  (the method providers implement), derived `embed_texts`, `embed_text`.
  `upstream/crates/rig-core/src/embeddings/embedding.rs:100-170`
- `EmbeddingsBuilder::new(model).documents(..)?.build().await` →
  `Vec<(Doc, Vec<Embedding>)>` in insertion order (documented load-bearing
  guarantee). `upstream/crates/rig-core/src/embeddings/builder.rs:57-139`
- `Embed` trait: `fn embed(&self, embedder: &mut TextEmbedder)`; derive
  `#[derive(Embed)]` (feature `derive`, on by default), tag fields `#[embed]`;
  `Vec<T>` embeds element-wise (`upstream/crates/rig-core/src/embeddings/embed.rs:65-67,186-190`,
  `upstream/crates/rig-derive/src/lib.rs:28-30`).
- `InMemoryVectorStore<D>::from_documents(embeddings)` (auto ids `doc{n}`),
  `from_documents_with_id_f(embeddings, |doc| id)`; `.index(model)` →
  `InMemoryVectorIndex` (`Filter = Filter<serde_json::Value>`)
  (`upstream/crates/rig-core/src/vector_store/in_memory_store.rs:84-108,378-380,431-436`).
- Builder: `InMemoryVectorStore::builder().documents(..)
  .index_strategy(IndexStrategy::BruteForce | LSH { .. }).build()` — default
  BruteForce (`upstream/crates/rig-core/src/vector_store/builder.rs:26-88`).
- Vector-store integration features of the `rig` facade (each enables the
  matching companion crate): `lancedb`, `qdrant`, `postgres`, `mongodb`,
  `sqlite`, `milvus`, `neo4j`, `scylladb`, `surrealdb`, `s3vectors`,
  `vectorize`, `helixdb`. `upstream/Cargo.toml:396-408`, table in
  `upstream/README.md:161-178`
- Reranking: `RerankModel::rerank(query, documents) -> RerankResponse` with
  `RerankResult { index, document, relevance_score }`. Score is only
  order-comparable within one response — providers disagree on range (Voyage
  AI 0..1, llama.cpp raw logits, negatives normal).
  `upstream/crates/rig-core/src/rerank.rs:22-61`
- `RerankingClient::rerank_model(model)` where the provider implements
  `HasRerank` — in-tree: `voyageai`, `llamacpp`
  (`upstream/crates/rig-core/src/client/rerank.rs:5-36`).
- Agent integration (rig-agent): `.dynamic_context(samples, index)` injects
  top docs into each prompt's context; `.retrieved_tools(sample, index,
  toolset)` picks tools per prompt by embedding similarity.
  `upstream/crates/rig-agent/src/agent/builder.rs:208-233,684-778`
- Dynamic tool RAG: implement `ToolEmbedding` (`InitError/Context/State`,
  `embedding_docs()`, `context()`, `init(state, context)`), register with
  `ToolSet::add_retrieved_tool`, embed `toolset.schemas()?`, index by tool name
  (`upstream/crates/rig-core/src/tool/contextual.rs:216-230`,
  `upstream/crates/rig-agent/src/tool/registry.rs:353-358,506-518`).
- `prelude` re-exports `VectorStoreIndex`, `InMemoryVectorStore`,
  `VectorSearchRequest`. `upstream/crates/rig-core/src/prelude.rs:36-39`

## Workflow

1. Derive `Embed` on the document type, tagging retrieval fields with `#[embed]`:

```rust
#[derive(::rig::Embed, ::serde::Serialize, Clone, Debug, Eq, PartialEq, Default)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,
}
```

2. Embed, store, index:

```rust
let openai_client = ::rig::providers::openai::Client::from_env()?;
let embedding_model = openai_client.embedding_model(::rig::providers::openai::TEXT_EMBEDDING_ADA_002);

let embeddings = ::rig::embeddings::EmbeddingsBuilder::new(embedding_model.clone())
    .documents(sample_documents())?
    .build()
    .await?;

let vector_store = ::rig::vector_store::in_memory_store::InMemoryVectorStore::from_documents(embeddings);
let index = vector_store.index(embedding_model);
```

3. Query directly:

```rust
let req = ::rig::vector_store::request::VectorSearchRequest::builder()
    .query("what currency can I use?")
    .samples(1)
    .build();

let hits = index.top_n::<WordDefinition>(req).await?; // Vec<(score, id, doc)>
```
4. Passive RAG — inject context each prompt:
   `agent(model).preamble(..).dynamic_context(1, index).build()`.
5. Active RAG — let the model search: pass the index as a tool (blanket
   `PortableTool`); or `.retrieved_tools(1, index, toolset)` for dynamic tools.
6. Rerank: `client.rerank_model(model).rerank(query, docs).await?`, sort by
   `relevance_score` descending, compare only within one response.

## Pitfalls

- `#[embed]` on an empty `Vec` embeds nothing — `EmbeddingsBuilder::build`
  fails the whole build naming the document
  (`upstream/crates/rig-core/src/embeddings/builder.rs:124-135`).
- `InsertDocuments` requires ≥1 embedding per document; hand-built tuples with
  empty lists silently misbehave — some stores insert nothing, some insert
  unfindable rows (`upstream/crates/rig-core/src/vector_store/mod.rs:118-125`).
- An index is bound to its embedding model for life; an index populated under
  one model is meaningless under another (`upstream/crates/rig-core/src/vector_store/in_memory_store.rs:395-402`).
- Use `embedding_model_with_ndims` when the model's dimensionality is
  nonstandard, otherwise search silently mismatches.
- `relevance_score` is not a probability: do not threshold at 0.5 or compare
  across providers (`upstream/crates/rig-core/src/rerank.rs:46-60`).
- `InMemoryVectorStore` ids default to `doc{n}` by insertion order — use
  `from_documents_with_id_f` when you need stable, meaningful ids.
- Non-JSON-valued filters need the `Filter: SearchFilter<Value = Value> +
  DeserializeOwned` bound to get the `PortableTool` blanket impl
  (`upstream/crates/rig-core/src/vector_store/mod.rs:161-167`).

## Verify

- Compile-check a minimal pipeline: build embeddings for two `#[derive(Embed)]`
  docs, query `top_n::<T>(req)`, assert the expected id ranks first.
- For dynamic tools, register two `ToolEmbedding` tools, prompt for one
  operation, assert its tool executed (sample-rate 1 exposes only the
  retrieved tool).
- Rerank smoke test: assert `rerank(..)` returns results sorted by score.

## Reference Examples (upstream `examples/`)

Runnable, idiomatic usage of this sub-skill's API surface. Captured from
upstream `main` at commit `9b94481` (2026-09-17) — newer than the Ground
Truth pin. Treat example APIs as the current idiom and re-verify against
your rig version.

| Example | Demonstrates |
| --- | --- |
| `rag` | `Embed` derive, `EmbeddingsBuilder`, `InMemoryVectorStore`, `dynamic_context` agent. Also under extraction. |
| `chain` | Manual retrieve-then-fold pipeline (`top_n` lookup before prompting). |
| `vector_search` | Direct index query. Compares `top_n` with `top_n_ids`. |
| `pdf_agent` | `PdfFileLoader` documents into a RAG chatbot (Ollama). Uses the shared `documents/` sample-PDF dir. |
| `vector_search_ollama` | Local Ollama embeddings against the in-memory index. |
| `vector_search_cohere` | Separate Cohere document and query embedding models. |
| `cohere_image_embeddings` | Embedding an image with Cohere Embed v3. |
| `custom_vector_store` | Redis-backed `VectorStoreIndex` implementation template. |
| `rag_ollama` | RAG pipeline fully on a local Ollama stack. |
| `gemini_extractor_with_rag` | RAG-grounded extraction (also under extraction). |
| `complex_agentic_loop_claude` | RAG + `ThinkTool` builtin in a Claude agentic loop. |

## Provenance

- Repo: `0xPlaygrounds/rig`, commit `6828097`, 2026-09-14.
- Upstream ships breaking changes frequently — re-verify symbol names and
  signatures against the checked-out revision before relying on this skill.
