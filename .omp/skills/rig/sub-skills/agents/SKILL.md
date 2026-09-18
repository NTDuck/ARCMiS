---
name: rig-agents
description: Build and prompt rig agents — builder, blocking and streamed prompts, multi-turn chat, max turns, AgentRun stepping, and AgentHook observability.
---

# rig Agents

## Scope

Creating an LLM agent from a provider client, prompting it (blocking or
streamed), multi-turn history, turn budgets, hand-stepping the agent state
machine, or observing runs with hooks. Not for extraction, RAG, or tool
definitions themselves.

## Ground Truth

- `AgentClientExt::agent(model)` — blanket-implemented on every `CompletionClient`;
  returns `AgentBuilder::new(self.completion_model(model))`; `use ::rig::prelude::*;`
  brings both traits in scope. `upstream/crates/rig-agent/src/client.rs`
- `AgentBuilder`: `preamble`, `append_preamble`, `tool`, `dynamic_tools`,
  `temperature`, `max_tokens`, `default_max_turns`, `add_hook`, `build`.
  `upstream/crates/rig-agent/src/agent/builder.rs`
- `Agent::prompt(impl Into<Message>) -> AgentRunner`; `.await` yields
  `Result<PromptResponse, PromptError>` (`IntoFuture`). Runner overrides:
  `history`, `max_turns`, `temperature`, `max_tokens`, `add_hook`,
  `tool_context`, `run`, `stream`. `Agent::resume(run)` continues a persisted
  run — its persisted prompt/history/budgets override runner settings.
  `upstream/crates/rig-agent/src/agent/completion.rs`, `upstream/crates/rig-agent/src/agent/runner.rs`
- `PromptResponse`: `output: String`, `usage`, `completion_calls`,
  `messages: Option<Vec<Message>>`, `memory_append`. `upstream/crates/rig-agent/src/run/response.rs`
- `Agent::chat(prompt, &mut Vec<Message>)` — history in, committed messages
  written back; bypasses conversation memory. `upstream/crates/rig-agent/src/agent/completion.rs`
- `AgentRunner::stream() -> StreamingResult` =
  `Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem, StreamingError>>>>`;
  variants `StreamAssistantItem(StreamEvent)`, `ToolCall`,
  `ToolExecutionCommitted`, `FinalResponse(PromptResponse)` (has `.output()`).
  `upstream/crates/rig-agent/src/agent/streaming.rs`
- `AgentRun` — sans-IO, serializable state machine: `next_step()` →
  `AgentRunStep::{CallModel, CallTools, Done(PromptResponse)}`, fed back via
  `model_response(ModelTurn)` / `tool_results(Vec<UserContent>)`;
  `ModelTurnOutcome::{Continue, NeedsResolution, TurnRetried}`; invalid calls
  resolved via `resolve_invalid_tool_call`. `upstream/crates/rig-agent/src/run/mod.rs`
- `AgentHook` — default-implemented callbacks: `on_run_start`, `on_model_select`,
  `on_completion_call`, `on_model_turn_finished`, `on_invalid_tool_call`,
  `on_text_delta`/`on_reasoning_delta`/`on_tool_call_delta`,
  `on_dispatch`/`on_outcome` (gated by `observes`), `on_run_settled`. Actions
  like `DispatchAction::proceed()` patch, retry, or stop runs.
  `upstream/crates/rig-agent/src/agent/hook.rs`

## Workflow

1. Build from a provider client; the tool loop runs within the turn budget:
   ```rust
   use ::rig::prelude::*;
   use ::rig::providers::openai;

   let agent = ::openai::Client::from_env()?
       .agent(::openai::GPT_4O)
       .preamble("You are a calculator. Always use the provided tools.")
       .tool(Add)
       .default_max_turns(4)
       .build();

   let response = agent.prompt("Calculate 2 - 5.").max_turns(8).await?.output;
   ```
2. Multi-turn chat — committed messages append to caller-owned history:
   ```rust
   let mut history = ::std::vec::Vec::<::rig::completion::Message>::new();
   let response = agent.chat("Calculate 5 - 2.", &mut history).await?;
   ```
3. Stream a run; poll until `FinalResponse`:
   ```rust
   use ::futures::StreamExt;

   let mut stream = agent.prompt("Entertain me!").history(&history).stream();
   while let Some(item) = stream.next().await {
       if let ::rig::agent::MultiTurnStreamItem::FinalResponse(response) = item? {
           let text = response.output().to_owned();
       }
   }
   ```
