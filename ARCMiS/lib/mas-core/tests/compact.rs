//! Integration tests for compaction arithmetic and ordering.

use mas_core::compact::compact_frames;
use mas_core::compact::project_tokens;
use mas_core::compact::ContextFrame;

fn frame(text: &str) -> ContextFrame {
    ContextFrame {
        text: text.to_owned(),
        source: "test".to_owned(),
    }
}

#[test]
fn projection_is_bytes_over_four_with_floor() {
    assert_eq!(project_tokens(""), 1);
    assert_eq!(project_tokens("abcd"), 1);
    assert_eq!(project_tokens("abcdefgh"), 2);
}

#[test]
fn newest_frames_win_the_budget() {
    let frames = vec![frame("aaaaaaaa"), frame("bbbbbbbb"), frame("cccccccc")];
    let outcome = compact_frames(frames, 4);
    assert_eq!(outcome.kept.len(), 2);
    assert_eq!(outcome.kept[0].text, "bbbbbbbb");
    assert_eq!(outcome.archived.len(), 1);
    assert_eq!(outcome.archived[0].text, "aaaaaaaa");
}

#[test]
fn empty_frames_still_consume_one_token() {
    let frames = vec![frame(""), frame("")];
    let outcome = compact_frames(frames, 1);
    assert_eq!(outcome.kept.len(), 1);
    assert_eq!(outcome.archived.len(), 1);
}

#[test]
fn everything_fits_when_budget_is_large() {
    let frames = vec![frame("one"), frame("two"), frame("three")];
    let outcome = compact_frames(frames, 1_000);
    assert!(outcome.archived.is_empty());
    assert_eq!(outcome.kept.len(), 3);
}
