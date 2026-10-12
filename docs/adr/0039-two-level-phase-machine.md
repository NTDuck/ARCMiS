# 0039. Two-level phase machine

- **Date:** 2026-10-12
- **Status:** accepted
- **Extends:** [0031](0031-mas-core-conductor-over-omp-transport.md), [0036](0036-hierarchical-mas-claude-code-modernization.md)
- **Related:** [0037](0037-mas-core-refactor-implementation-plan.md)

## Context

The Claude Code Modernization workflow defines a run-level FSM
with fourteen states and a per-module FSM with six states plus
three failure transitions. The current `Phase` enum holds ten
states for the run-level FSM only. The refactor must add the new
run-level states and introduce a per-module FSM that the conductor
coordinates with the run-level FSM.

## Decision

### 1. Run-level FSM

The `Phase` enum grows from ten to fourteen states. The new
states slot into the existing forward walk:

```
Init -> Preflight -> Assess -> Map -> ExtractRules -> ReviewRules
     -> DraftBrief -> MigratePilot -> VerifyPilot -> MigrateBatches
     -> VerifyModule -> FinalVerify -> Harden -> HumanSignOff -> Done
```

The mapping from old to new:

| Old phase | New phase |
| --------- | --------- |
| `Preflight` | `Preflight` |
| `Discovery` | `Map` |
| `Contract` | `DraftBrief` |
| `Planning` | `MigratePilot` |
| `Pilot` | `VerifyPilot` |
| `Migration` | `MigrateBatches` |
| `Integration` | `VerifyModule` |
| `Hardening` | `Harden` |
| `FinalValidation` | `FinalVerify` |
| `Done` | `Done` |

New phases added by the refactor:

| New phase | Purpose |
| --------- | ------- |
| `Init` | Deterministic initialization before any model call. |
| `Assess` | Repository assessment: size, complexity, risk. |
| `ExtractRules` | Business rules extraction from source code. |
| `ReviewRules` | Citation audit of extracted rules. |
| `HumanSignOff` | Human approval gate before `Done`. |

The `Init` phase sits before `Preflight`. The `Assess`,
`ExtractRules`, `ReviewRules` phases sit between `Preflight` and
`DraftBrief`. The `HumanSignOff` phase sits between `Harden` and
`Done`.

### 2. Per-module FSM

A new `ModulePhase` enum carries the per-module FSM:

```rust
pub enum ModulePhase {
    Planned,
    Ready,
    Baselined,
    Migrating,
    Verifying,
    Proven,
    Diagnosing,
    Repairing,
    Escalated,
}
```

The forward walk:

```
Planned -> Ready -> Baselined -> Migrating -> Verifying -> Proven
```

The failure transitions:

```
Migrating -> Diagnosing -> Repairing -> Migrating
Migrating -> Escalated
Verifying -> Diagnosing -> Repairing -> Verifying
Verifying -> Escalated
```

The `Diagnosing` phase runs the `FailureAnalyst` to classify the
failure. The `Repairing` phase runs the `Repairer` to apply the
suggested fix. The `Escalated` phase is terminal: the module
cannot be migrated automatically and requires human intervention.

### 3. Evidence gates per transition

Every phase transition has an evidence gate. The gate is a pure
function over the artifact store and the test outcomes. The gate
returns `true` when the evidence satisfies the transition's
requirements.

Run-level gates:

| Transition | Gate |
| ---------- | ---- |
| `Init -> Preflight` | Config loaded, transport ready. |
| `Preflight -> Assess` | Config validated, hierarchy compiled. |
| `Assess -> Map` | Repository metadata collected. |
| `Map -> ExtractRules` | Source map published. |
| `ExtractRules -> ReviewRules` | Business rules extracted. |
| `ReviewRules -> DraftBrief` | Citations audited. |
| `DraftBrief -> MigratePilot` | Migration brief published. |
| `MigratePilot -> VerifyPilot` | Pilot module migrated. |
| `VerifyPilot -> MigrateBatches` | Pilot module verified. |
| `MigrateBatches -> VerifyModule` | All modules migrated. |
| `VerifyModule -> FinalVerify` | All modules verified. |
| `FinalVerify -> Harden` | Toolchain green. |
| `Harden -> HumanSignOff` | Security audit complete. |
| `HumanSignOff -> Done` | Human approval received. |

Module-level gates:

