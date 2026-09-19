//! `debug` drives one debug adapter session through a pluggable backend.
//!
//! This pass validates arguments, enforces one active session, and routes each
//! action to a backend group. Real adapter wiring lands later. The default
//! backend reports that no adapter is configured.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::{ready, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

/// `debug` runs one debug adapter operation.
pub struct Debug {
    /// Debug adapter backend. The orchestrator wires a real implementation.
    pub backend: Arc<dyn DapBackend + Send + Sync>,
    /// Identifier of the active session, if one exists.
    pub session: Arc<Mutex<Option<String>>>,
}

impl fmt::Debug for Debug {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Debug").finish_non_exhaustive()
    }
}

impl Tool for Debug {
    const NAME: &'static str = "debug";
    type Error = ToolExecutionError;
    type Args = DebugArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Run one debug adapter operation and return the adapter response.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "launch", "attach", "set_breakpoint", "remove_breakpoint",
                        "set_instruction_breakpoint", "remove_instruction_breakpoint",
                        "set_data_breakpoint", "remove_data_breakpoint",
                        "data_breakpoint_info", "continue", "step_over", "step_in",
                        "step_out", "pause", "evaluate", "stack_trace", "threads",
                        "scopes", "variables", "disassemble", "read_memory",
                        "write_memory", "modules", "loaded_sources", "custom_request",
                        "output", "terminate", "sessions"
                    ],
                    "description": "Debug adapter operation to run"
                },
                "program": { "type": "string", "description": "Program path for launch" },
                "args": { "type": "array", "items": { "type": "string" }, "description": "Program arguments for launch" },
                "adapter": { "type": "string", "description": "Configured adapter identifier" },
                "cwd": { "type": "string", "description": "Working directory for the session" },
                "file": { "type": "string", "description": "Source file for breakpoint actions" },
                "line": { "type": "integer", "description": "Source line for breakpoint actions" },
                "function": { "type": "string", "description": "Function name for function breakpoints" },
                "name": { "type": "string", "description": "Breakpoint or variable name" },
                "condition": { "type": "string", "description": "Breakpoint condition expression" },
                "hit_condition": { "type": "string", "description": "Breakpoint hit count expression" },
                "expression": { "type": "string", "description": "Expression for evaluate" },
                "context": { "type": "string", "description": "Evaluate context, default repl" },
                "frame_id": { "type": "integer", "description": "Stack frame reference" },
                "scope_id": { "type": "integer", "description": "Scope reference for variables" },
                "variable_ref": { "type": "integer", "description": "Variable reference handle" },
                "pid": { "type": "integer", "description": "Process id for attach" },
                "port": { "type": "integer", "description": "Remote attach port" },
                "host": { "type": "string", "description": "Remote attach host" },
                "levels": { "type": "integer", "description": "Maximum stack frames" },
                "memory_reference": { "type": "string", "description": "Memory reference for memory actions" },
                "instruction_reference": { "type": "string", "description": "Instruction reference for disassemble" },
                "instruction_count": { "type": "integer", "description": "Instruction count for disassemble" },
                "instruction_offset": { "type": "integer", "description": "Instruction offset for disassemble" },
                "count": { "type": "integer", "description": "Byte count for read_memory" },
                "data": { "type": "string", "description": "Base64 memory payload for write_memory" },
                "data_id": { "type": "string", "description": "Data breakpoint identifier" },
                "access_type": { "type": "string", "enum": ["read", "write", "readWrite"], "description": "Access type for data breakpoints" },
                "command": { "type": "string", "description": "Custom DAP request command" },
                "arguments": { "type": "object", "description": "Custom DAP request arguments" },
                "offset": { "type": "integer", "description": "Generic offset value" },
                "resolve_symbols": { "type": "boolean", "description": "Resolve symbols when loading modules" },
                "allow_partial": { "type": "boolean", "description": "Accept partial results" },
                "start_module": { "type": "integer", "description": "First module index" },
                "module_count": { "type": "integer", "description": "Module count" },
                "timeout": { "type": "integer", "description": "Operation timeout in seconds, clamped to 5 through 300" }
            },
            "required": ["action"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let action = args.action;
        let timeout = args.timeout.unwrap_or(30).clamp(5, 300);
        validate(action, &args)?;
        let request = DebugRequest {
            action,
            program: args.program,
            args: args.args.unwrap_or_default(),
            adapter: args.adapter,
            cwd: args.cwd,
            file: args.file,
            line: args.line,
            function: args.function,
            name: args.name,
            condition: args.condition,
            hit_condition: args.hit_condition,
            expression: args.expression,
            context: args.context.unwrap_or_else(|| "repl".to_owned()),
            frame_id: args.frame_id,
            scope_id: args.scope_id,
            variable_ref: args.variable_ref,
            pid: args.pid,
            port: args.port,
            host: args.host,
            levels: args.levels,
            memory_reference: args.memory_reference,
            instruction_reference: args.instruction_reference,
            instruction_count: args.instruction_count,
            instruction_offset: args.instruction_offset,
            count: args.count,
            data: args.data,
            data_id: args.data_id,
            access_type: args.access_type,
            command: args.command,
            arguments: args.arguments,
            offset: args.offset,
            resolve_symbols: args.resolve_symbols.unwrap_or(false),
            allow_partial: args.allow_partial.unwrap_or(false),
            start_module: args.start_module,
            module_count: args.module_count,
            timeout,
        };
        self.claim_session(action, &request)?;
        let result = dispatch(self.backend.as_ref(), action, &request).await;
        if matches!(action, DebugAction::Terminate) {
            self.release_session();
        }
        let action_name = serde_json::to_value(action).unwrap_or(serde_json::Value::Null);
        Ok(ToolOutput::json(serde_json::json!({
            "action": action_name,
            "result": result,
        })))
    }
}

