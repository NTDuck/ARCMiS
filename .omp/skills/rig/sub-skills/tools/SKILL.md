---
name: rig-tools
description: Define, register, and control rig tools — Tool trait, dynamic tools, manual calls, forcing tool use, approvals, and result outcomes.
---

# rig Tools

## Scope

Use when implementing rig LLM tools (typed `Tool` impls, `#[rig_tool]` derive, runtime `DynamicTool`s), registering them on an agent builder or `ToolSet`, calling tools manually, forcing tool use, gating calls with approval hooks, or classifying tool failures. Not for prompting itself, RAG pipelines, extraction.

## Ground Truth

- `Tool` trait: `const NAME: &'static str`; `type Args: Deserialize + Send + Sync`; `type Output: IntoToolOutput` (any owned `Serialize` value implements it); `type Error: Error + Send + Sync + 'static`; `description() -> String`; `parameters() -> serde_json::Value` (JSON Schema); default `map_error(&self, error) -> ToolExecutionError`; `async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error>`. `upstream/crates/rig-core/src/tool/contextual.rs`
- `ToolOutput`: `::json(Value)`, `::text(impl Into<String>)`, `::one(ToolResultContent)`; `as_content`, `into_content`, `render`. `upstream/crates/rig-core/src/tool/output.rs`
- `ToolContext`: inbound slots `insert`/`get`/`require`/`remove<T: ContextValue>`; host-only result metadata `insert_result`/`result`/`require_result` (never sent to the model). `ContextValue` = one `const KEY: &'static str`; derive via `#[derive(::rig::ContextValue)]` + `#[context(key = "...")]`. `upstream/crates/rig-core/src/tool/context.rs`
- `ToolExecutionError`: constructors `invalid_args`, `timeout`, `cancelled`, `not_found`, `permission_denied`, `rate_limited`, `provider`, `network`, `other`, `refused`; builders `with_model_feedback`, `with_model_output`, `with_code`, `with_retryable`, `with_source`. `ToolResult`: `output()`, `error()`, `refusal()`, `is_success()`, `status_name()`. `upstream/crates/rig-core/src/tool/result.rs`
- `#[rig_tool]` derive: generates a `Tool` from a plain function; `#[rig_tool(description = "...", params(arg = "..."))]`; non-`Option` params required, `Option<T>` optional. `::rig::rig_tool` re-export. `upstream/crates/rig-derive/src/lib.rs`
- Builder registration: `.tool(T)`, `.dynamic_tool(t)` / `.dynamic_tools(Vec<DynamicTool>)`, `.portable_dynamic_tool(t)`, `.retrieved_tools(sample, index, toolset)` (RAG; `sample` = tools added per prompt), `.retrieved_tools_handler(...)`, `.tool_server_handle(h)`. `upstream/crates/rig-agent/src/agent/builder.rs`
- `ToolSet`: `default()`, `from_tools`, `add_tool(T: Tool)`, `add_dynamic_tool(DynamicTool)`, `add_retrieved_tool(T: ToolEmbedding) -> Result<String, serde_json::Error>`, `add_tools`, `remove_tool`, `tool_definitions()`, `schemas()`, `execute(name, args, &mut ToolContext) -> ToolResult`. `upstream/crates/rig-agent/src/tool/registry.rs`
- `DynamicTool::new(name, description, parameters_json, callback)` — callback gets `(&mut ToolContext, serde_json::Value)`, returns `Result<ToolOutput, ToolExecutionError>` as a pinned boxed future. `upstream/crates/rig-core/src/tool/contextual.rs`
- `ToolEmbedding: Tool` adds `type InitError`, `type Context` (serializable), `type State`, `embedding_docs()`, `context()`, `init(state, context) -> Result<Self, InitError>`. `upstream/crates/rig-core/src/tool/contextual.rs`
- `ToolChoice`: `Auto` (default), `None`, `Required`, `Specific { function_names }`. `upstream/crates/rig-core/src/completion/message.rs`
- Hooks: `on_completion_call` → `CompletionCallAction::patch(RequestPatch::new().tool_choice(..))`; `on_dispatch` (every effect family) → `DispatchAction::proceed()`/`skip(reason)` — denial reason is fed to the model as the tool result; `on_outcome` sees `OutcomeEvent` with `tool_result()`, `tool_context()`, `block_id`; `HookContext::turn()` is 1-based; `ctx.scratchpad()` shares run-scoped state; hooks attach via `.add_hook(h)` on the runner. `upstream/crates/rig-agent/src/agent/hook.rs`

## Workflow

