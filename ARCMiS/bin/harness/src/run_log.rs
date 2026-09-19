//! The run log and tracing install.
//!
//! The harness streams every agent run through one hook. Tool calls
//! surface through the rig hook with a cut-down args preview. The
//! structured results land in the tracing log and the result yaml.

use agents::ValidatorStepOutcome;
use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};

/// Tool-call args preview length. Longer args are cut and marked.
const ARG_PREVIEW_CHARS: usize = 200;

/// The tracing subscriber. One global install with an env filter.
pub struct Tracing;

impl Tracing {
    /// Install the subscriber. `RUST_LOG` selects the filter. `info` is the default.
    pub fn init() {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }
}

/// The run log. Tool calls surface through the rig hook. The agents'
/// structured results land in the tracing log and the result yaml.
#[derive(Clone, Default)]
pub struct RunLog;

impl AgentHook for RunLog {
    async fn on_completion_call(&self, ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        tracing::info!(turn = ctx.turn(), "model call");
        CompletionCallAction::continue_run()
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        tracing::info!(
            turn = ctx.turn(),
            tool = event.tool_name,
            args = %truncate_args(event.args),
            "tool call"
        );
        ToolCallAction::run()
    }
}

/// Cut `args` to [`ARG_PREVIEW_CHARS`] characters and append `...` when cut.
fn truncate_args(args: &str) -> String {
    if args.chars().count() <= ARG_PREVIEW_CHARS {
        return args.to_owned();
    }
    let cut: String = args.chars().take(ARG_PREVIEW_CHARS).collect();
    format!("{cut}...")
}

/// Render one toolchain step outcome as a detail line. Test counts appear
/// when the step reported them.
pub fn step_report_line(step: &ValidatorStepOutcome) -> String {
    match (step.tests_passed, step.tests_failed) {
        (Some(passed), Some(failed)) => format!("tests passed {passed}, failed {failed}"),
        _ => String::new(),
    }
}
