//! Tool domain for ARCMiS.
//!
//! The crate exposes these tools, grouped like oh-my-pi:
//!
//! Files and search: [`read`], [`write`], [`edit`], [`search`], [`find`],
//! [`ast_grep`], [`ast_edit`]. Runtime: [`bash`], [`eval`], [`ssh`].
//! Code intelligence: [`lsp`], [`debug`]. Coordination: [`task`], [`irc`],
//! [`todo`], [`job`], [`ask`].
//!
//! Every tool implements [`rig::tool::Tool`]. Stateful tools hold their
//! shared state behind `Arc` fields constructed by the host.
pub mod util {
    //! Support modules for the tool domain. Nothing here implements a
    //! rig tool. The tools live in the crate root.
    pub mod catalog;
    pub mod jobs;
    pub mod path;
    pub mod paths;
    pub mod proc;
    pub mod snapshots;
}

pub mod ask;
pub mod ast_edit;
pub mod ast_grep;
pub mod bash;
pub mod debug;
pub mod edit;
pub mod envelope;
pub mod eval;
pub mod find;
pub mod irc;
pub mod job;
pub mod lsp;
pub mod portable;
pub mod read;
pub mod registry;
pub mod search;
pub mod ssh;
pub mod task;
pub mod todo;
pub mod write;

// Re-exports kept to the symbols with consumers outside their module:
// `Edit`/`Read`/`Write` (tests), `build_tools`/`Named` (agents, harness).
// Everything else imports through its module path (`tools::read::ReadArgs`).
pub use edit::Edit;
pub use portable::build_tools;
pub use portable::Named;
pub use read::Read;
pub use write::Write;
