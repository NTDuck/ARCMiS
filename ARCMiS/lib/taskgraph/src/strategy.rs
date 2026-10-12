//! Migration strategy for one task.

/// Migration strategy for one task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strategy {
    /// Lift the source code with minimal change.
    Uplift,
    /// Transform the source code into idiomatic target code.
    Transform,
    /// Reimagine the module from its specification.
    Reimagine,
}
