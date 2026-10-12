//! Map SDK events onto the local frame vocabulary.

use omp_rpc::Event as SdkEvent;
use serde_json::Value;

use crate::frames::Event;
use crate::map_notification::map_notification;
use crate::response::ResponseFrame;

/// Map one SDK event onto the local vocabulary.
pub fn map_sdk_event(event: SdkEvent) -> Event {
    match event {
        SdkEvent::Notification(notification) => map_notification(notification),
        SdkEvent::UnmatchedResponse(response) => Event::Response(ResponseFrame {
            id: response.id,
            command: Some(response.command),
            success: response.success,
            data: response.data,
            error: response.error.map(Value::String),
        }),
        SdkEvent::Undecodable {
            raw,
            error,
        } => {
            // WHY a catch-all: omp adds frame types between releases and
            // the SDK refuses the unknown shape. Dropping them silently
            // hangs wait loops (verified in a prior integration), so
            // unknown frames stay reachable as the raw payload and log
            // at debug.
            tracing::debug!(error = %error, "kept undecodable frame");
            let extra = match raw {
                Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            Event::Other(crate::session_event::SessionEvent {
                extra,
            })
        },
    }
}
