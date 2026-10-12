//! `frame_sink` maps decoded omp frames onto task-graph state changes.
//!
//! One `Response` frame completes the oldest in-progress node. A
//! `PromptResult` frame with the wire status `completed` completes it the
//! same way. A result with `error` or `aborted` burns one repair attempt
//! on the node. Every other frame is bookkeeping and passes through
//! unobserved.

use crate::context::ContextStore;
use crate::frames::Event;
use crate::taskgraph::TaskGraph;

/// Apply one decoded frame to the graph and context.
pub fn absorb(graph: &mut TaskGraph, context: &mut ContextStore, event: Event) {
    match event {
        Event::Response(response) => {
            let in_progress = graph.ids_in_progress();
            if let Some(id) = in_progress.first().copied() {
                graph.finish(id);
                let text = response.data.as_ref().and_then(|data| data.as_str()).unwrap_or_default();
                context.publish("response", text);
            }
        },
        Event::PromptResult(result) => {
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
                Some("error") | Some("aborted") => {
                    let in_progress = graph.ids_in_progress();
                    if let Some(id) = in_progress.first().copied() {
                        let status = graph.record_failure(id);
                        tracing::warn!(id, ?status, "work item failed");
                    }
                },
                _ => {},
            }
        },
        Event::Ready(_) | Event::Other(_) => {},
        Event::SubagentLifecycle(_) | Event::SubagentProgress(_) | Event::SubagentEvent(_) => {},
        Event::ExtensionUiRequest(_) => {},
    }
}
