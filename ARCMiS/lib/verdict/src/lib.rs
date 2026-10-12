//! `mas_verdict` computes run verdicts from task evidence.
//!
//! The verdict is the conductor's final answer: did the run succeed or
//! fail, and why. The `VerdictComputer` rolls per-task verdicts into
//! one run-level verdict using the evidence each task collected.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every phase gate held through FinalValidation.
    Success,
    /// The run ended early. Carries the failure class.
    Failed {
        /// Why the run failed.
        reason: String,
    },
}

/// One task's evidence for verdict computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvidence {
    /// Task identifier.
    pub task_id: String,
    /// Whether the task succeeded.
    pub succeeded: bool,
    /// Optional failure reason.
    pub failure_reason: Option<String>,
}

/// Computes run-level verdicts from per-task evidence.
#[derive(Debug, Default)]
pub struct VerdictComputer;

impl VerdictComputer {
    /// A new verdict computer.
    pub fn new() -> Self {
        Self
    }

    /// Roll per-task evidence into one run verdict. Returns `Success`
    /// when every task succeeded. Returns `Failed` with the first
    /// failure reason when any task failed.
    pub fn compute(&self, evidence: &[TaskEvidence]) -> Verdict {
        for entry in evidence {
            if !entry.succeeded {
                let reason = entry.failure_reason.clone().unwrap_or_else(|| "task failed".to_owned());
                return Verdict::Failed {
                    reason,
                };
            }
        }
        Verdict::Success
    }
}
