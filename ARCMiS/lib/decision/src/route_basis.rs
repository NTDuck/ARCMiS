//! Which routing layer answered.

use crate::decision_source::DecisionSource;

/// Which routing layer answered.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteBasis {
    /// Laya typed decision.
    Typed(DecisionSource),
    /// Deterministic keyword layer.
    Keyword(DecisionSource),
}