impl Debug {
    /// Record the session for launch and attach. Reject a second live session.
    fn claim_session(&self, action: DebugAction, request: &DebugRequest) -> Result<(), ToolExecutionError> {
        let starts = matches!(action, DebugAction::Launch | DebugAction::Attach);
        if !starts {
            return Ok(());
        }
        let mut session = self.session.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(active) = session.as_ref() {
            return Err(ToolExecutionError::other(format!(
                "Debug session {active} is still active. Terminate it before launching another."
            )));
        }
        *session = Some(session_id(request));
        Ok(())
    }

    /// Clear the session record after terminate.
    fn release_session(&self) {
        let mut session = self.session.lock().unwrap_or_else(|error| error.into_inner());
        *session = None;
    }
}

/// Arguments for `debug`.
#[derive(Debug, Deserialize)]
pub struct DebugArgs {
    /// Debug adapter operation to run.
    pub action: DebugAction,
    /// Program path for launch.
    pub program: Option<String>,
    /// Program arguments for launch.
    pub args: Option<Vec<String>>,
    /// Configured adapter identifier.
    pub adapter: Option<String>,
    /// Working directory for the session.
    pub cwd: Option<String>,
    /// Source file for breakpoint actions.
    pub file: Option<String>,
    /// Source line for breakpoint actions.
    pub line: Option<u32>,
    /// Function name for function breakpoints.
    pub function: Option<String>,
    /// Breakpoint or variable name.
    pub name: Option<String>,
    /// Breakpoint condition expression.
    pub condition: Option<String>,
    /// Breakpoint hit count expression.
    pub hit_condition: Option<String>,
    /// Expression for evaluate.
    pub expression: Option<String>,
    /// Evaluate context.
    pub context: Option<String>,
    /// Stack frame reference.
    pub frame_id: Option<u64>,
    /// Scope reference for variables.
    pub scope_id: Option<u64>,
    /// Variable reference handle.
    pub variable_ref: Option<u64>,
    /// Process id for attach.
    pub pid: Option<u32>,
    /// Remote attach port.
    pub port: Option<u16>,
    /// Remote attach host.
    pub host: Option<String>,
    /// Maximum stack frames.
    pub levels: Option<u32>,
    /// Memory reference for memory actions.
    pub memory_reference: Option<String>,
    /// Instruction reference for disassemble.
    pub instruction_reference: Option<String>,
    /// Instruction count for disassemble.
    pub instruction_count: Option<u32>,
    /// Instruction offset for disassemble.
    pub instruction_offset: Option<i64>,
    /// Byte count for read_memory.
    pub count: Option<u32>,
    /// Base64 memory payload for write_memory.
    pub data: Option<String>,
    /// Data breakpoint identifier.
    pub data_id: Option<String>,
    /// Access type for data breakpoints.
    pub access_type: Option<DebugAccessType>,
    /// Custom DAP request command.
    pub command: Option<String>,
    /// Custom DAP request arguments.
    pub arguments: Option<serde_json::Value>,
    /// Generic offset value.
    pub offset: Option<i64>,
    /// Resolve symbols when loading modules.
    pub resolve_symbols: Option<bool>,
    /// Accept partial results.
    pub allow_partial: Option<bool>,
    /// First module index.
    pub start_module: Option<u32>,
    /// Module count.
    pub module_count: Option<u32>,
    /// Operation timeout in seconds.
    pub timeout: Option<u64>,
}

