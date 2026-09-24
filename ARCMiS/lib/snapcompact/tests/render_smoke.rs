//! Render smoke test: serialize a large synthetic history, render frames,
//! assert the PNG output is non-empty and decodable.

use snapcompact::compact;
use snapcompact::CompactOptions;
use snapcompact::SerializeOptions;
use snapcompact::estimate_tokens;

fn synthetic_history(turns: usize) -> Vec<rig::message::Message> {
    use rig::completion::Message;
    let mut messages = Vec::new();
    for turn in 0..turns {
        messages.push(Message::User {
            content: vec![rig::message::UserContent::Text(rig::message::Text {
                text: format!("request {turn}: migrate module {turn} from C to Rust, keep the public API stable, and run the toolchain afterwards"),
                additional_params: None,
            })],
        });
        messages.push(Message::Assistant {
            id: None,
            content: vec![rig::message::AssistantContent::Text(rig::message::Text {
                text: format!("response {turn}: analyzed module {turn}, wrote 240 lines, build passed, tests green, coverage improved by 3.4 percent; noting edge case in parser for follow-up during the hardening phase"),
                additional_params: None,
            })],
        });
    }
    messages
}

#[test]
fn serializes_and_renders_frames() {
    let font = snapcompact::load_font();
    let history = synthetic_history(500);
    let opts = SerializeOptions::default();
    let transcript = snapcompact::serialize_history(&history, &opts);
    assert!(
        transcript.len() > 50_000,
        "transcript should be dense, got {} bytes",
        transcript.len()
    );
    let frames = snapcompact::render_frames(
        &transcript,
        &font,
        snapcompact::DEFAULT_FRAME_WIDTH,
        snapcompact::DEFAULT_FRAME_HEIGHT,
    );
    assert!(frames.len() >= 3, "expected several frames, got {}", frames.len());
    let pngs = snapcompact::render::encode_png(&frames);
    for (index, png) in pngs.iter().enumerate() {
        assert!(png.len() > 1000, "frame {index} suspiciously small: {}", png.len());
        let decoded = image::load_from_memory(png).expect("frame must decode as PNG");
        assert_eq!(decoded.width(), snapcompact::DEFAULT_FRAME_WIDTH);
        assert_eq!(decoded.height(), snapcompact::DEFAULT_FRAME_HEIGHT);
    }
    // Ink check: frame 0 must contain black pixels (text was drawn).
    let has_ink = frames[0]
        .pixels()
        .any(|p| p[0] == 0 && p[1] == 0 && p[2] == 0);
    assert!(has_ink, "frame 0 has no ink; rendering produced blank output");
}

#[test]
fn compact_keeps_recent_and_archives_rest() {
    let font = snapcompact::load_font();
    let history = synthetic_history(300);
    let before: u32 = history.iter().map(snapcompact::message_tokens_public).sum();
    let opts = CompactOptions {
        keep_recent_tokens: 2000,
        max_frames: 8,
        ..CompactOptions::default()
    };
    let result = compact(history, &font, &opts);
    assert!(!result.kept.is_empty(), "recent messages must be kept");
    assert!(!result.frames.is_empty(), "older history must be archived");
    assert!(result.frames.len() <= 8, "frame cap violated");
    assert!(result.tokens_after < result.tokens_before);
    assert!(
        result.summary_text.contains("archived"),
        "summary should name the archive: {}",
        result.summary_text
    );
    let _ = estimate_tokens("x"); // re-export reachability check.
}
