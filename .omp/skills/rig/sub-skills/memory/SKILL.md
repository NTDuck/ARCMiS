---
name: rig-memory
description: Give rig agents conversation memory — core memory traits, in-memory backend, rig-memory history policies, and memory wiring on agent builders.
---

# rig Memory

## Scope

Use when an agent must recall prior conversation turns: wiring a memory backend
into an agent builder, prompting under a conversation id, shaping loaded
history, or demoting/compacting truncated turns. Not for caller-managed
history (`chat`/`history`) or vector stores.

## Ground Truth

- `rig::memory` is **always available**: the facade re-exports
  `::rig_core::memory::*`; the `memory` feature adds `::rig_memory::*` to the
  same module. `upstream/src/lib.rs`
- Core traits and backend (`::rig::memory`, no feature needed) —
  `upstream/crates/rig-core/src/memory.rs`: `ConversationMemory` with
  `load`/`append`/`clear(&ConversationId) -> Result<..>` and forwarding
  impls for `Arc<M>`/`Box<M>`; `InMemoryConversationMemory::new()`
  (thread-safe `HashMap`, lost on restart) with `with_filter(F)` applying an
  `Fn(Vec<Message>) -> Vec<Message>` filter on every `load`;
  `MemoryError::{Backend, Policy, Internal}` + `::backend(source)` ctor;
  `DemotionHook::on_demote` + `NoopDemotionHook`; `Compactor` with
  `Artifact: Into<Message> + Clone` and `compact(id, evicted, carry_over)`.
- Policies (`::rig_memory`, facade feature `memory`) —
  `upstream/crates/rig-memory/src/lib.rs`:
  - `MemoryPolicy::apply(Vec<Message>) -> Result<Vec<Message>, MemoryError>`;
    truncating policies override `apply_with_demoted` returning `(kept,
    demoted)` — `demoted` is the dropped prefix, in order.
  - `IntoFilter::into_filter()` yields a closure for
    `InMemoryConversationMemory::with_filter`.
  - `NoopMemoryPolicy`; `SlidingWindowMemory::last_messages(n)`;
    `TokenWindowMemory::new(max_tokens, counter)` with `TokenCounter::count`
    (also implemented for `Fn(&Message) -> usize` closures);
    `HeuristicTokenCounter::new(bytes_per_token, per_message_overhead,
    per_attachment_tokens)` — `::anthropic()` preset (3.5/4/256), `Default`
    (4.0/4/256 OpenAI/Gemini rule of thumb); heuristic only.
  - Adapters (all implement `ConversationMemory`): `PolicyMemory::new(inner,
    policy)` propagates policy errors as `MemoryError::Policy`;
    `DemotingPolicyMemory::new(inner, policy, hook)` feeds demoted messages to
    the hook; `CompactingMemory::new(inner, policy, compactor)` splices
    `[summary, ...kept_window]`; both expose `forget(&id)`/
    `tracked_conversations()`.
  - `TemplateCompactor::new()` / `with_header(..)` / `with_max_bytes(..)`
    produces a `TextSummary` artifact rendered as `Message::System`.
- Agent wiring: `AgentBuilder::memory(B: ConversationMemory + 'static)`
  registers a `MemoryAdapter` under the agent's `<owner>/memory` key;
  `memory_handler(handler)` serves memory from any Serve handler
  (`upstream/crates/rig-agent/src/agent/builder.rs`).
  `AgentRunner::conversation(id: impl Into<ConversationId>)` (also on the
  builder) loads history before the prompt and appends the committed transcript
  after the run; explicit `history(..)` bypasses both and `without_memory()`
  disables it (`upstream/crates/rig-agent/src/agent/runner.rs`).
  `PromptResponse::memory_append: Option<MemoryAppend>` —
  `MemoryAppend::{Acknowledged, Failed { report }}`, `is_acknowledged()`,
  `failure()`; `None` when memory was not configured, bypassed, disabled, or
  the run was resumed (`upstream/crates/rig-agent/src/run/response.rs`).
  Streaming is identical; `MultiTurnStreamItem::FinalResponse` carries it
  (`upstream/examples/agent_with_memory_streaming/src/main.rs`).

