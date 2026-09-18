---
name: rig-multi-agent
description: Compose rig multi-agent topologies — agent-as-tool, orchestrator, routing, parallelization, chaining, evaluator-optimizer, and debate.
---

# rig Multi-Agent Patterns

## Scope

Use when composing several rig agents into one system: agent-as-tool
delegation, routing, concurrent sub-agents, prompt chaining, orchestrator,
evaluator-optimizer, and debate. Single-agent basics are in sibling skills.

## Ground Truth

- `AgentClientExt::agent(model)` / `AgentClientExt::extractor::<T>(model)` — blanket-
  implemented on every `CompletionClient`; `use ::rig::prelude::*;` brings both in
  scope. `upstream/crates/rig-agent/src/client.rs`
- `Agent::into_tool() -> DynamicTool` wraps an agent as a tool taking `{ prompt: String }`
  (`impl From<Agent> for DynamicTool` also exists); register with
  `AgentBuilder::dynamic_tool(..)`.
  `upstream/crates/rig-agent/src/agent/tool.rs`, `upstream/crates/rig-agent/src/agent/builder.rs`
- Manual `::rig::tool::Tool` impl wrapping an agent: implement `const NAME`,
  `type Error/Args/Output`, `description`, `parameters` (JSON schema), and
  `call(&self, &mut ::rig::tool::ToolContext, args)`; inside `call` run
  `self.0.chat(args.prompt, &mut history).await` with an empty history vec.
  `upstream/examples/multi_agent/src/main.rs`
- `Agent::prompt(..).await` yields `PromptResponse` (`output: String`,
  `messages: Option<Vec<Message>>`); `.history(&h)` seeds, `.max_turns(n)` budgets.
  `upstream/crates/rig-agent/src/agent/runner.rs`, `upstream/crates/rig-agent/src/run/response.rs`
- `Extractor::<T>::extract(text) -> TypedRun<T>` is an `IntoFuture` (import
  `::std::future::IntoFuture`); awaiting yields `TypedPromptResponse<T>` with `output: T`.
  Extraction runs an internal `submit` output tool with `max_turns(1)` — one call, one turn.
  `ExtractorBuilder`: `append_preamble`, `retries(u64)`, `build`.
  `upstream/crates/rig-agent/src/extractor.rs`, `upstream/crates/rig-agent/src/agent/typed.rs`
- Concurrency is ordinary Tokio: `::futures::join!(a, b, c)` awaits all and keeps each
  `Result`; `try_join!` short-circuits on the first error.
  `upstream/examples/agent_parallelization/src/main.rs`

## Workflow

1. **Agent-as-tool (typed, preferred).** The outer model decides when to delegate
   (`upstream/examples/agent_with_agent_tool/src/main.rs`):
   ```rust
   use ::rig::prelude::*;
   use ::rig::providers::openai;
   let client = ::openai::Client::from_env()?;
   let calculator = client.agent(::openai::GPT_4O)
       .preamble("You are a calculator. Use the provided tools.")
       .default_max_turns(2).build();
   let outer = client.agent(::openai::GPT_4O)
       .preamble("Solve problems. Delegate arithmetic via the tool.")
       .default_max_turns(2).dynamic_tool(calculator.into_tool()).build();
   let answer = outer.prompt("Calculate 2 - 5").await?.output;
   ```
2. **Agent-as-tool (manual `Tool` impl).** For custom args, name, or post-processing:
   ```rust
   struct TranslatorTool(::rig::agent::Agent);
   impl ::rig::tool::Tool for TranslatorTool {
       const NAME: &'static str = "translator";
       type Error = ::rig::completion::PromptError;
       type Args = TranslatorArgs;
       type Output = ::std::string::String;
       fn description(&self) -> ::std::string::String { "Translate to English.".into() }
       fn parameters(&self) -> ::serde_json::Value { ::serde_json::json!({ /* schema */ }) }
       async fn call(&self, _context: &mut ::rig::tool::ToolContext, args: Self::Args)
           -> ::core::result::Result<Self::Output, Self::Error> {
           let mut empty_history = ::std::vec::Vec::<::rig::completion::Message>::new();
           Ok(self.0.chat(args.prompt, &mut empty_history).await?.output)
       }
   }
   // .tool(TranslatorTool(translator_agent)) on the outer builder
   ```
