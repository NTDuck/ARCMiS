//! Tool guard: the allowlist and source-readonly enforcement between a
//! specialist and its tools. The guard checks every call the registry's
//! metadata marks as a write against the role's allowlist and the workspace
//! rule (source/ is read-only).

use blackboard::Workspace;

/// Guard over one role's tool calls.
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

    /// Whether the role may write this path. Read-only tools pass; write
    /// tools must target outside `source/`.
    pub fn permits_path(&self, tool_name: &str, path: &std::path::Path) -> anyhow::Result<()> {
        anyhow::ensure!(self.permits_tool(tool_name), "tool '{tool_name}' is not allowed for this role");
        self.workspace.assert_source_readonly(path)
    }
}

/// Wrap one write-capable tool with the guard. The wrapper rejects calls
/// before the tool runs; reads pass through untouched.
pub struct GuardedTool<T> {
    /// The wrapped tool.
    inner: T,
    /// Tool name for allowlist checks.
    name: &'static str,
    /// The guard.
    guard: std::sync::Arc<Guard>,
}

impl<T> GuardedTool<T> {
    /// Wrap `tool` under `guard`.
    #[must_use]
    pub fn new(inner: T, name: &'static str, guard: std::sync::Arc<Guard>) -> Self {
        Self {
            inner,
            name,
            guard,
        }
    }
}

/// Error the guard produces.
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    /// The tool call was rejected.
    #[error("{0}")]
    Rejected(String),
    /// The tool itself failed.
    #[error(transparent)]
    Tool(#[from] anyhow::Error),
}

impl<T> rig::tool::Tool for GuardedTool<T>
where
    T: rig::tool::Tool + Send + Sync,
    T::Args: serde::Serialize,
    T::Error: From<ToolGuardError>,
{
    const NAME: &'static str = T::NAME;
    type Args = T::Args;
    type Output = T::Output;
    type Error = T::Error;

    fn description(&self) -> String {
        self.inner.description()
    }

    fn parameters(&self) -> serde_json::Value {
        self.inner.parameters()
    }

    async fn call(&self, context: &mut rig::tool::ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Path checks need the path out of the args; every write-capable tool
        // in this crate carries `path` or `file` as its target argument.
        if let Some(path) = arg_path(&args) {
            if let Err(error) = self.guard.permits_path(self.name, &path) {
                return Err(T::Error::from(ToolGuardError::new(error.to_string())));
            }
        }
        self.inner.call(context, args).await
    }
}

/// Bridge error: the guard rejected the call before the tool ran. Write
/// tools adopt this through `impl From<ToolGuardError> for ToolError`.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolGuardError(#[from] anyhow::Error);

impl ToolGuardError {
    /// Build a rejection from a message.
    #[must_use]
    pub fn new(message: String) -> Self {
        Self(anyhow::anyhow!(message))
    }
}

/// Extract the write target from common tool arg shapes. Tools without a
/// path-shaped target return `None` and skip the path check.
fn arg_path<T: serde::de::DeserializeOwned + serde::Serialize>(args: &T) -> Option<std::path::PathBuf> {
    let value = serde_json::to_value(args).ok()?;
    let path = value.get("path").or_else(|| value.get("file"))?.as_str()?;
    Some(std::path::PathBuf::from(path))
}
