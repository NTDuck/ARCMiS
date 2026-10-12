//! Forward decoded SDK frames into the async stream.

use frames::map_sdk_event;
use frames::Event;
use omp_rpc::Event as SdkEvent;

/// Forward decoded SDK frames into the async stream until the child ends.
pub fn forward_frames(receiver: std::sync::mpsc::Receiver<SdkEvent>, sender: &tokio::sync::mpsc::Sender<Event>) {
    for frame in receiver.iter() {
        if sender.blocking_send(map_sdk_event(frame)).is_err() {
            return;
        }
    }
}
