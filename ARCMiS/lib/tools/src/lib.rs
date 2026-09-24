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

pub use ask::Ask;
pub use ask::AskArgs;
pub use ast_edit::AstEdit;
pub use ast_grep::AstGrep;
pub use bash::Bash;
pub use debug::Debug;
pub use edit::Edit;
pub use edit::EditArgs;
pub use eval::Eval;
pub use find::Find;
pub use irc::Irc;
pub use job::Job;
pub use lsp::Lsp;
pub use portable::Named;
pub use portable::build_tools;
pub use read::Read;
pub use read::ReadArgs;
pub use search::Search;
pub use ssh::Ssh;
pub use task::Task;
pub use todo::Todo;
pub use util::catalog::Catalog;
pub use util::catalog::Tool;
pub use util::jobs::JobRegistry;
pub use util::path::path_sanitize;
pub use util::snapshots::InMemorySnapshotStore;
pub use util::snapshots::SnapshotStore;
pub use write::Write;
pub use write::WriteArgs;
