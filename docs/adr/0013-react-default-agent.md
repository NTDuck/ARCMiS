# 0013. ReAct default agent

- **Date:** 2026-09-17
- **Status:** superseded by [0015](0015-two-structured-artifact-agents.md)

## Context

The default agent drove rig's native tool-call loop. The model emitted structured tool calls and rig dispatched them. The run contract called for the ReAct methodology of Yao et al. (arXiv:2210.03629), implemented exactly as the paper defines it. The two surfaces differ. Rig emits provider-native tool calls. ReAct emits `Thought` and `Action` text lines with bracketed actions and receives `Observation` lines.

## Decision

- The default agent is a ReAct loop over the raw completion API. Every step is one Thought and one Action in a single assistant message, matching the paper's `Thought N` / `Action N` exemplar format. The harness executes the action and injects the environment reply as an `Observation:` user message. The context is the full raw trajectory. No summary buffer exists.
- The action vocabulary is `Write[json]`, `Bash[line]`, and `Finish[answer]`. `Finish` is an ordinary action whose return is the answer, per the paper. A reply without a parseable Action gets an observation asking for the format. The loop continues.
- Nothing goes beyond the paper: no planning trees, no reflection or self-critique, no self-consistency, no memory beyond the trajectory. The step budget is 12 (`REACT_MAX_STEPS`), close to the paper's 7-step reasoning-task cap. The config `max_turns` no longer bounds the loop directly.
- The loop runs on `CompletionRequestBuilder` (preamble, messages, temperature, ollama params, max tokens per request). This agent does not use rig's agent tool-call machinery.
- Non-action text errors and unknown action names surface as observations, never as crashes. Only a model error or an exhausted budget returns an error.

## Consequences

- The model needs no native tool-call support. The loop works over plain text providers.
- Tool schemas no longer reach the provider. The preamble carries the action vocabulary, so preamble wording is load-bearing.
- The agent now wires two domain tools (`Write` and `Bash`) instead of three. `Read`, `Search`, and the rest of the seventeen-tool surface stay available to future agents or a tool-exploration phase.
- rig `PromptError::MaxTurnsError` no longer occurs. Budget exhaustion returns a request error with the budget text.
