//! Integration tests for the compact crate.

use mas_compact::compact_frames;
use mas_compact::project_tokens;
use mas_compact::CompactionOutcome;
use mas_compact::ContextFrame;

fn frame(text: &str) -> ContextFrame {
    ContextFrame {
        text: text.to_owned(),
        source: "test".to_owned(),
    }
}

#[test]
fn project_tokens_floors_at_one() {
    assert_eq!(project_tokens(""), 1);
    assert_eq!(project_tokens("ab"), 1);
    assert_eq!(project_tokens("abcd"), 1);
    assert_eq!(project_tokens("abcde"), 1);
    assert_eq!(project_tokens("abcdefgh"), 2);
}

#[test]
fn compact_keeps_newest_frames_within_budget() {
    let frames = vec![frame("aaaa"), frame("bbbb"), frame("cccc"), frame("dddd")];
    let outcome = compact_frames(frames, 2);
    assert_eq!(outcome.kept.len(), 2);
    assert_eq!(outcome.archived.len(), 2);
    assert_eq!(outcome.kept[0].text, "cccc");
    assert_eq!(outcome.kept[1].text, "dddd");
    assert_eq!(outcome.archived[0].text, "aaaa");
    assert_eq!(outcome.archived[1].text, "bbbb");
}

#[test]
fn compact_archives_everything_when_budget_is_zero() {
    let frames = vec![frame("aaaa"), frame("bbbb")];
    let outcome = compact_frames(frames, 0);
    assert!(outcome.kept.is_empty());
    assert_eq!(outcome.archived.len(), 2);
}

#[test]
fn compact_keeps_everything_when_budget_is_unlimited() {
    let frames = vec![frame("aaaa"), frame("bbbb"), frame("cccc")];
    let outcome = compact_frames(frames, 100);
    assert_eq!(outcome.kept.len(), 3);
    assert!(outcome.archived.is_empty());
}

#[test]
fn compact_handles_empty_input() {
    let outcome = compact_frames(Vec::new(), 10);
    assert_eq!(outcome, CompactionOutcome::default());
}