/// One `debug` operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugAction {
    Launch,
    Attach,
    SetBreakpoint,
    RemoveBreakpoint,
    SetInstructionBreakpoint,
    RemoveInstructionBreakpoint,
    SetDataBreakpoint,
    RemoveDataBreakpoint,
    DataBreakpointInfo,
    Continue,
    StepOver,
    StepIn,
    StepOut,
    Pause,
    Evaluate,
    StackTrace,
    Threads,
    Scopes,
    Variables,
    Disassemble,
    ReadMemory,
    WriteMemory,
    Modules,
    LoadedSources,
    CustomRequest,
    Output,
    Terminate,
    Sessions,
}

/// Access kind for a data breakpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DebugAccessType {
    Read,
    Write,
    ReadWrite,
}

/// Normalized request values passed to the backend.
#[derive(Debug, Clone)]
pub struct DebugRequest {
    pub action: DebugAction,
    pub program: Option<String>,
    pub args: Vec<String>,
    pub adapter: Option<String>,
    pub cwd: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub function: Option<String>,
    pub name: Option<String>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub expression: Option<String>,
    pub context: String,
    pub frame_id: Option<u64>,
    pub scope_id: Option<u64>,
    pub variable_ref: Option<u64>,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub levels: Option<u32>,
    pub memory_reference: Option<String>,
    pub instruction_reference: Option<String>,
    pub instruction_count: Option<u32>,
    pub instruction_offset: Option<i64>,
    pub count: Option<u32>,
    pub data: Option<String>,
    pub data_id: Option<String>,
    pub access_type: Option<DebugAccessType>,
    pub command: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub offset: Option<i64>,
    pub resolve_symbols: bool,
    pub allow_partial: bool,
    pub start_module: Option<u32>,
    pub module_count: Option<u32>,
    pub timeout: u64,
}

/// Boxed future returned by every backend method.
pub type DebugFuture<'a> = Pin<Box<dyn Future<Output = String> + Send + 'a>>;

/// Pluggable debug adapter boundary. One method per action group.
pub trait DapBackend: Send + Sync {
    fn launch<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn attach<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn breakpoints<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn stepping<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn inspect<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn memory<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn modules<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
    fn control<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a>;
}

/// Fallback backend for the dispatch shell. It reports that no adapter exists.
#[derive(Debug, Default)]
pub struct NullDapBackend;

impl DapBackend for NullDapBackend {
    fn launch<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn attach<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn breakpoints<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn stepping<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn inspect<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn memory<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn modules<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }

    fn control<'a>(&self, request: &'a DebugRequest) -> DebugFuture<'a> {
        unconfigured_result(request)
    }
}

/// One unconfigured backend result: the model sees the notice text.
fn unconfigured_result<'a>(request: &'a DebugRequest) -> DebugFuture<'a> {
    let text = unconfigured(request);
    Box::pin(ready(text))
}

