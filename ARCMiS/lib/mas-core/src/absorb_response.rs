//! Apply one Response frame to the graph and context.

use runtime::ResponseFrame;
use taskgraph::TaskGraph;

use crate::context::ContextStore;

/// Apply one Response frame to the graph and context.
pub fn absorb_response(graph: &mut TaskGraph, context: &mut ContextStore, response: ResponseFrame) {
    let in_progress = graph.ids_in_progress();
    if let Some(id) = in_progress.first().copied() {
        graph.finish(id);
        let text = response.data.as_ref().and_then(|data| data.as_str()).unwrap_or_default();
        context.publish("response", text);
    }
}
