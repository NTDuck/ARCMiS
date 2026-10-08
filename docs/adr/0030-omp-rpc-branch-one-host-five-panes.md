# ADR 0030: omp-RPC Branch — One Host, One TUI, Five Panes

## Status

Accepted (branch `omp`, 2026-10-08)

## Context

The `omp` branch rebuilds the migration demo on the omp harness instead of rig.
Requirements:

1. Typing `ARCMiS` in a terminal opens a tmux-like TUI with 5 agent panes.
2. Exactly the ReCodeAgent setup (prompts, preambles, control flow) from the
   paper (arXiv 2604.07341) and its upstream source (vendored at
   `assets/ReCodeAgent`).
3. The user interacts only with the orchestrator, through omp, "exactly like
   how I would do it normally" (composer semantics: steer, follow-up, abort).
4. "in orchestrator omp all other agents running should appear as subagents
   (also a omp builtin feature)".
5. Agents message each other and maintain a blackboard as common memory. Messages may reference blackboard content.
6. View is TUI, not CLI. I ask for no extra UI decoration beyond this list.

## Decision

- **One omp host process.** The TUI (bin `omp-tui`) spawns exactly one
  `omp --mode rpc --approval-mode yolo` subprocess and speaks omp's JSONL RPC
  protocol over its stdio. There is no per-agent omp process, no HTTP, no
  agent-to-agent process mesh.
- **Workers are omp subagents.** The four ReCodeAgent roles (analyzer,
  planning, translator, validator) are omp *task agents* declared as project
  agent files (`.omp/agents/*.md`, one per agent, matching the upstream
  folder-per-agent layout). The orchestrator LLM spawns them with its builtin
  `task` tool. Their lifecycle and events reach the TUI as
  `subagent_lifecycle` / `subagent_progress` / `subagent_event` frames on the
  one host stdout stream (`set_subagent_subscription level "events"`), and as
  `get_subagents` registry snapshots. This surface is omp's builtin subagent
  support. The TUI does not spawn workers itself.
- **Orchestrator pane = the omp session.** Pane 1 renders the main session's
  live transcript (`agent_start`, `message_update`, `tool_execution_*`,
  `agent_end`) and an interactive composer bound to RPC `prompt` /
  `abort_and_prompt` with omp queue semantics (steer vs follow-up, queue
  chips, abort). The other four panes are view-only, like tmux unfocused
  panes.
- **Pane routing.** `get_subagents` maps registry ids to the four worker
  panes. `subagent_*` frames for each id append to that pane. A nested
  `Parent.Child` id routes to the pane of its top segment.
- **Blackboard = shared `local://` root + planning files.** Subagents share
  the parent session's `local://` sandbox (omp builtin), and the ReCodeAgent
  control flow already communicates through a file blackboard
  (`planning_dir/`): research, overall design, functions list, name mapping,
  skeleton, implementation plan, validation report/summary. The orchestrator
  prompt pre-seeds `local://blackboard.md` with the run parameters (source
  root, target root, target language, planning dir) so every agent and every
  message can reference `local://blackboard.md` paths.
- **Inter-agent messaging = `write agent://<id>`** (omp builtin IRC-style
  peer messaging). The orchestrator preamble instructs spawned workers to
  report status to `agent://all` or the orchestrator id. `write` is therefore
  in the worker tool lists.
- **Control flow parity.** This binary generates the orchestrator's own prompt
  from the verbatim `run_agents()` semantics: run analyzer, then
  planning, then up to `max_translation_validation_iterations` (5)
  translator⇄validator iterations. Per-phase success criteria come from the
  Python source. Abort the pipeline on a failed phase. PASS detection
  via `validation-summary.md` existence or `## Status: PASS` in
  `validation-report.md`. Each worker agent definition embeds its verbatim
  upstream template (rendered once, with run parameters) as its system
  prompt, prefixed by its verbatim `CLAUDE.md` preamble section.
- **Upstream deviations, declared:** upstream runs `claude -p` per agent with
  MCP `project-analyzer` / language servers. This demo runs on omp tools
  (`read`, `glob`, `grep`, `bash`, `write`, `edit`) which cover the same
  operations. Upstream translator/validator one-shot subagents become the
  workers' own `task` spawns (depth 2), which still appear in the tree.
  `WebFetch` becomes `web_search`/`read`.

## Consequences

- The TUI is a *host*: it owns one RPC transport, not five sessions. All
  state comes from frames. `get_subagents` and `get_state` are snapshots.
- Worker panes are mirrors of omp's registry/progress stream, so Agent Hub
  semantics (idle/parked/aborted, revive, kill) stay consistent with the
  builtin view.
- Subagent subscription level `events` gives per-worker `message_update`
  snapshots. Panes render a compact transcript tail per worker instead of
  full streaming internals.
- Nothing in the TUI talks to model providers. All credentials and model
  routing stay inside the omp host configuration.

## Verification

- `omp --mode rpc` frame capture script in `docs/dev/`.
- Manual: run the TUI, confirm 5 panes, worker ids appear in
  `get_subagents`, conversion of a fixture project produces
  `planning_dir/` artifacts and target sources.
