# ARCMiS harness bottleneck → Jev-candidate map (read-only investigation)

All paths relative to `/home/ayin/projs/ARCMiS`. Live artifacts only read, never touched. Laya skill + corpus read before any laya claim (`.omp/skills/laya/SKILL.md`, references = laya-rs 0.1.0 verbatim).

## 0. Shared context the ranking rests on

- Deep-dive round: `20261007T004708Zv3s34-html-sweep` (10.7h wall, 1087 model calls, single run no restarts — cleanest closed sample). Cross-check: `20261005T071828Zv3s31-remimu-sweep` (13h59m, 1638 calls).
- **Role wall + tokens, html round** (from `traces/turns.jsonl` model_call→model_response deltas + usage):
  - translator: 313 turns, **3.27h**, 10.43M in / 97.7k out, 31.4k in/call, max 64.3k
  - repairer: 347 turns, **2.99h**, 9.26M in, 26.7k in/call
  - validator: 119 turns, 1.73h, 2.99M in, 21.1k in/call
  - tester: 88 turns, 1.04h, 2.13M in
  - orchestrator: 20 turns, **0.05h**, 79.5k in total (≈4.0k/call) — cheap
  - Total model wall 10.2h of 10.7h; **862/1063 responses are empty-text tool-only turns**.
- **Cached input tokens = 0.0M of 26.3M input (0%)** across all 1087 calls. Every retry/tool-loop turn re-pays full prefill.
- **Phase wall (html)**: Discovery 34.6min → Contract 128.6min → **Planning 458.7min** → Pilot 19.9min. Remimu: Planning 5.4h of 14.7h.
- Remimu grind: translator 798 turns/6.17h, repairer 560 turns/4.37h (13.4h of model wall).
- Death census html ledger (`run/ledgers/failures.jsonl`): 22×503 admission/queue, 14×Length output-cap, 12×MaxTurns-40, 1×context_length_exceeded.
- Wall-time ranking (ROUNDS.yaml, 41 rounds with known wall): remimu 13h59m > html 10h41m > strsim 9h09m > skp 7h32m > coroutine 5h22m …

**Honest headline**: the dominant sinks are *not* decision LLM calls — they are translator/repairer tool-loop grind with 30–60k uncached prefills. Decision calls (orchestrator, leads) cost <1h combined. Jev-style typed decisions can only shave the decision/turn layer, and one structural candidate (validator pre-gate) is deterministic, not laya.

## 1. Ranked bottleneck → candidate table

