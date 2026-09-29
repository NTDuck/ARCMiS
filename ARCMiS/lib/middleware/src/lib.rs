//! Role-agnostic rig `AgentHook` middleware (ADR 0025). Every module here is
//! a cross-cutting concern over the model-call path: it never names a role,
//! a fleet, or the orchestration domain. The agents crate composes these
//! hooks per role. The domain logic (which hook, with which parameters for
//! which role) stays there.

pub mod continuation;
pub mod role_output_tokens;
pub mod trace;

pub use continuation::TrailingUserMessageHook;
pub use continuation::CONTINUATION_QUERY;
