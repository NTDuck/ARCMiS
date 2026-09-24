//! Offload: when the projected context exceeds the model window, compact the
//! discarded history into PNG frames through snapcompact. The harness drives
//! the hook per delegation; this module carries the wiring.

use snapcompact::CompactOptions;
use snapcompact::SnapcompactHook;

/// Build the snapcompact hook from the run config values. `frame_budget`
/// tokens is the projection threshold at which compaction fires.
#[must_use]
pub fn hook(frame_budget: u32) -> SnapcompactHook {
    SnapcompactHook {
        threshold_tokens: frame_budget,
        options: CompactOptions {
            keep_recent_tokens: frame_budget / 2,
            ..CompactOptions::default()
        },
        font: std::sync::Arc::new(snapcompact::load_font()),
    }
}