| # | Bottleneck (evidence) | Candidate | Saving class | Surface | Risk |
|---|---|---|---|---|---|
| 1 | **Translator/repairer grind**: 6.26h/10.2h model wall html; 10.5h/14.7h remimu; 30–64k input/call, 0% cached (`traces/turns.jsonl` usage; remimu 32-60k bin: translator 191, repairer 163 calls) | **Not a small decision.** Two levers: (a) snapcompact token-estimate gap, (b) prefix caching. Compaction trigger is deterministic (no LLM) but under-fires: `snapcompact/src/hook.rs:45-50` projects via `message_tokens_public` = bytes/4 (`compact.rs:108+`), config threshold 12000/keep 4000 (`ARCMiS/lib/agents/src/util/config.rs:27-40`), yet translator prompts reach 60-64k engine tokens — estimate ≪ engine count (image frames + tool JSON under-counted). Prefill re-payment is engine-side (ninfer `cached_input_tokens:0` everywhere). | Tokens (26.3M input/round) + latency | `ARCMiS/lib/snapcompact/src/compact.rs`, `hook.rs`; engine config | Estimate fix is harness-side but changes compaction timing mid-run; caching is infra |
| 2 | **Repair re-entry classification**: every failed validation spawns a full failure-analyst LLM dispatch (`ARCMiS/lib/orchestrator/src/taskgraph.rs:496-499`), then repairer (line 508). Remimu: 74 failure-analyst turns, 0.84h, 1.25M input. Comment at line 505 already records the cost lesson ("96 repairer calls with 300-800s turns") | **(b) Retry-vs-abort triage** — replace/augment failure-analyst classification with a laya forward pass on the trained `agent_trace_observability` schema. The existing pattern is `JevTriage` (`jev_triage.rs`, ADR 0027) but it only *observes* after member dispatches (`lead.rs:247-251`, policy `Enforce` gates at `lead.rs:253-283`); the collective's failure-analyst path (`run_collective`, taskgraph.rs:461-517) never consults it | Turns (1 LLM turn per failed validation) + ~1.25M input tokens/round; latency ms vs ~53s/turn | `taskgraph.rs::run_collective`; new module next to `jev_triage.rs` | **Hard constraint**: checkpoint is fine-tuned on exactly one workflow; off-schema labels → ~0.36 accuracy (`jev_triage.rs:5-7`). Laya returns a class label, not a diagnosis; repairer currently receives diagnosis text capped 1200 chars (taskgraph.rs:502-504) — dropping it forces repairer re-diagnosis (more tool turns). Prototype must measure this trade offline first |
| 3 | **Validator as suite-runner**: `run_collective` dispatches the validator LLM unconditionally per batch (`taskgraph.rs:470-479`); the validator runs cargo test itself through tool calls (html: 139 responses, 127 `read`-class tool calls, 21k in/call). "Is the suite green?" is structurally deterministic | **(e) Validator pre-gate — confirmed deterministic-able.** Run the target test command in the harness before dispatching; skip validator when green (collective already stops early on validator pass, taskgraph.rs:480-486). NOT an LLM call today for the verdict itself — `judge_output` parses the validator's `VALIDATION` line deterministically (`loop_.rs:569-596`). **This is a pure harness change, no laya needed** | Validator turns + 2.99M input tokens/round (html) | `taskgraph.rs::run_collective`; config knob in `MasConfig` (config.rs:95+) | Loses the validator's semantic review beyond test-green; needs config gate + ADR (decisions rule); test command already in `source.target.test_command` (html `config.yml`) |
| 4 | **Lead next-dispatch choice**: lead LLM turn per inner round choosing which member(s) to dispatch (`lead.rs:101-133`, prompt lines 109-131). Html: migration-lead 70 calls, 1.5k in/call, 0.45h; remimu 53 calls | **(c-adjacent) Lead routing decision** — genuinely a small choice over ≤4 members with a transcript. But per-call cost is already tiny (1.5k in, 27s) and the lead must read dispatch results to choose | Turns: marginal; ~few minutes/round | `lead.rs::parse_lead_round` consumers; new laya question module | Off-schema checkpoint problem as #2; low ROI vs #2/#3 |
| 5 | **Phase-exit / done arbitration**: orchestrator emits `DECISION: done`; gate is already deterministic (delegation watermark + carried-work reprieve, `loop_.rs:284-306`; `carried_work_advance` ~line 780). v3s26 `decisions[0]`-only drop already fixed in code (`split_leading_closes`, `loop_.rs:473-479`, doc comment lines 468-476). Orchestrator turns cost 9.1s mean, 4k in | **(a)/(f) — already deterministic; no LLM call to remove.** A laya "should we advance?" would *add* nothing; the v3s26 pathology was a parser bug, closed by `10ac4f6` (SUMMARY.md v3s26 postmortem) | ~0 | — | None — candidate closed |
| 6 | **Tier-1 difficulty routing**: `route_tier1` is pure code (`taskgraph.rs:302-344`); `resolve_role` deliberately skips the router model — keyword pass or default translator (`taskgraph.rs:350-357`, doc: "The router model is skipped") | **(c) — already deterministic.** `router.rs:12-46` keyword pass exists | 0 (already saved) | — | Only residual: keyword-miss defaults to translator; a laya router would need a trained schema (see #2 constraint) |
| 7 | **Admission-503 restart inflation**: html ledger 22×503; bst rescore wall 10520s vs 7962s true window; remimu 50354s vs 53084s (SUMMARY.md wave closes); `prompt_with_retries` backs off `min(60*attempt, 300)` (`loop_.rs:672-683`) | Not a decision — backoff is deterministic; infra/engine queue capacity | Wall hours (bst lost ~43min, skp absorbed 11×503 burst) | engine/sweep.sh slots | Out of scope for laya |
| 8 | **Output-cap/MaxTurns deaths**: 14 Length + 12 MaxTurns-40 (html), 5 repairer cap deaths (v3s6 seed) → fixes already config-owned (`role_output_tokens`, `role_think.repairer=false`, config.rs:130-140) | Config policy, not a decision call | Turns | configs | Already landed |

## 2. Decision-call frequency per round (what a Jev layer would actually touch)

- Orchestrator rounds: 20 calls/round-cap (html), 4k in each — **0.05h total**. Replacing with laya saves nothing material.
- Lead inner rounds: 70 (migration-lead, html) at 1.5k in — 0.45h.
- Failure-analyst: 74 (remimu) — 0.84h + feeds repairs.
- jev_triage today: **disabled in the current sweep configs** (no `jev_triage` section in html `config.yml`; last A/B in `20261001T002104Zv3s1-gonameparts-sweep/config.yml` at threshold 0.3). A/B history (SUMMARY.md "jev decision-quality A/B"): at 0.9 the judge never fired (all below gate, silent Fallback); candidates listed: threshold 0.6, log Fallback at info, persist member output heads — none landed in current wave configs.

## 3. Hard constraint on any new typed decision

`jev_triage.rs:5-7` + `round_triage.rs:1-13`: the checkpoint (`assets/models/laya-typed-decisions`, 808MB, `model.safetensors` + `encoder/config.json` + `tokenizer/`, `rl_agent_config.json` shows ModernBERT-large, max_len 1024) is fine-tuned on exactly four workflows; **any new question schema not in the trained set yields ~0.36 base accuracy**. So "extend laya with one new typed decision" has two honest shapes:

1. Reuse the trained `agent_trace_observability` five-question map on a *new call site* (state mapping is the only new code — exactly what `round_triage.rs` did for ADR 0028).
2. Train/convert a new checkpoint (out of prototype scope; `laya-rs convert` path exists per SKILL.md).

## 4. Minimal prototype recommendation (no harness contact)

**Do (zero harness/flag/prompt changes):**

- New replay test/bench (e.g. `ARCMiS/lib/orchestrator/tests/repair_triage_replay.rs` or a `harness triage` extension following `triage_cmd.rs:1-90`, which is already "synchronous and read-only"): load `assets/models/laya-typed-decisions` via the corpus pattern `AgentBuilder::new().build(dir)` (`references/src/agent.rs:348` loader; `predict`/`predict_map` at `agent.rs:538-553`), replay **recorded** dispatch results from a closed round's `run/ledgers/decisions.jsonl` + `traces/turns.jsonl` (pass/fail ground truth already in ledger rows), consult `jev_triage::workflow_questions()` per result, and report (a) agreement with actual outcomes, (b) confidence distribution → the 0.9-gate no-op evidence question, (c) counterfactual: how many failure-analyst turns (#2) and lead rounds (#4) a confident verdict would have decided. This is the same offline-method used for the A/B in SUMMARY.md.
- Fallback-logging + threshold A/B can ride the same replay with zero risk (the SUMMARY.md-listed candidates (a)/(b)).

**Needs a harness flag (config-owned, no prompts):**

- Validator pre-gate (#3) — one `MasConfig` knob + `run_collective` branch + ADR; biggest single token saving of all candidates and needs no laya at all.
- jev_triage at Enforce with threshold 0.6 on the dispatch path — section exists (`config.rs:267+`), currently just unset in wave configs.

**Needs prompt changes:** none of the above. (Deliberately: task-text discipline and role prompts are already tuned; no-tuning rule forbids touching them for a prototype.)

**Not viable as Jev candidates:** tier routing and phase-exit/done (already deterministic code), 503/backoff (infra), translator grind (generation work, not classification — though #1's compaction-estimate fix is where its wall time actually moves).

## 5. Candidate-by-candidate verification detail (evidence per assignment item)

- **(a) Phase-exit/advance**: orchestrator turn with prose `DECISION: done` (`loop_.rs:99-106`); the harness executes it mechanically. Gate = phase_delegations watermark + `carried_work_advance` reprieve (`loop_.rs:284-306`), deterministic Rust, no model call. Laya candidate: none.
- **(b) Retry-vs-abort after failure classification**: `run_collective` loop (`taskgraph.rs:461-517`), `max_repairs` default 2 (`config.rs:321`). Live candidate #2 in the table.
- **(c) Task difficulty routing (tier-1 direct vs lead)**: `route_tier1` (`taskgraph.rs:302-344`) — name-a-lead → handoff, name-a-specialist under `deny_direct` → Refuse (lines 334-341). Pure code. `deny_direct: true` default (`config.rs:172`).
- **(d) Context compaction triggers**: snapcompact hook fires on projected tokens crossing threshold (`hook.rs:45-50`), deterministic, attached per-agent at build (`registry.rs:106-116`, `179-186`). No LLM call exists to replace; the improvement is estimate accuracy, not decision replacement.
- **(e) Validator pre-gate**: confirmed deterministic today at the verdict layer (`judge_output`, `loop_.rs:569-596` parses `VALIDATION`/`CRITIQUE`/`REPAIR`/`DIAGNOSIS` lines); the *cost* is that the validator is an LLM agent running the suite through tool calls. Candidate #3.
- **(f) Done-verb arbitration (v3s26)**: root cause was `decisions[0]`-only application dropping the trailing `done` after leading `close` verbs; fixed by `split_leading_closes` (`loop_.rs:468-479`, commit `10ac4f6`). The refusal-loop pathology (fft v3s3: three rounds of done-refusals with the deliverable complete) already addressed by `carried_work_advance`. No laya role.

## 6. Constraint compliance notes for the eventual implementation

- No magic constants: confidence threshold, pre-gate on/off all config-bubbled (`ARCMiS/lib/agents/src/util/config.rs` pattern).
- ADR required: #3 (new gate), #2 call-site (extends ADR 0027) — `docs/adr/` has 0023/0027/0028 to extend, not duplicate.
- Minimal-code: #3 is ~30 lines in `run_collective` + config field; #2 replay is test-only initially.
- Lints: workspace clippy configured at `Cargo.toml` root; new files plain paths per rust rule.
- Laya API symbols verified against the vendored corpus only: `AgentBuilder::build(dir)` (`references/src/agent.rs:348`), `predict`/`predict_map` (`agent.rs:538/553`), `Answer.choice/score/noul/probabilities/confidence`, `QuestionKind::{Choice,Score,Noul}` via `laya_core` re-export (`references/src/lib.rs:45-46`), `output_tokens == 0` on a decision (`references/tests/agent.rs:38-44`). Nothing written from model prior.
