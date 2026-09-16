//! Tool domain for ARCMiS.
//!
//! The crate exposes these tools:
//!
//! - `read_file` — [`ReadFile`] reads one file from the workspace.
//! - `write_file` — [`WriteFile`] writes one file inside the output workspace.
//! - `run_command` — [`RunCommand`] runs one command in a working directory.

pub mod catalog;
pub mod path;
pub mod read_file;
pub mod run_command;
pub mod write_file;

pub use catalog::{Catalog, Tool};
pub use path::path_sanitize;
pub use read_file::ReadFile;
pub use run_command::{CommandOutput, RunCommand};
pub use write_file::WriteFile;