3. **Routing.** The classifier's plain-text output selects the follow-up; validate
   (`upstream/examples/agent_routing/src/main.rs`):
   ```rust
   let category = router_agent.prompt(INPUT).await?.output;
   let follow_up = match category.trim() {
       "sheep" => "Calculate 5+5. Return only the number.",
       other => ::anyhow::bail!("could not process category: {other}"),
   };
   let response = worker_agent.prompt(follow_up).await?.output;
   ```
4. **Prompt chaining.** Feed one agent's `output` (trimmed) as the next
   agent's prompt — sequential composition for deterministic pipelines.
   `upstream/examples/agent_prompt_chaining/src/main.rs`
5. **Parallelization.** Extractors return `TypedRun` (`IntoFuture`); join them:
   ```rust
   use ::std::future::IntoFuture;
   let (a, b, c) = ::futures::join!(
       score_agent_a.extract(statement).into_future(),
       score_agent_b.extract(statement).into_future(),
       score_agent_c.extract(statement).into_future(),
   );
   ```
6. **Orchestrator–worker–judge.** An `extractor::<Plan>` agent decomposes the task into
   structured subtasks; workers each `extract` one; a judge `extractor` picks the best
   (feed results back via `::serde_json::to_string_pretty`). `upstream/examples/agent_orchestrator/src/main.rs`
7. **Evaluator–optimizer.** Loop: generator `.prompt(..)`, then an `extractor::<Evaluation>`
   with a `Pass/NeedsImprovement/Fail` enum plus `feedback: String`; break on `Pass`,
   else re-prompt with the feedback. `upstream/examples/agent_evaluator_optimizer/src/main.rs`
8. **Debate across models.** Two agents with opposed preambles exchange outputs for
   `n` rounds; keep one `Vec<::rig::completion::Message>` per side and pass it via
   `.history(&history)`. `upstream/examples/debate/src/main.rs`

## Pitfalls

- `TypedRun` implements `IntoFuture`, not `Future` — import `::std::future::IntoFuture`
  to `.await` (or `join!`) an `Extractor::extract`.
- Extractors run `max_turns(1)` with an internal `submit` output tool — no tool loop
  happens inside a single extraction.
- `Agent::into_tool()` yields a `DynamicTool` with `{ prompt }` args only — for
  structured context, write a manual `Tool` impl (Workflow 2).
- Give inner and outer agents explicit `default_max_turns`; default budgets are small
  and delegation chains hit `PromptError::MaxTurnsError`.
- A wrapped agent's `chat` into a fresh empty history is stateless per call; thread
  history yourself if delegation must remember.
- `::futures::join!` (not `try_join!`) keeps each `Result` when partial results are
  still useful.
- Debate/history relies on `PromptResponse.messages` being `Some` — `unwrap_or_default()`
  defensively as the example does.

## Verify

- Two-agent bin (Workflow 1): assert the outer agent used the inner tool (log inside the
  `Tool` impl or a hook) and the answer is correct.
- Routing: one prompt per category plus a garbage prompt; assert the bail error.
- Parallelization: `::futures::join!` three extractors; assert all three
  `TypedPromptResponse`s parse.
- Evaluator-optimizer: mock a failing first generation; assert the loop re-prompts.

## Reference Examples (upstream `examples/`)

Runnable, idiomatic usage of this sub-skill's API surface. Captured from
upstream `main` at commit `9b94481` (2026-09-17) — newer than the Ground
Truth pin. Treat example APIs as the current idiom and re-verify against
your rig version.

| Example | Demonstrates |
| --- | --- |
| `agent_with_agent_tool` | `Agent::into_tool()` delegation (typed wrapper). |
| `multi_agent` | Manual `Tool` impls wrapping sub-agents in a chatbot. |
| `agent_routing` | Classifier output selects the follow-up agent/prompt. |
| `agent_parallelization` | `join!`/`try_join!` over concurrent extractors. |
| `agent_prompt_chaining` | Two agents in sequence, output → prompt. |
| `agent_orchestrator` | Extractor-built plan, workers, judge over serialized results. |
| `agent_evaluator_optimizer` | Extracted `Pass/NeedsImprovement/Fail` feedback loop. |
| `debate` | Opposed preambles exchanging `history` per round, cross-provider. |
| `reasoning_loop` | Extractor derives reasoning steps, executor runs them, extractor evaluates. |
| `enum_dispatch` | Multi-provider agent registry behind one enum + prompt facade. |

## Provenance

- Repo: `0xPlaygrounds/rig`, commit `6828097`, date 2026-09-14.
- Upstream ships breaking changes frequently — re-verify symbol names and
  signatures against the checked-out revision before relying on this skill.