## Workflow

1. Wire memory into an agent and prompt under a conversation id
   (`upstream/examples/agent_with_memory/src/main.rs`):
   ```rust
   let agent = ::rig::providers::openai::Client::from_env()?
       .agent(::rig::providers::openai::GPT_4O)
       .preamble("You are a helpful assistant with persistent memory.")
       .memory(::rig::memory::InMemoryConversationMemory::new())
       .build();
   let first = agent.prompt("My name is Alice.")
       .conversation("user-123").await?.output;
   let second = agent.prompt("What's my name?")
       .conversation("user-123").await?.output;
   ```

2. Shape history with a named policy (facade feature `memory`):
   ```rust
   let memory = ::rig_memory::InMemoryConversationMemory::new()
       .with_filter(::rig_memory::SlidingWindowMemory::last_messages(20)
           .into_filter());
   ```

3. Wrap any backend (adapters wrap any `ConversationMemory`):
   ```rust
   let memory = ::rig_memory::CompactingMemory::new(
       backend,
       ::rig_memory::TokenWindowMemory::new(
           8_000, ::rig_memory::HeuristicTokenCounter::default(),
       ),
       ::rig_memory::TemplateCompactor::new().with_max_bytes(4 * 1024),
   );
   ```

## Pitfalls

- `into_filter()` swallows policy errors (`tracing::warn!`, unfiltered
  history returned); use `PolicyMemory` when a policy error must surface.
- A **load** failure fails the run (`PromptError::MemoryError`) before any
  model call; an **append** failure does not — the answer stands and
  `memory_append` reports `Failed`. No retries, no exactly-once writes.
- A window that opens on an orphan tool result lacks its assistant tool call
  (providers reject that); sliding policies demote the leading tool-result.
- `CompactingMemory` splices the summary *outside* the policy budget, so a
  token-budgeted prompt can exceed budget. `TemplateCompactor` grows
  monotonically unless bounded via `with_max_bytes`.
- `DemotionHook`/`Compactor` implementations must be idempotent per
  `(conversation_id, messages)`: delivery watermarks are in-process only and
  replay after a restart. Wrapper state maps grow per conversation — call
  `forget(&id)` when one ends.
- `memory_append` is `None` for resumed runs — the persisting driver owns the
  append. `HeuristicTokenCounter` is heuristic only; implement `TokenCounter`
  (closure impls work) with a real tokenizer for accuracy.
- Streaming with memory: Workflow 1's shape with `.stream()` after
  `.conversation(id)`; poll for the `FinalResponse` carrying `memory_append`.

## Verify

- No API key needed: construct `InMemoryConversationMemory::new()`, `append`
  two `Message`s under `"c-1".into()`; assert `load` returns them, `clear`
  empties, and a `last_messages(1)` filter keeps only the newest message.
- Build a bin per Workflow 1 against a live provider; assert turn 2 recalls
  turn 1's fact and `memory_append.is_acknowledged()` is true.

## Reference Examples (upstream `examples/`)

Runnable, idiomatic usage of this sub-skill's API surface. Captured from
upstream `main` at commit `9b94481` (2026-09-17) — newer than the Ground
Truth pin. Treat example APIs as the current idiom and re-verify against
your rig version.

| Example | Demonstrates |
| --- | --- |
| `agent_with_memory` | `.memory(InMemoryConversationMemory::new())` + `.conversation(id)` per prompt. |
| `agent_with_memory_streaming` | Same wiring with `.stream()`. The `FinalResponse` item carries `memory_append`. |

## Provenance

- Repo: `0xPlaygrounds/rig`, commit `6828097`, 2026-09-14 (facade `rig`
  0.42.0, `rig-core` + `rig-memory`). Upstream ships breaking changes
  frequently — re-verify symbols against the checked-out revision.
