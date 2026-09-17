//! Tool domain for ARCMiS.
//!
//! The crate exposes these tools:
//!
//! - `read_file` — [`ReadFile`] reads one file from the workspace.
//! - `write_file` — [`WriteFile`] writes one file inside the output workspace.
//! - `run_command` — [`RunCommand`] runs one command in a working directory.

pub mod util {
    //! Support modules for the tool domain. Nothing here implements a
    //! rig tool. The tools live in the crate root.
    pub mod catalog;
    pub mod path;
}

pub mod read_file;
pub mod run_command;
pub mod write_file;

pub use read_file::ReadFile;
pub use run_command::{CommandOutput, RunCommand};
pub use util::catalog::{Catalog, Tool};
pub use util::path::path_sanitize;
pub use write_file::WriteFile;
