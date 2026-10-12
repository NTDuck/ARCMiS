//! Typed gate criteria one phase exit measured.

/// Typed gate criteria one phase exit measured. Every variant names its
/// evidence precisely: no evidence means no advance.
#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    /// The phase saw at least one completed dispatch.
    CompletedDispatch,
    /// The phase saw at least `minimum` deliverables published.
    Deliverables {
        /// Number of deliverables published.
        published: usize,
        /// Minimum deliverables required.
        minimum: usize,
    },
    /// The phase recorded a green test-command run.
    TestCommandGreen,
}

impl Evidence {
    /// Whether this evidence satisfies its own gate.
    pub fn holds(&self) -> bool {
        match self {
            Evidence::CompletedDispatch => true,
            Evidence::Deliverables {
                published,
                minimum,
            } => published >= minimum,
            Evidence::TestCommandGreen => true,
        }
    }
}
