//! Next handler to dispatch to.

/// Next handler to dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextHandler {
    /// The same handler retries.
    Same,
    /// A repair handler picks up.
    Repair,
    /// A human reviewer picks up.
    Human,
}
