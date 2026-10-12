//! `frames` re-exports the wire vocabulary from `ARCMiS-frames`.

pub use frames::AgentProgress;
pub use frames::Event;
pub use frames::ExtensionUiRequestFrame;
pub use frames::ExtensionUiRequestFrame as ExtensionUiRequest;
pub use frames::PromptErrorDetail;
pub use frames::PromptResultFrame;
pub use frames::ReadyFrame;
pub use frames::ResponseFrame;
pub use frames::SessionEvent;
pub use frames::SubagentEventFrame;
pub use frames::SubagentEventPayload;
pub use frames::SubagentLifecycleFrame;
pub use frames::SubagentLifecyclePayload;
pub use frames::SubagentProgressFrame;
pub use frames::SubagentProgressPayload;
pub use frames::map_sdk_event;
