//! Apply one PromptResult frame to the graph and context.

use runtime::PromptResultFrame;
use taskgraph::TaskGraph;

use crate::context::ContextStore;

/// Apply one PromptResult frame to the graph and context.
pub fn absorb_prompt_result(graph: &mut TaskGraph, context: &mut ContextStore, result: PromptResultFrame) {
    // WHY `completed`, not `success`: the wire vocabulary is
    // `omp_rpc::PromptStatus` (completed | aborted | error). No
    // `success` value exists on the wire, so the old comparison
    // marked every finished turn a failure and burned the repair
    // budget.
    match result.status.as_deref() {
        Some("completed") => {
            let in_progress = graph.ids_in_progress();
            if let Some(id) = in_progress.first().copied() {
                graph.finish(id);
            }
        },
        Some(result_status @ ("error" | "aborted")) => {
            let in_progress = graph.ids_in_progress();
            if let Some(id) = in_progress.first().copied() {
                let item_state = graph.record_failure(id);
                let detail = result.error.as_ref();
                // WHY structured fields: a real failure must carry
                // its wire detail so the run log names the cause.
                tracing::warn!(
                    id,
                    item_state = ?item_state,
                    result_status,
                    message = detail.map_or("", |error| error.message.as_str()),
                    retryable = detail.is_some_and(|error| error.retryable),
                    provider = detail.and_then(|error| error.provider.as_deref()).unwrap_or(""),
                    model = detail.and_then(|error| error.model.as_deref()).unwrap_or(""),
                    http_status = detail.and_then(|error| error.http_status),
                    "work item failed"
                );
            }
        },
        _ => {},
    }
    let _ = context;
}
