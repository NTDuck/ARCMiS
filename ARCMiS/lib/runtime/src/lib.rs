//! `mas_runtime` is the IO and runtime layer for the MAS conductor.
//!
//! It bundles the wire vocabulary, the transport seam, the error
//! classes, the context compaction, and the read-only advisor. The
//! conductor and its collaborators depend on this crate for every
//! concern that touches the outside world.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod advisor;
pub mod advisor_config;
pub mod advisor_consultation;
pub mod advisor_error;
pub mod advisor_verdict;
pub mod agent_progress;
pub mod compact;
pub mod compact_frames;
pub mod compaction_outcome;
pub mod conductor_error;
pub mod context_frame;
pub mod error;
pub mod extension_ui_request;
pub mod forward_frames;
pub mod frames;
pub mod omp_launch_config;
pub mod project_tokens;
pub mod prompt_error_detail;
pub mod prompt_result;
pub mod ready;
pub mod response;
pub mod sdk_error_map;
pub mod session_event;
pub mod subagent_event;
pub mod subagent_lifecycle;
pub mod subagent_progress;
pub mod transport;
pub mod transport_error;
pub mod transport_trait;
pub mod validation_error;

mod map;
mod map_notification;

pub use advisor::OmpAdvisor;
pub use advisor_config::AdvisorConfig;
pub use advisor_consultation::AdvisorConsultation;
pub use advisor_verdict::AdvisorVerdict;
pub use compact::{compact_frames, project_tokens, CompactionOutcome, ContextFrame};
pub use error::{AdvisorError, ConductorError, TransportError, ValidationError};
pub use forward_frames::forward_frames;
pub use frames::{
    AgentProgress, Event, ExtensionUiRequestFrame, PromptErrorDetail, PromptResultFrame, ReadyFrame, ResponseFrame,
    SessionEvent, SubagentEventFrame, SubagentEventPayload, SubagentLifecycleFrame, SubagentLifecyclePayload,
    SubagentProgressFrame, SubagentProgressPayload,
};
pub use map::map_sdk_event;
pub use omp_launch_config::OmpLaunchConfig;
pub use sdk_error_map::map_sdk_error;
pub use transport::OmpTransport;
pub use transport_trait::Transport;
