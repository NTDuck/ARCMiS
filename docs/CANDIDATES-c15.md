# C15 candidate backlog - standing defect classes

Standing rule (author directive): every candidate targets at least one
defect class below alongside the primary metric. N-values are config keys
with defaults; no problem-set-specific constants.

## 1. WALL TIME (runs too long; smoke 2159 s / 13 rounds; c11b killed 6 min after last pass)

- **C1 brief no-re-read enforcement** - the lead brief already forbids
  re-reading own output; enforce mechanically: the read gate charges a
  stalled round when a tier-3 agent reads a file it wrote this batch.
  Hypothesis: charging self-rereads as stalled rounds cuts wall seconds
  per completed task by removing redundant read round-trips.
- **C2 suite-delta validation** - validators re-run the whole suite when
  the delta is one module. Give the validator preamble a delta protocol:
  run the touched module's tests first, full suite only on the final
  batch validation. Hypothesis: per-batch validation latency drops with
  no verdict change because the full suite still runs at FinalValidation.
- **C3 earlier fanout on independent batches** - the orchestrator round
  prompt should tell the lead to fanout independent leaf batches (the
  task graph already supports it). Hypothesis: two independent leaf
  batches interleaved cut the Migration wall clock by overlapping the
  batch tails.

## 2. MEANINGLESS LOOPS (3 stalled smoke rounds; refused done claims; MaxTurns deaths at 22 turns)

- **C4 identical-DECISION breaker (N=2)** - detect a lead repeating the
  same DECISION line twice (N configurable, default 2) across rounds and
  force escalate: the next round prompt names the repeat and demands a
  delta instruction or done. Hypothesis: forcing escalation at N=2
  converts silent repeat loops into either progress or an honest
  failure, cutting stalled rounds per run.
- **C5 refusal rounds name the reason with a required delta** - a
  refused delegation already feeds the ledger line back; extend the
  feedback to require the next task text to start with the delta
  keyword (`FIX:` or `RETRY-WITH:`) and count refusals as stalled
  rounds. Hypothesis: naming the refusal plus a required delta stops
  unchanged re-delegations (measures the task-text discipline fix).
- **C6 lead loop guard** - the lead parser rejects a re-delegation of an
  unchanged task to the same member after a fail unless the text carries
  a delta keyword (pairs with C5, one code site). Hypothesis: the parser
  guard eliminates the retry-identical-task pattern seen in the smoke.

## 3. TURN BUDGET STARVATION (22 MaxTurnsError deaths in the smoke; migration-lead turns:8)

- **C7 80 percent budget warning** - inject one prompt line at 80
  percent of the turn budget: "N turns left; wrap up and emit done with
  the deliverable status." Hypothesis: the wrap-up warning converts
  mid-deliverable MaxTurns deaths into completed or honestly-failed
  tasks.
- **C8 adaptive turn budgets by deliverable class** - write-heavy roles
  (translator, architect) get a higher default than read-only roles
  (judge, critic) via config (`role_turns` map). Hypothesis: matching
  budgets to deliverable classes removes the starvation class without
  raising the global ceiling.

## 4. BURN-RATE WASTE (cap deaths; already largely addressed)

- **C9 cap-death trace watch** - keep scoring deaths at exactly the
  output cap (finish_reason=Length with no text) as a defect-class
  metric; no code change, scoring-side. Hypothesis: none (metric only).

## Scoring keys added per set (from this round on)

- primary: verified tests (unchanged)
- `wall_seconds_per_completed_task`
- `stalled_rounds`
- `maxturns_deaths`
- `cap_deaths`
