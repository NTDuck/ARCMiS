//! The typed workload a node carries.

/// The typed workload a node carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Source-code analysis toward the source map.
    Analyze,
    /// Dependency planning into batches.
    Plan,
    /// Translate one batch.
    Translate,
    /// Judge one translated batch.
    Validate,
    /// Run the test command for one batch.
    Test,
    /// Repair one batch after diagnosed failure.
    Repair,
    /// Compose translated batches.
    Integrate,
    /// End-of-run adversarial review.
    Review,
}
