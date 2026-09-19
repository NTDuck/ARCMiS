//! `lsp` dispatches language server requests to a pluggable backend.
//!
//! This pass validates arguments and routes each action to the backend. Real
//! language server wiring lands later. The default backend reports that no
//! server is configured.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use std::fmt;
use std::future::{ready, Future};
use std::pin::Pin;
use std::sync::Arc;

/// `lsp` sends one request to a language server.
pub struct Lsp {
    /// Language server backend. The orchestrator wires a real implementation.
    pub backend: Arc<dyn LspBackend + Send + Sync>,
}

impl fmt::Debug for Lsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Lsp").finish_non_exhaustive()
    }
}

impl Tool for Lsp {
    const NAME: &'static str = "lsp";
    type Error = ToolExecutionError;
    type Args = LspArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Send one request to a language server and return the server response.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "diagnostics", "definition", "references", "hover", "symbols",
                        "rename", "rename_file", "code_actions", "type_definition",
                        "implementation", "status", "reload", "capabilities", "request"
                    ],
                    "description": "Language server action to run"
                },
                "file": { "type": "string", "description": "File path the action targets" },
                "line": { "type": "integer", "description": "Zero based line number for position actions" },
                "symbol": { "type": "string", "description": "Symbol name for symbol scoped actions" },
                "query": { "type": "string", "description": "Raw server request text for the request action" },
                "new_name": { "type": "string", "description": "New symbol name for rename actions" },
                "apply": { "type": "boolean", "description": "Apply workspace edits instead of returning them" },
                "timeout": { "type": "integer", "description": "Request timeout in seconds, clamped to 5 through 60" },
                "payload": { "type": "object", "description": "Extra fields for the request action" }
            },
            "required": ["action"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let action = args.action;
        let timeout = clamp_timeout(args.timeout, 20, 5, 60);
        validate(action, &args)?;
        let request = LspRequest {
            file: args.file,
            line: args.line,
            symbol: args.symbol,
            query: args.query,
            new_name: args.new_name,
            apply: args.apply.unwrap_or(false),
            payload: args.payload,
            timeout,
        };
        let result = dispatch(self.backend.as_ref(), action, &request).await;
        let action_name = serde_json::to_value(action).unwrap_or(serde_json::Value::Null);
        Ok(ToolOutput::json(serde_json::json!({
            "action": action_name,
            "result": result,
        })))
    }
}

impl Default for Lsp {
    fn default() -> Self {
        Self {
            backend: Arc::new(NullLspBackend),
        }
    }
}

/// Arguments for `lsp`.
#[derive(Debug, serde::Deserialize)]
pub struct LspArgs {
    /// Language server action to run.
    pub action: LspAction,
    /// File path the action targets.
    pub file: Option<String>,
    /// Zero based line number for position actions.
    pub line: Option<u32>,
    /// Symbol name for symbol scoped actions.
    pub symbol: Option<String>,
    /// Raw server request text for the request action.
    pub query: Option<String>,
    /// New symbol name for rename actions.
    pub new_name: Option<String>,
    /// Apply workspace edits instead of returning them.
    pub apply: Option<bool>,
    /// Request timeout in seconds.
    pub timeout: Option<u64>,
    /// Extra fields for the request action.
    pub payload: Option<serde_json::Value>,
}

/// One `lsp` action.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspAction {
    Diagnostics,
    Definition,
    References,
    Hover,
    Symbols,
    Rename,
    RenameFile,
    CodeActions,
    TypeDefinition,
    Implementation,
    Status,
    Reload,
    Capabilities,
    Request,
}

/// Normalized request values passed to the backend.
#[derive(Debug, Clone)]
pub struct LspRequest {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub symbol: Option<String>,
    pub query: Option<String>,
    pub new_name: Option<String>,
    pub apply: bool,
    pub payload: Option<serde_json::Value>,
    pub timeout: u64,
}

/// Boxed future returned by every backend method.
pub type LspFuture<'a> = Pin<Box<dyn Future<Output = String> + Send + 'a>>;

/// Pluggable language server boundary. One method per `lsp` action.
pub trait LspBackend: Send + Sync {
    fn diagnostics<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn definition<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn references<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn hover<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn symbols<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn rename<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn rename_file<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn code_actions<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn type_definition<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn implementation<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn status<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn reload<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn capabilities<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
    fn request<'a>(&self, request: &'a LspRequest) -> LspFuture<'a>;
}

/// Fallback backend for the dispatch shell. It reports that no server exists.
#[derive(Debug, Default)]
pub struct NullLspBackend;

impl LspBackend for NullLspBackend {
    fn diagnostics<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn definition<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn references<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn hover<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn symbols<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn rename<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn rename_file<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn code_actions<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn type_definition<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn implementation<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn status<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn reload<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn capabilities<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }

    fn request<'a>(&self, request: &'a LspRequest) -> LspFuture<'a> {
        unconfigured_result(request)
    }
}

/// Build the fallback text for a tool without a real backend.
/// One unconfigured backend result: the model sees the notice text.
fn unconfigured_result<'a>(request: &'a LspRequest) -> LspFuture<'a> {
    let text = unconfigured(request);
    Box::pin(ready(text))
}

fn unconfigured(request: &LspRequest) -> String {
    let file = request.file.as_deref().unwrap_or("<no file>");
    format!("no language server configured for {file}")
}

/// Clamp an optional timeout to its default and bounds.
fn clamp_timeout(value: Option<u64>, default: u64, minimum: u64, maximum: u64) -> u64 {
    value.unwrap_or(default).clamp(minimum, maximum)
}

/// Reject actions whose required arguments are missing.
fn validate(action: LspAction, args: &LspArgs) -> Result<(), ToolExecutionError> {
    match action {
        LspAction::Rename | LspAction::RenameFile => {
            require(args.new_name.is_some(), "new_name is required for a rename action")
        },
        LspAction::Request => require(args.query.is_some(), "query is required for the request action"),
        LspAction::Diagnostics
        | LspAction::Definition
        | LspAction::References
        | LspAction::Hover
        | LspAction::Symbols
        | LspAction::CodeActions
        | LspAction::TypeDefinition
        | LspAction::Implementation => require(args.file.is_some(), "file is required for the requested action"),
        LspAction::Status | LspAction::Reload | LspAction::Capabilities => Ok(()),
    }
}

/// Fail with an invalid args error when a requirement is not met.
fn require(met: bool, message: &str) -> Result<(), ToolExecutionError> {
    if met {
        Ok(())
    } else {
        Err(ToolExecutionError::invalid_args(message))
    }
}

/// Route one action to its backend method.
async fn dispatch(backend: &dyn LspBackend, action: LspAction, request: &LspRequest) -> String {
    match action {
        LspAction::Diagnostics => backend.diagnostics(request).await,
        LspAction::Definition => backend.definition(request).await,
        LspAction::References => backend.references(request).await,
        LspAction::Hover => backend.hover(request).await,
        LspAction::Symbols => backend.symbols(request).await,
        LspAction::Rename => backend.rename(request).await,
        LspAction::RenameFile => backend.rename_file(request).await,
        LspAction::CodeActions => backend.code_actions(request).await,
        LspAction::TypeDefinition => backend.type_definition(request).await,
        LspAction::Implementation => backend.implementation(request).await,
        LspAction::Status => backend.status(request).await,
        LspAction::Reload => backend.reload(request).await,
        LspAction::Capabilities => backend.capabilities(request).await,
        LspAction::Request => backend.request(request).await,
    }
}
