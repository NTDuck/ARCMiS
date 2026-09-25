//! Portable dynamic adapters: wrap every concrete tool in a JSON-in,
//! text-out `PortableDynamicTool` so the MAS registry can register the exact
//! allowlist per role without typed plumbing.

use std::future::Future;

use crate::ast_edit::AstEdit;
use crate::ast_grep::AstGrep;
use crate::bash::Bash;
use crate::edit::Edit;
use crate::eval::Eval;
use crate::find::Find;
use crate::lsp::Lsp;
use crate::read::Read;
use crate::search::Search;
use crate::write::Write;
use oxi_hashline::InMemorySnapshotStore;
use rig::tool::PortableDynamicTool;
use std::path::PathBuf;
use std::sync::Arc;

/// One dynamic tool factory output: the adapter plus its name.
pub struct Named {
    /// Registered tool name.
    pub name: &'static str,
    /// The portable adapter.
    pub tool: PortableDynamicTool,
}

/// Build the tool set for one role allowlist. `root` is the sandbox root;
/// every path tool resolves inside it. Tools named in `allow` are registered.
#[must_use]
pub fn build_tools(root: &PathBuf, allow: &[&str]) -> Vec<Named> {
    let snapshots: Arc<dyn oxi_hashline::SnapshotStore> = Arc::new(InMemorySnapshotStore::new());
    let mut tools = Vec::new();

    if allow.contains(&"read") {
        let read = Arc::new(Read {
            root: root.clone(),
            snapshots: snapshots.clone(),
        });
        let read_for_call = read.clone();
        tools.push(named(&read, move |args| {
            let read = read_for_call.clone();
            async move { execute(read.as_ref(), args).await }
        }));
    }
    if allow.contains(&"write") {
        let write = Arc::new(Write {
            root: root.clone(),
            snapshots: snapshots.clone(),
        });
        let write_for_call = write.clone();
        tools.push(named(&write, move |args| {
            let write = write_for_call.clone();
            async move { execute(write.as_ref(), args).await }
        }));
    }
    if allow.contains(&"edit") {
        let edit = Edit::new(root.clone(), snapshots.clone());
        let edit_for_call = edit.clone();
        tools.push(named(&edit, move |args| {
            let edit = edit_for_call.clone();
            async move { execute(edit.as_ref(), args).await }
        }));
    }
    if allow.contains(&"search") {
        let search = Arc::new(Search {
            root: root.clone(),
            snapshots: snapshots.clone(),
        });
        let search_for_call = search.clone();
        tools.push(named(&search, move |args| {
            let search = search_for_call.clone();
            async move { execute(search.as_ref(), args).await }
        }));
    }
    if allow.contains(&"find") {
        let find = Arc::new(Find {
            root: root.clone(),
        });
        let find_for_call = find.clone();
        tools.push(named(&find, move |args| {
            let find = find_for_call.clone();
            async move { execute(find.as_ref(), args).await }
        }));
    }
    if allow.contains(&"ast_grep") {
        let ast_grep = Arc::new(AstGrep {
            root: root.clone(),
        });
        let ast_grep_for_call = ast_grep.clone();
        tools.push(named(&ast_grep, move |args| {
            let ast_grep = ast_grep_for_call.clone();
            async move { execute(ast_grep.as_ref(), args).await }
        }));
    }
    if allow.contains(&"ast_edit") {
        let ast_edit = Arc::new(AstEdit {
            root: root.clone(),
        });
        let ast_edit_for_call = ast_edit.clone();
        tools.push(named(&ast_edit, move |args| {
            let ast_edit = ast_edit_for_call.clone();
            async move { execute(ast_edit.as_ref(), args).await }
        }));
    }
    if allow.contains(&"bash") {
        let bash = Arc::new(Bash {
            root: root.clone(),
        });
        let bash_for_call = bash.clone();
        tools.push(named(&bash, move |args| {
            let bash = bash_for_call.clone();
            async move { execute(bash.as_ref(), args).await }
        }));
    }
    if allow.contains(&"lsp") {
        let lsp = Arc::new(Lsp::default());
        let lsp_for_call = lsp.clone();
        tools.push(named(&lsp, move |args| {
            let lsp = lsp_for_call.clone();
            async move { execute(lsp.as_ref(), args).await }
        }));
    }
    if allow.contains(&"eval") {
        let eval = Arc::new(Eval {});
        let eval_for_call = eval.clone();
        tools.push(named(&eval, move |args| {
            let eval = eval_for_call.clone();
            async move { execute(eval.as_ref(), args).await }
        }));
    }
    tools
}

/// Build one named adapter from a tool's metadata and an execute closure.
fn named<T, F, Fut>(tool: &Arc<T>, callback: F) -> Named
where
    T: rig::tool::Tool<
            Args: serde::de::DeserializeOwned,
            Output = rig::tool::ToolOutput,
            Error = rig::tool::ToolExecutionError,
        > + 'static,
    F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<rig::tool::ToolOutput, rig::tool::ToolExecutionError>> + Send + 'static,
{
    Named {
        name: T::NAME,
        tool: PortableDynamicTool::new(T::NAME, tool.description(), tool.parameters(), move |args| {
            let callback = &callback;
            Box::pin(callback(args))
        }),
    }
}

/// Parse `args` into the tool's typed args and call it with a fresh empty
/// context. Every tool in this crate holds its state in its own fields, so
/// the per-call context carries nothing.
async fn execute<T>(tool: &T, args: serde_json::Value) -> Result<rig::tool::ToolOutput, rig::tool::ToolExecutionError>
where
    T: rig::tool::Tool<
        Args: serde::de::DeserializeOwned,
        Output = rig::tool::ToolOutput,
        Error = rig::tool::ToolExecutionError,
    >,
{
    let args: T::Args =
        serde_json::from_value(args).map_err(|error| rig::tool::ToolExecutionError::from_error(error))?;
    tool.call(&mut rig::tool::ToolContext::new(), args).await
}