| Transition | Gate |
| ---------- | ---- |
| `Planned -> Ready` | Module scheduled. |
| `Ready -> Baselined` | Baseline tests captured. |
| `Baselined -> Migrating` | Strategy selected. |
| `Migrating -> Verifying` | Translation complete. |
| `Verifying -> Proven` | Tests green, artifacts complete. |
| `Migrating -> Diagnosing` | Translation failed. |
| `Diagnosing -> Repairing` | Failure classified. |
| `Repairing -> Migrating` | Fix applied. |
| `Migrating -> Escalated` | Repair budget exhausted. |
| `Verifying -> Escalated` | Verification budget exhausted. |

### 4. Failure regression edges

Run-level regression edges:

| From | To | Reason |
| ---- | -- | ------ |
| `Assess` | `Preflight` | Config invalid. |
| `Map` | `Assess` | Source map incomplete. |
| `ExtractRules` | `Map` | Rules extraction failed. |
| `ReviewRules` | `ExtractRules` | Citation audit failed. |
| `DraftBrief` | `ReviewRules` | Brief incomplete. |
| `MigratePilot` | `DraftBrief` | Pilot strategy invalid. |
| `VerifyPilot` | `MigratePilot` | Pilot verification failed. |
| `MigrateBatches` | `VerifyPilot` | Batch migration failed. |
| `VerifyModule` | `MigrateBatches` | Module verification failed. |
| `FinalVerify` | `VerifyModule` | Toolchain red. |
| `Harden` | `FinalVerify` | Security audit failed. |

Module-level regression edges:

| From | To | Reason |
| ---- | -- | ------ |
| `Baselined` | `Ready` | Baseline tests failed. |
| `Migrating` | `Baselined` | Translation regressed. |
| `Verifying` | `Migrating` | Verification regressed. |
| `Diagnosing` | `Baselined` | Diagnosis inconclusive. |
| `Repairing` | `Diagnosing` | Repair failed. |

### 5. Coordination between run-level and module-level FSMs

The run-level FSM owns the module-level FSMs. The conductor holds
one `ModuleStateMachine` per module. A module FSM advances only
when the run-level FSM permits:

- `MigrateBatches` permits module FSMs to advance from `Ready`
  through `Proven`.
- `VerifyModule` permits module FSMs to advance from `Verifying`
  through `Proven`.
- `FinalVerify` reads the verdicts from all module FSMs and
  computes the run-level verdict.

The conductor coordinates the two FSMs in the `round` method:

1. Read the current run-level phase.
2. If the run-level phase permits module FSM advancement, advance
   the ready module FSMs.
3. Dispatch the ready module work items.
4. Drain transport events.
5. Update module FSMs based on dispatch results.
6. Check the run-level phase gate.
7. Advance the run-level FSM if the gate holds.

### 6. Backward compatibility

The existing `Phase` enum keeps all ten old variants. The old
variants are marked `#[deprecated]` with a note pointing to the
new variant. The `From<OldPhase> for Phase` impl maps old to new.
Existing call sites that match on old variants continue to work;
the compiler emits a deprecation warning.

The `PhaseMachine` keeps its existing API. New methods add
module-level coordination without changing the existing
`try_advance`, `accept`, `is_legal`, `forward_next`,
`regression_target` methods.

## Consequences

- The `Phase` enum grows from 10 to 14 variants. Every existing
  variant keeps its name and behavior. New variants add new
  phases without breaking existing configs.
- A new `ModulePhase` enum carries the per-module FSM. The
  conductor holds one `ModuleStateMachine` per module.
- The run-level FSM owns the module-level FSMs. Module FSMs
  advance only when the run-level FSM permits.
- Evidence gates move from a single `Evidence` enum to a
  per-transition gate function table. The table is a pure
  function over the artifact store and the test outcomes.
- Failure regression edges cover every phase that can fail. The
  edges are deterministic: a failure in phase X regresses to
  phase Y, never to an arbitrary phase.
- The deprecation warnings guide operators to update their
  configs. The warnings are non-blocking; the refactored
  conductor accepts both old and new phase names.

## Verification

- Unit: the phase machine walks the full forward path from
  `Init` to `Done` and rejects illegal transitions.
- Unit: the module state machine walks the full forward path
  from `Planned` to `Proven` and exercises every failure
  transition.
- Unit: every evidence gate returns `true` when the evidence
  satisfies the transition's requirements and `false` otherwise.
- Unit: every regression edge is legal and points to a phase
  that can rework the failure.
- Unit: the conductor coordinates the two FSMs without races.
- Integration: the conductor runs the full phase walk over a
  scripted transport with the example config and reaches `Done`.
