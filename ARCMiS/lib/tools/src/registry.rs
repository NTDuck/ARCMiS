//! Tool registry: the single source of truth for what tools exist and what
//! their properties are. The harness constructs it once at startup and passes
//! it to the guard, the router, and the observability controller.
//!
//! The registry erases tools at the metadata level (rig's `Tool` is not
//! object-safe); it answers "what tools exist and what are their properties".
//! Execution stays with the concrete tool instances the host wires into each
//! agent builder.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::envelope::AnyTool;
use crate::envelope::ToolCategory;
use crate::envelope::ToolEnvelope;
use crate::envelope::ToolMetadata;

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

/// Register one tool envelope into the registry.
#[macro_export]
macro_rules! put {
    ($registry:expr, $t:ty) => {
        $registry.register(envelope_of::<$t>());
    };
}

/// Build the default registry with every filesystem, search, runtime, and
/// coordination tool in this crate.
#[must_use]
pub fn default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    crate::put!(registry, crate::read::Read);
    crate::put!(registry, crate::write::Write);
    crate::put!(registry, crate::edit::Edit);
    crate::put!(registry, crate::search::Search);
    crate::put!(registry, crate::find::Find);
    crate::put!(registry, crate::ast_grep::AstGrep);
    crate::put!(registry, crate::ast_edit::AstEdit);
    crate::put!(registry, crate::bash::Bash);
    crate::put!(registry, crate::eval::Eval);
    crate::put!(registry, crate::ssh::Ssh);
    crate::put!(registry, crate::lsp::Lsp);
    crate::put!(registry, crate::debug::Debug);
    crate::put!(registry, crate::task::Task);
    crate::put!(registry, crate::irc::Irc);
    crate::put!(registry, crate::todo::Todo);
    crate::put!(registry, crate::job::Job);
    crate::put!(registry, crate::ask::Ask);
    registry
}
