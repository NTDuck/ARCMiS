//! `lsp` dispatches language server requests to a pluggable backend.
//!
//! This pass validates arguments and routes each action to the backend. Real
//! language server wiring lands later. The default backend reports that no
//! server is configured.
/// `lsp` sends one request to a language server.
pub struct Lsp {
    /// Language server backend. The orchestrator wires a real implementation.
    pub backend: std::sync::Arc<dyn LspBackend + core::marker::Send + core::marker::Sync>,
}

impl core::fmt::Debug for Lsp {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("Lsp").finish_non_exhaustive()
    }
}

impl rig::tool::Tool for Lsp {
    const NAME: &'static str = "lsp";
    type Error = rig::tool::ToolExecutionError;
    type Args = LspArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> std::string::String {
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

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> core::result::Result<Self::Output, Self::Error> {
        let action = args.action;
        let timeout = clamp_timeout(args.timeout, 20, 5, 60);
        validate(action, &args)?;
        let request = LspRequest {
            file: args.file.clone(),
            line: args.line,
            symbol: args.symbol.clone(),
            query: args.query.clone(),
            new_name: args.new_name.clone(),
            apply: args.apply.unwrap_or(false),
            payload: args.payload.clone(),
            timeout,
        };
        let result = dispatch(self.backend.as_ref(), action, &request).await;
        let action_name = serde_json::to_value(action).unwrap_or(serde_json::Value::Null);
        core::result::Result::Ok(rig::tool::ToolOutput::json(serde_json::json!({
            "action": action_name,
            "result": result,
        })))
    }
}

impl core::default::Default for Lsp {
    fn default() -> Self {
        Self {
            backend: std::sync::Arc::new(NullLspBackend),
        }
    }
}

/// Arguments for `lsp`.
#[derive(Debug, serde::Deserialize)]
pub struct LspArgs {
    /// Language server action to run.
    pub action: LspAction,
    /// File path the action targets.
    pub file: core::option::Option<std::string::String>,
    /// Zero based line number for position actions.
    pub line: core::option::Option<u32>,
    /// Symbol name for symbol scoped actions.
    pub symbol: core::option::Option<std::string::String>,
    /// Raw server request text for the request action.
    pub query: core::option::Option<std::string::String>,
    /// New symbol name for rename actions.
    pub new_name: core::option::Option<std::string::String>,
    /// Apply workspace edits instead of returning them.
    pub apply: core::option::Option<bool>,
    /// Request timeout in seconds.
    pub timeout: core::option::Option<u64>,
    /// Extra fields for the request action.
    pub payload: core::option::Option<serde_json::Value>,
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
    pub file: core::option::Option<std::string::String>,
    pub line: core::option::Option<u32>,
    pub symbol: core::option::Option<std::string::String>,
    pub query: core::option::Option<std::string::String>,
    pub new_name: core::option::Option<std::string::String>,
    pub apply: bool,
    pub payload: core::option::Option<serde_json::Value>,
    pub timeout: u64,
}

/// Boxed future returned by every backend method.
pub type LspFuture<'a> =
    std::pin::Pin<std::boxed::Box<dyn core::future::Future<Output = std::string::String> + core::marker::Send + 'a>>;

/// Pluggable language server boundary. One method per `lsp` action.
pub trait LspBackend: core::marker::Send + core::marker::Sync {
    fn diagnostics(&self, request: &LspRequest) -> LspFuture<'_>;
    fn definition(&self, request: &LspRequest) -> LspFuture<'_>;
    fn references(&self, request: &LspRequest) -> LspFuture<'_>;
    fn hover(&self, request: &LspRequest) -> LspFuture<'_>;
    fn symbols(&self, request: &LspRequest) -> LspFuture<'_>;
    fn rename(&self, request: &LspRequest) -> LspFuture<'_>;
    fn rename_file(&self, request: &LspRequest) -> LspFuture<'_>;
    fn code_actions(&self, request: &LspRequest) -> LspFuture<'_>;
    fn type_definition(&self, request: &LspRequest) -> LspFuture<'_>;
    fn implementation(&self, request: &LspRequest) -> LspFuture<'_>;
    fn status(&self, request: &LspRequest) -> LspFuture<'_>;
    fn reload(&self, request: &LspRequest) -> LspFuture<'_>;
    fn capabilities(&self, request: &LspRequest) -> LspFuture<'_>;
    fn request(&self, request: &LspRequest) -> LspFuture<'_>;
}

/// Fallback backend for the dispatch shell. It reports that no server exists.
#[derive(Debug, Default)]
pub struct NullLspBackend;

impl LspBackend for NullLspBackend {
    fn diagnostics(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn definition(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn references(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn hover(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn symbols(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn rename(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn rename_file(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn code_actions(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn type_definition(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn implementation(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn status(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn reload(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn capabilities(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }

    fn request(&self, request: &LspRequest) -> LspFuture<'_> {
        std::boxed::Box::pin(core::future::ready(unconfigured(request)))
    }
}

/// Build the fallback text for a tool without a real backend.
fn unconfigured(request: &LspRequest) -> std::string::String {
    let file = request.file.as_deref().unwrap_or("<no file>");
    std::format!("no language server configured for {file}")
}

/// Clamp an optional timeout to its default and bounds.
fn clamp_timeout(value: core::option::Option<u64>, default: u64, minimum: u64, maximum: u64) -> u64 {
    value.unwrap_or(default).clamp(minimum, maximum)
}

/// Reject actions whose required arguments are missing.
fn validate(action: LspAction, args: &LspArgs) -> core::result::Result<(), rig::tool::ToolExecutionError> {
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
        LspAction::Status | LspAction::Reload | LspAction::Capabilities => core::result::Result::Ok(()),
    }
}

/// Fail with an invalid args error when a requirement is not met.
fn require(met: bool, message: &str) -> core::result::Result<(), rig::tool::ToolExecutionError> {
    if met {
        core::result::Result::Ok(())
    } else {
        core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(message))
    }
}

/// Route one action to its backend method.
async fn dispatch(backend: &dyn LspBackend, action: LspAction, request: &LspRequest) -> std::string::String {
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
