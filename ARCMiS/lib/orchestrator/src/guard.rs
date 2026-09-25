//! Tool guard: the allowlist and source-readonly enforcement between a
//! specialist and its tools. The guard checks each path-shaped tool argument
//! against the role's allowlist and the workspace rule (source/ is
//! read-only).

use blackboard::Workspace;
use std::path::Path;

/// Guard over one role's tool calls.
#[derive(Clone)]
pub struct Guard {
    /// Role's allowed tool names.
    allowed: Vec<&'static str>,
    /// Workspace whose source/ tree is read-only.
    workspace: Workspace,
}

impl Guard {
    /// Build a guard for one role.
    #[must_use]
    pub fn new(allowed: Vec<&'static str>, workspace: Workspace) -> Self {
        Self {
            allowed,
            workspace,
        }
    }

    /// Whether the role may call this tool at all.
    #[must_use]
    pub fn permits_tool(&self, name: &str) -> bool {
        self.allowed.contains(&name)
    }

    /// The role's allowed tool names.
    #[must_use]
    pub fn allowed(&self) -> &[&'static str] {
        &self.allowed
    }

    /// Whether the role may write this path. Read-only tools pass; write
    /// tools must target outside `source/`.
    pub fn permits_path(&self, tool_name: &str, path: &Path) -> anyhow::Result<()> {
        anyhow::ensure!(self.permits_tool(tool_name), "tool '{tool_name}' is not allowed for this role");
        self.workspace.assert_source_readonly(path)
    }
}