1. Typed tool — metadata, one async `call`; register on the builder (the tool loop runs inside the turn budget):

   ```rust
   impl ::rig::tool::Tool for Add {
       const NAME: &'static str = "add";
       type Error = MathError;
       type Args = AddArgs;
       type Output = i64;

       fn description(&self) -> String { "Add x and y together".to_string() }

       fn parameters(&self) -> ::serde_json::Value {
           ::serde_json::json!({
               "type": "object",
               "properties": {
                   "x": { "type": "number", "description": "First addend" },
                   "y": { "type": "number", "description": "Second addend" }
               },
               "required": ["x", "y"]
           })
       }

       async fn call(&self, _context: &mut ::rig::tool::ToolContext, args: Self::Args)
           -> Result<Self::Output, Self::Error> {
           Ok(args.x + args.y)
       }
   }

   let agent = client.agent(model)
       .preamble("Use the tools for arithmetic.")
       .tool(Add)
       .dynamic_tools(runtime_tools()) // Vec<::rig::tool::DynamicTool>
       .default_max_turns(2)
       .build();
   ```

2. Runtime-defined tool over raw JSON args:

   ```rust
   ::rig::tool::DynamicTool::new("add", "Add x and y", parameters, |_context, args| {
       ::std::boxed::Box::pin(async move {
           let args: OperationArgs = ::serde_json::from_value(args)
               .map_err(|e| ::rig::tool::ToolExecutionError::invalid_args(e.to_string()).with_source(e))?;
           Ok(::rig::tool::ToolOutput::json(::serde_json::json!(args.x + args.y)))
       })
   })
   ```

3. Manual tool calls below the agent — raw request, `ToolSet` execution, results fed back as user tool-result messages:

   ```rust
   let mut tools = ::rig::tool::ToolSet::default();
   tools.add_tool(Add);
   let mut request = model.completion_request(prompt)
       .preamble(preamble.into())
       .messages(history.clone())
       .tools(tools.tool_definitions());
   let response = request.send().await?;
   let result = tools
       .execute(&call.function.name, ::serde_json::to_string(&call.function.arguments)?, &mut ::rig::tool::ToolContext::new())
       .await;
   let result_content = ::rig::message::UserContent::tool_result_for(
       call.id.clone(), call.provider.clone(), call.function.name.clone(),
       result.output().clone().into_content(),
   );
   ```

4. Force a tool call on turn 1 only — a `RequestPatch` is per-turn and non-sticky, so unpatched later turns return to `Auto`:

   ```rust
   impl ::rig::agent::AgentHook for ForceToolOnFirstTurn {
       async fn on_completion_call(&self, ctx: &::rig::agent::HookContext,
           _event: ::rig::agent::CompletionCallEvent<'_>) -> ::rig::agent::CompletionCallAction {
           if ctx.turn() == 1 {
               ::rig::agent::CompletionCallAction::patch(
                   ::rig::agent::RequestPatch::new().tool_choice(::rig::message::ToolChoice::Required),
               )
           } else {
               ::rig::agent::CompletionCallAction::continue_run()
           }
       }
   }
   ```

5. Approval policy — fail-closed: in `on_dispatch`, return `DispatchAction::proceed()` for allow-listed tools (check `event.tool_name()`/`tool_args()` first) and `DispatchAction::skip(reason)` otherwise; the reason reaches the model as the tool result so it can adjust instead of failing.

6. Structured failures — override `map_error` to classify and enrich: `::rig::tool::ToolExecutionError::other("disk read failed").with_model_feedback("...").with_code("EIO").with_retryable(false)`; `network` exists for retryable connectivity failures. Observe per-call facts (`ToolResult`, `ToolContext` metadata) in `on_outcome`.

## Pitfalls

- Default `map_error` maps any source error to `ToolErrorKind::Other` with safe kind-level model feedback; override it or use `with_model_feedback` / `with_model_output` for actionable content.
- Patching `ToolChoice::Required` on every turn loops until `PromptError::MaxTurnsError` — gate the patch on `ctx.turn() == 1`.
- `on_dispatch` fires for every effect family (completions included); check `event.tool_name()`/`tool_args()` before applying tool policy.
- `ToolContext` slots key on `ContextValue::KEY` — two types sharing a key collide; interior-sharing values (`Arc<Mutex<_>>`, atomics) belong on the tool instance, not the context.
- `ToolSet::execute` returns a `ToolResult`, never panics; check `is_success()`/`error()`.
- Manual loops must wrap output with `UserContent::tool_result_for(call_id, provider, name, content)`; a plain text message does not satisfy the tool-result contract.
- Any serializable output works as `type Output`, but only `ToolResultContent`/`Vec<ToolResultContent>`/explicit `ToolOutput` preserve multimodal (image) presentation.

## Verify

- `cargo check` after a new `Tool` impl — associated-type and schema-shape errors surface here.
- Assert schema shape: `tools.tool_definitions()` returns `Vec<::rig::completion::ToolDefinition>` in registration order.
- Reference examples (upstream `examples/`): `agent_with_tools` (dynamic tools), `manual_tool_calls` (raw loop), `force_tool_first_turn` (patch footgun), `tool_result_outcomes` (classification + policy hooks), `agent_with_approval_policy`, `rag_dynamic_tools` (retrieved tools).

## Provenance

- Repo: 0xPlaygrounds/rig, commit 6828097, date 2026-09-14.
- Upstream ships breaking changes: re-verify trait signatures before reuse.
