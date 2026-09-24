//! Deterministic PNG bitmap archival for long agent contexts.
//!
//! Snapcompact replaces LLM summarization with a local pass: serialize the
//! discarded history to dense text, render the text to PNG frames with the
//! public-domain X11 8x13 bitmap font, and attach the frames as image blocks
//! on the next model call. Deterministic: same history in, same frames out.

pub mod compact;
pub mod hook;
pub mod render;
pub mod serialize;

pub use compact::compact;
pub use compact::message_tokens_public;
pub use compact::CompactOptions;
pub use compact::CompactResult;
pub use hook::SnapcompactHook;
pub use render::DEFAULT_FRAME_HEIGHT;
pub use render::DEFAULT_FRAME_WIDTH;
pub use render::render_frames;
pub use serialize::estimate_tokens;
pub use serialize::serialize_history;
pub use serialize::SerializeOptions;

use bdf_parser::BdfFont;

/// Parse the bundled 8x13 BDF font.
///
/// The font is public domain (X11 fixed, Markus Kuhn's ISO10646 build).
#[must_use]
pub fn load_font() -> BdfFont {
    const FONT_BYTES: &[u8] = include_bytes!("../assets/8x13.bdf");
    BdfFont::parse(FONT_BYTES).expect("bundled 8x13.bdf must parse")
}
