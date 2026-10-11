//! `frame_sink` maps decoded omp frames onto task-graph state changes.
//!
//! One `Response` frame completes the oldest in-progress node. One
//! non-success `PromptResult` frame burns one repair attempt on it.
//! Every other frame is bookkeeping and passes through unobserved.

use crate::context::ContextStore;
use crate::taskgraph::TaskGraph;
use omprpc::frame::Event;

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
            let failed = result.status.as_deref().is_some_and(|status| status != "success");
            if failed {
                let in_progress = graph.ids_in_progress();
                if let Some(id) = in_progress.first().copied() {
                    let status = graph.record_failure(id);
                    tracing::warn!(id, ?status, "work item failed");
                }
            }
        },
        Event::Ready(_) | Event::Other(_) => {},
        Event::SubagentLifecycle(_) | Event::SubagentProgress(_) | Event::SubagentEvent(_) => {},
        Event::ExtensionUiRequest(_) => {},
    }
}
