# 0015. Two structured-artifact agents: monolith and validator

- **Date:** 2026-09-18
- **Status:** accepted (supersedes ADR 0013)

## Context

ADR 0013 shipped one default agent: a hand-written ReAct loop over the raw
completion API, with `Write[x]`/`Bash[x]`/`Finish[x]` text actions parsed by
custom code. The hand-rolled parser, the manual turn loop, and the free-text
`Finish[answer]` contract duplicated machinery rig already owns, and the
agent boundary carried no typed artifacts: the harness read the final text
and measured the output directory outside the agent contract. User direction
replaced that agent with two thin agents and demanded structured,
schema-enforced input and output at every agent boundary, achieved through
rig features rather than prompt wording.

## Decision

- Two agents replace the default agent:
  - `monolith` (`lib/agents/src/monolith.rs`): translates the input codebase
    in one run. Tools: `write`, `bash` (rooted at the output dir). Input
    artifact: [`MonolithRequest`]. Output artifact: [`MonolithResponse`].
  - `validator` (`lib/agents/src/validator.rs`): validates the monolith's
    output codebase. Tool: `bash` (rooted at the output dir). Input
    artifact: [`ValidatorRequest`]. Output artifact: [`ValidatorResponse`]
    with one `ValidatorStepOutcome` per toolchain step.
- The rig API enforces structure, not the prompts: both agents set
  `output_schema::<T>()` with `OutputMode::Tool`. rig registers the schema
  as a synthetic output tool, validates required fields, re-prompts on
  missing fields, and finalizes the run with the tool-call arguments
  (verified in rig-agent 0.42 `agent/run/mod.rs`). The harness parses the
  final string back into the artifact type.
- The harness (`bin/harness/src/main.rs`) is a thin sequencer: construct one
  ollama client, wire both agents, run monolith then validator, log every
  model call and tool call to the console through one `AgentHook`, and write
  `run-report.md` from the structured validation result.
- Agent definitions stay thin: one preamble of working rules, typed input,
  tools, typed output. No custom loop, parser, or measurement code.
- Artifact ownership: a DTO owned exclusively by one agent or tool lives in
  that agent's or tool's own file. `MonolithRequest` and `MonolithResponse` live
  in `lib/agents/src/monolith.rs`; `ValidatorRequest`, `ValidatorResponse`,
  and `ValidatorStepOutcome` live in `lib/agents/src/validator.rs`. All derive
  `Serialize`, `Deserialize`, and `JsonSchema`, and the crate root
  re-exports them. There is no shared `util/artifacts.rs`.
- The hand-written ReAct loop, its `EmittedAction` parser, and the harness
  measurement module: I deleted them. Compilation and test measurement move into
  the validator's toolchain steps, reported through `ValidatorStepOutcome`.

## Consequences

- Agent boundaries are machine-checkable: a schema violation re-prompts
  inside the run instead of leaking free text into the harness.
- The validator is the single source of compilation status and test pass
  rate. The run report renders from its structured result.
- Ollama models that cannot reliably emit tool calls will fail to deliver
  the output-tool call. Rig re-prompts within the turn budget, and the
  harness surfaces the error. This is the honest failure mode: unstructured
  output no longer masquerades as success.
