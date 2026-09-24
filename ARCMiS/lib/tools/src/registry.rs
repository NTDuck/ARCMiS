//! Tool registry: the single source of truth for what tools exist and what
//! their properties are. The manager constructs it once at startup and passes
//! it to the guard, the router, and the observability controller.
//!
//! The registry erases tools at the metadata level (rig's `Tool` is not
//! object-safe); it answers "what tools exist and what are their properties".
//! Execution stays with the concrete tool instances the host wires into each
//! agent builder.

use crate::envelope::AnyTool;
use crate::envelope::ToolCategory;
use crate::envelope::ToolEnvelope;
use crate::envelope::ToolMetadata;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

/// Registry of envelope-addressable tools.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, AnyTool>,
}

impl ToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one tool's envelope. Replaces a prior entry with the same name.
    pub fn register(&mut self, tool: AnyTool) {
        self.tools.insert(tool.metadata().name, tool);
    }

    /// Look up one tool's envelope by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<AnyTool> {
        self.tools.get(name).cloned()
    }

    /// Metadata for one tool by name.
    #[must_use]
    pub fn metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.tools.get(name).map(|tool| tool.metadata())
    }

    /// All tools, in name order.
    pub fn all(&self) -> impl Iterator<Item = AnyTool> + '_ {
        self.tools.values().cloned()
    }

    /// Tools in one category.
    pub fn by_category(&self, category: ToolCategory) -> impl Iterator<Item = AnyTool> + '_ {
        self.tools.values().filter(move |tool| tool.metadata().category == category).cloned()
    }

    /// Tools with one side-effect class.
    pub fn by_side_effect(&self, side_effect: crate::envelope::SideEffectClass) -> impl Iterator<Item = AnyTool> + '_ {
        self.tools.values().filter(move |tool| tool.metadata().side_effect == side_effect).cloned()
    }

    /// Number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry holds no tools.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Probe for a tool type's inherent `const METADATA`.
///
/// The tool files opt in through `impl_envelope!`.
pub trait ConstMetadata {
    /// The tool's static metadata.
    const METADATA: ToolMetadata;
}

/// Metadata-only envelope over a tool type with `const METADATA`.
///
/// The zero-sized marker is unconditionally `Send + Sync` because
/// `PhantomData<fn() -> T>` owns no data; the tool instance stays with the
/// rig agent.
struct EnvelopeOf<T>(PhantomData<fn() -> T>);

impl<T> ToolEnvelope for EnvelopeOf<T>
where
    T: ConstMetadata,
{
    fn metadata(&self) -> ToolMetadata {
        T::METADATA
    }
}

unsafe impl<T> Send for EnvelopeOf<T> {}
unsafe impl<T> Sync for EnvelopeOf<T> {}

/// Erase one tool's metadata into the registry without holding the tool.
fn envelope_of<T>() -> AnyTool
where
    T: ConstMetadata + 'static,
{
    Arc::new(EnvelopeOf::<T>(PhantomData))
}

/// Build the default registry with every filesystem, search, runtime, and
/// coordination tool in this crate.
#[must_use]
pub fn default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    macro_rules! put {
        ($t:ty) => {
            registry.register(envelope_of::<$t>());
        };
    }
    put!(crate::read::Read);
    put!(crate::write::Write);
    put!(crate::edit::Edit);
    put!(crate::search::Search);
    put!(crate::find::Find);
    put!(crate::ast_grep::AstGrep);
    put!(crate::ast_edit::AstEdit);
    put!(crate::bash::Bash);
    put!(crate::eval::Eval);
    put!(crate::ssh::Ssh);
    put!(crate::lsp::Lsp);
    put!(crate::debug::Debug);
    put!(crate::task::Task);
    put!(crate::irc::Irc);
    put!(crate::todo::Todo);
    put!(crate::job::Job);
    put!(crate::ask::Ask);
    registry
}
