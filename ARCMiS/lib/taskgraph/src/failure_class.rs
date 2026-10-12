//! Failure class for one task result.

/// Failure class for one task result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// The translation step failed.
    TranslationError,
    /// The verification step failed.
    VerificationError,
    /// The task ran past its time budget.
    TimeoutError,
    /// The task exhausted its attempt budget.
    BudgetExhausted,
    /// The task was escalated to a human.
    Escalated,
}
