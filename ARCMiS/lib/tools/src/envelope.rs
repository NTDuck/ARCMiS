//! Tool envelope: uniform metadata every tool carries at its boundary.
//!
//! The guard, the model router, and the observability controller address
//! tools through this envelope alone. They never inspect tool internals —
//! the same contract oh-my-pi's approval layer reads (`strict`, `approval`,
//! `concurrency`), typed for the MAS: category for grouping, side-effect
//! class for Allow/Deny/Ask policy, cost class for model routing, preview
//! caps for trace bounding.

use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

/// Functional grouping of one tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ToolCategory {
    /// read, write, edit.
    FileSystem,
    /// search, find, ast_grep, ast_edit.
    Search,
    /// lsp, debug.
    CodeIntelligence,
    /// bash, eval, ssh.
    Runtime,
    /// task, irc, todo, job, ask.
    Coordination,
    /// snapcompact, observability.
    Meta,
}

/// What one call can do to the world. The guard maps this to
/// Allow/Deny/Ask without reading tool internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SideEffectClass {
    /// Pure read: read, search, find, ast_grep, lsp.
    ReadOnly,
    /// Local write inside the workspace: write, edit, ast_edit, todo.
    WriteLocal,
    /// Remote mutation: ssh, eval (remote kernels).
    WriteRemote,
    /// Destructive potential: bash (deny-pattern match), edit targeting
    /// `source/`.
    Destructive,
    /// Privileged execution: bash (sudo), debug (ptrace).
    Privileged,
}

/// Relative resource weight. The router biases model strength by cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CostClass {
    /// read, search, find, ast_grep, todo.
    Cheap,
    /// write, edit, ast_edit, lsp, bash.
    Medium,
    /// eval, ssh, debug, task.
    Expensive,
}

/// Declarative metadata for one tool. Constructed as a `const` per tool; no
/// runtime cost at the boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name as the model sees it.
    pub name: &'static str,
    /// One-line purpose.
    pub description: &'static str,
    /// Functional grouping.
    pub category: ToolCategory,
    /// World effect of one call.
    pub side_effect: SideEffectClass,
    /// Relative resource weight.
    pub cost: CostClass,
    /// Whether a call requires human-style approval before execution.
    pub requires_approval: bool,
    /// Maximum characters of call arguments recorded in a trace.
    pub args_preview_cap: usize,
    /// Maximum characters of call result recorded in a trace.
    pub result_preview_cap: usize,
}

/// Envelope entry in a registry: the tool erased behind its metadata.
///
/// rig's `Tool` trait is not object-safe (`Sized` bound plus generic
/// `call`), so the registry erases at the metadata level. Dispatch happens
/// through the concrete tool the host holds alongside the registry; the
/// guard, router, and observability controller consume this envelope only.
pub trait ToolEnvelope: Send + Sync {
    /// Declarative metadata for this tool.
    fn metadata(&self) -> ToolMetadata;
}

/// Type-erased envelope entry for the registry.
pub type AnyTool = Arc<dyn ToolEnvelope>;

/// Implement [`ToolEnvelope`] for a tool whose inherent impl declares
/// `const METADATA: ToolMetadata`.
#[macro_export]
macro_rules! impl_envelope {
    ($tool:ty) => {
        impl $crate::envelope::ToolEnvelope for $tool {
            fn metadata(&self) -> $crate::envelope::ToolMetadata {
                <$tool>::METADATA
            }
        }
        impl $crate::registry::ConstMetadata for $tool {
            const METADATA: $crate::envelope::ToolMetadata = <$tool>::METADATA;
        }
    };
}

/// Default preview caps shared by most tools.
pub const DEFAULT_ARGS_PREVIEW_CAP: usize = 512;
/// Default result preview cap.
pub const DEFAULT_RESULT_PREVIEW_CAP: usize = 2048;

/// Convenience: metadata whose only varying fields are name, description, and
/// the three classes.
#[allow(clippy::too_many_arguments)]
pub const fn base_metadata(
    name: &'static str,
    description: &'static str,
    category: ToolCategory,
    side_effect: SideEffectClass,
    cost: CostClass,
    requires_approval: bool,
) -> ToolMetadata {
    ToolMetadata {
        name,
        description,
        category,
        side_effect,
        cost,
        requires_approval,
        args_preview_cap: DEFAULT_ARGS_PREVIEW_CAP,
        result_preview_cap: DEFAULT_RESULT_PREVIEW_CAP,
    }
}
