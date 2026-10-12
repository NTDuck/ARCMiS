//! Run failure classes for the MAS conductor and its collaborators.
//!
//! Every variant names a distinct recovery posture: restart the
//! transport, correct the configuration, tighten the gateway policy,
//! or raise the budgets. The error types live together so the
//! conductor and its collaborators share one vocabulary.

pub use crate::advisor_error::AdvisorError;
pub use crate::conductor_error::ConductorError;
pub use crate::transport_error::TransportError;
pub use crate::validation_error::ValidationError;
