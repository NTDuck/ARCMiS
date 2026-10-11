# Technical Specification

- **Date:** 2026-10-11
- **Status:** accepted
- **Companion:** [PRODUCT-SPECS.md](PRODUCT-SPECS.md)

## Crate map

Members live under `ARCMiS/lib/*` and `ARCMiS/bin/*` (ADR 0006).
Package names follow `ARCMiS-<name>` and imports use the short alias
(ADR 0009). No `mod.rs`, one item per file.

### `bin/cli`

Argument parsing, config load, tokio main. Requires an explicit run
config path (ADR 0032). Deserialized config flows onward into the
runtime as parameters: library code holds no settings of its own
(`.omp/rules/config.md`).

### `lib/run-config`

Serde YAML config model for a MAS modernization run. Preflight
validation rejects blank mandatory fields and hierarchy ambiguity
(a member in two teams) with actionable findings, citing the
rejected field and what to set (ADR 0026, ADR 0032). Example:
`configs/example.yml`.

### `lib/omprpc`

`omp --mode rpc` JSONL transport. Owns: banner skip, ready
handshake, `negotiate_protocol`, `set_subagent_subscription`,
`prompt`/`prompt_result`, and subagent routing by
`payload.agent` (ADRs 0030). Wire types carry camelCase serde
tagging. Frame decoding rejects partial reads and unexpected fields.

### `lib/mas-core`

Conductor runtime over the transport. Components:

- **Static hierarchy registry.** One role registry per
  config. A disabled hierarchy reproduces single-tier
  behavior exactly (ADR 0026).
- **Task graph plus phase machine.** Evidence-gated phase
  exit moving forward, failure-driving regression edges.
- **Dynamic router.** Deterministic pass first, laya typed
  decisions at the bottlenecks, orchestrator arbitration
  only for the ambiguous remainder (ADR 0031).
- **Blackboard store.** Topic revisions under a bound,
  per-agent mailboxes under a bound.
- **Direct messaging.** Lead-to-member channels riding the
  blackboard transport.
- **Laya typed-decision guard.** Every consultation accepts
  only `agent_trace_observability`-schema questions: a
  constrained decoding path per ADR 0027, validated at
  init. Verdicts below the confidence threshold cascade
  back to deterministic behavior. Checkpoint loading stays
  local-directory only (ADR 0023).
- **Jev-as-a-guard tool gateway.** Deterministic tier
  first: role allowlist, path policy, deny patterns, budget.
  Lay-down: laya or model arbitration handles only the
  ambiguous remainder (ADR 0031).
- **Snapcompact-style context compaction.** All agents
  attach a compaction hook. Crossing the projection
  threshold archives older context into frames, keeping
  the recent token window verbatim.

## Config contract

One mandatory YAML file per run. The sections, as modeled by
`lib/run-config`:

| Section                                              | Purpose                                       |
| ---------------------------------------------------- | --------------------------------------------- |
| `run.name`                                           | Run identifier cited in logs and artifacts    |
| `paths.input_dir` / `paths.output_dir`               | Input codebase, output destination            |
| `languages.pair`                                     | `{ source, target }` language pair            |
| `languages.test_command`                             | Target-language test command                  |
| `model.id`, `model.extra_args`                       | Model override, extra omp launch args         |
| `hierarchy.enabled`, `hierarchy.teams`                | Team layout. one `{ lead, members, turns, stagnation_rounds }` per team |
| `blackboard.max_topic_revisions`, `blackboard.inbox_depth` | Blackboard and mailbox bounds           |
| `guard.allowlist`, `guard.denylist`                  | Tool allowlist and deny patterns              |
| `compaction.threshold_tokens`, `compaction.keep_recent_tokens` | Snapcompact-style compaction knobs    |
| `laya.enabled`, `laya.checkpoint_dir`, `laya.confidence_threshold` | Typed-decision settings (ADR 0023) |
| `limits.{turns, fanout, collective_turns, max_repairs, stagnation_rounds}` | Budget ceilings                |

## Verification

- **Unit and integration tests.** `cargo nextest run --profile ci
  --workspace --all-targets --no-tests=pass`. Tests live in
  `{crate}/tests/`, never in-module (ADR 0006).
- **Style and rule gates.** `cargo fmt --check`, `cargo clippy
  --workspace --all-targets`, `python3 .omp/scripts/lint-rules.py`,
  and the STE linter over touched files. All must hold before every
  commit (`.omp/rules/build-and-gates.md`).
- **Smoke.** `mise run cli -- configs/example.yml` exercises argument
  parsing, config load, and preflight validation on a real
  configuration.
