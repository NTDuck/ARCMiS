//! `mas_error` names the run failure classes callers must distinguish.
//!
//! Every variant names a distinct recovery posture: restart the
//! transport, correct the configuration, tighten the gateway policy,
//! or raise the budgets. The error types live together so the
//! conductor and its collaborators share one vocabulary.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod advisor_error;
pub mod conductor_error;
pub mod transport_error;
pub mod validation_error;

pub use advisor_error::AdvisorError;
pub use conductor_error::ConductorError;
pub use transport_error::TransportError;
pub use validation_error::ValidationError;