fn unconfigured(request: &DebugRequest) -> String {
    let target = request.program.as_deref().unwrap_or("<no program>");
    format!("no debug adapter configured for {target}")
}

/// Derive the session identifier for a launch or attach request.
fn session_id(request: &DebugRequest) -> String {
    if let Some(program) = request.program.as_deref() {
        return program.to_owned();
    }
    if let Some(pid) = request.pid {
        return format!("pid-{pid}");
    }
    match (request.host.as_deref(), request.port) {
        (Some(host), Some(port)) => {
            format!("{host}:{port}")
        },
        (_, Some(port)) => format!("port-{port}"),
        _ => "default".to_owned(),
    }
}

/// Reject operations whose required arguments are missing.
fn validate(action: DebugAction, args: &DebugArgs) -> Result<(), ToolExecutionError> {
    match action {
        DebugAction::Launch => require(args.program.is_some(), "program is required for launch"),
        DebugAction::Attach => require(args.pid.is_some() || args.port.is_some(), "pid or port is required for attach"),
        DebugAction::SetBreakpoint => require(
            (args.file.is_some() && args.line.is_some()) || args.function.is_some(),
            "file with line or function is required for set_breakpoint",
        ),
        DebugAction::Evaluate => require(args.expression.is_some(), "expression is required for evaluate"),
        DebugAction::Variables => require(
            args.variable_ref.is_some() || args.scope_id.is_some(),
            "variable_ref or scope_id is required for variables",
        ),
        DebugAction::ReadMemory => require(
            args.memory_reference.is_some() && args.count.is_some(),
            "memory_reference and count are required for read_memory",
        ),
        DebugAction::WriteMemory => require(
            args.memory_reference.is_some() && args.data.is_some(),
            "memory_reference and data are required for write_memory",
        ),
        DebugAction::CustomRequest => require(args.command.is_some(), "command is required for custom_request"),
        DebugAction::RemoveBreakpoint
        | DebugAction::SetInstructionBreakpoint
        | DebugAction::RemoveInstructionBreakpoint
        | DebugAction::SetDataBreakpoint
        | DebugAction::RemoveDataBreakpoint
        | DebugAction::DataBreakpointInfo
        | DebugAction::Continue
        | DebugAction::StepOver
        | DebugAction::StepIn
        | DebugAction::StepOut
        | DebugAction::Pause
        | DebugAction::StackTrace
        | DebugAction::Threads
        | DebugAction::Scopes
        | DebugAction::Disassemble
        | DebugAction::Modules
        | DebugAction::LoadedSources
        | DebugAction::Output
        | DebugAction::Terminate
        | DebugAction::Sessions => Ok(()),
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

/// Route one action to its backend group method.
async fn dispatch(backend: &dyn DapBackend, action: DebugAction, request: &DebugRequest) -> String {
    match action {
        DebugAction::Launch => backend.launch(request).await,
        DebugAction::Attach => backend.attach(request).await,
        DebugAction::SetBreakpoint
        | DebugAction::RemoveBreakpoint
        | DebugAction::SetInstructionBreakpoint
        | DebugAction::RemoveInstructionBreakpoint
        | DebugAction::SetDataBreakpoint
        | DebugAction::RemoveDataBreakpoint
        | DebugAction::DataBreakpointInfo => backend.breakpoints(request).await,
        DebugAction::Continue
        | DebugAction::StepOver
        | DebugAction::StepIn
        | DebugAction::StepOut
        | DebugAction::Pause => backend.stepping(request).await,
        DebugAction::Evaluate
        | DebugAction::StackTrace
        | DebugAction::Threads
        | DebugAction::Scopes
        | DebugAction::Variables
        | DebugAction::Disassemble
        | DebugAction::Output => backend.inspect(request).await,
        DebugAction::ReadMemory | DebugAction::WriteMemory => backend.memory(request).await,
        DebugAction::Modules | DebugAction::LoadedSources => backend.modules(request).await,
        DebugAction::CustomRequest | DebugAction::Terminate | DebugAction::Sessions => backend.control(request).await,
    }
}
