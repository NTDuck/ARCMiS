//! `frame_sink` maps decoded omp frames onto task-graph state changes.
//!
//! One `Response` frame completes the oldest in-progress node. A
//! `PromptResult` frame with the wire status `completed` completes it the
//! same way. A result with `error` or `aborted` burns one repair attempt
//! on the node. A failing result logs its wire detail for triage. Every
//! other frame is bookkeeping and passes through unobserved.

use runtime::Event;
use taskgraph::TaskGraph;

use crate::absorb_prompt_result::absorb_prompt_result;
use crate::absorb_response::absorb_response;
use crate::context::ContextStore;

/// Apply one decoded frame to the graph and context.
pub fn absorb(graph: &mut TaskGraph, context: &mut ContextStore, event: Event) {
    match event {
        Event::Response(response) => absorb_response(graph, context, response),
        Event::PromptResult(result) => absorb_prompt_result(graph, context, result),
        Event::Ready(_) | Event::Other(_) => {},
        Event::SubagentLifecycle(_) | Event::SubagentProgress(_) | Event::SubagentEvent(_) => {},
        Event::ExtensionUiRequest(_) => {},
    }
}