4. Hand-step the state machine for approval pauses, custom tool execution, or
   cross-process resume. `AgentRun` does no IO — the driver sends the model
   request itself and executes tools under its own policy:
   ```rust
   use ::rig::agent::run::{AgentRun, AgentRunStep, ModelTurn};

   let mut run = AgentRun::new("What is 2 + 5?").max_turns(2);
   loop {
       match run.next_step()? {
           AgentRunStep::CallModel { prompt, history, turn } => {
               let response = model
                   .completion_request(prompt)
                   .messages(history)
                   .send()
                   .await?;
               let _ = run.model_response(ModelTurn::new(
                   response.message_id.clone(),
                   response.choice.clone(),
                   response.usage,
                   advertised_tools.clone(),
                   allowed_tools.clone(),
               ))?;
           }
           AgentRunStep::CallTools { calls } => {
               // Serialize `run` here to pause; resume later/elsewhere.
               let results = /* execute `calls`, skip `preresolved_result` */;
               run.tool_results(results)?;
           }
           AgentRunStep::Done(response) => break,
       }
   }
   ```
   On `ModelTurnOutcome::NeedsResolution(context)`, answer with
   `run.resolve_invalid_tool_call(::rig::agent::run::InvalidToolCallAction::fail())?`
   (or retry/repair/skip) before the next `next_step()`.
5. Hooks — attach to the builder or the runner; hooks fire at every observable
   point of `run()`/`stream()` alike:
   ```rust
   use ::rig::agent::{AgentHook, DispatchAction, DispatchEvent, HookContext};

   struct ToolLoggerHook;

   impl ::rig::agent::AgentHook for ToolLoggerHook {
       async fn on_dispatch(&self, _ctx: &HookContext, event: DispatchEvent<'_>)
           -> DispatchAction
       {
           if let (Some(name), Some(args)) = (event.tool_name(), event.tool_args()) {
               ::std::println!("[hook] tool call: {name}({args})");
           }
           ::rig::agent::DispatchAction::proceed()
       }
   }

   let response = agent.prompt("What is 2 + 5?").max_turns(2)
       .add_hook(ToolLoggerHook).run().await?;
   ```

## Pitfalls

- `.await` on the runner equals `.run()`; `.stream()` is separate — both share
  the same loop and hook events except streamed deltas.
- `AgentRun` stepping never uses the agent's tools, hooks, or preamble — the
  driver must send the completion request itself.
- Calls suppressed by invalid-tool-call recovery arrive in `CallTools` with
  `preresolved_result: Some(..)` — return that content, do not execute.
- A resumed run ignores runner `.history()`/`.max_turns()`; pending tool calls
  re-execute on resume.
- Streaming `ToolExecutionCommitted` surfaces only after the whole tool batch
  settles — not a real-time start event.
- Default turn budget is small; tool-heavy tasks need `default_max_turns`
  (builder) or per-run `.max_turns(n)` (runner).

## Verify

- Construct an agent per Workflow 1, prompt it; assert the tool executed and
  `response.output` is non-empty.
- Streaming: assert a `FinalResponse` item arrived before the stream ended.
- Stepping: serialize an `AgentRun` at `CallTools`, deserialize, assert the
  resumed run re-emits the same pending calls from `next_step()`.

## Reference Examples (upstream `examples/`)

Runnable, idiomatic usage of this sub-skill's API surface. Captured from
upstream `main` at commit `9b94481` (2026-09-17) — newer than the Ground
Truth pin. Treat example APIs as the current idiom and re-verify against
your rig version.

| Example | Demonstrates |
| --- | --- |
| `agent` | Smallest client → agent → prompt flow. |
| `agent_stream_chat` | Streamed run over prior history: `prompt(..).history(..).stream()`. |
| `multi_turn_agent` | Sequential prompts with typed tools (Anthropic). |
| `multi_turn_agent_extended` | Multi-turn arithmetic chat with a larger tool set. |
| `agent_with_context` | Small context documents passed directly to the agent. |
| `agent_with_loaders` | `FileLoader` (with glob) documents in agent context. |
| `agent_with_default_max_turns` | Builder turn budget for tool-heavy prompts. |
| `agent_run_stepping` | Hand-driven `AgentRun` state machine plus runner-with-hooks. |
| `agent_no_tokio` | `run_channel` on `bevy_tasks` with an erased `BoxedHttpClient`. Depends on neither tokio nor reqwest. |
| `agent_with_retry_hook` | Retry policy in the `HookContext` scratchpad. Retries consume the turn budget. |
| `request_hook` | Stacked hooks all run. `RequestPatch::extra_context` injects context per turn. |
| `openai_streaming_per_call_usage` | Per-completion-call usage from a stream. |
| `discord_bot` | Agent deployed as a Discord bot (own workspace — run with `--manifest-path`). |
| `agent_with_memory_streaming` | Rig-managed memory plus streaming. Also under memory. |
| `agent_with_tools_otel` | Multi-turn tool run traced to an OTel collector. |
| `openai_streaming_with_tools_otel` | Streaming a tool-using run to stdout, traced to OTel (`stream_to_stdout`). |

## Provenance

- Repo: `0xPlaygrounds/rig`, commit `6828097`, 2026-09-14 (facade rig
  0.36.0-era).
- Upstream ships breaking changes frequently — re-verify symbol names and
  signatures against the checked-out revision before relying on this skill.
