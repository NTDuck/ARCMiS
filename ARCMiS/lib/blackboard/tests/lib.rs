//! Integration tests for the blackboard crate.

use mas_blackboard::Blackboard;

#[test]
fn publish_returns_monotonic_numbers() {
    let mut board = Blackboard::new(10);
    let first = board.publish("topic", "author", "body one");
    let second = board.publish("topic", "author", "body two");
    assert_eq!(first, 1);
    assert_eq!(second, 2);
}

#[test]
fn latest_returns_the_head_revision() {
    let mut board = Blackboard::new(10);
    board.publish("topic", "author", "body one");
    board.publish("topic", "author", "body two");
    let latest = board.latest("topic").expect("latest present");
    assert_eq!(latest.body, "body two");
    assert_eq!(latest.number, 2);
}

#[test]
fn bounded_blackboard_trims_old_revisions() {
    let mut board = Blackboard::new(2);
    board.publish("topic", "author", "body one");
    board.publish("topic", "author", "body two");
    board.publish("topic", "author", "body three");
    let revisions = board.revisions("topic");
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].body, "body two");
    assert_eq!(revisions[1].body, "body three");
}

#[test]
fn unbounded_blackboard_retains_everything() {
    let mut board = Blackboard::new(0);
    for index in 0..5 {
        board.publish("topic", "author", &format!("body {index}"));
    }
    assert_eq!(board.revisions("topic").len(), 5);
}

#[test]
fn blackboard_keeps_monotonic_numbers_past_trim() {
    let mut board = Blackboard::new(1);
    board.publish("topic", "author", "body one");
    board.publish("topic", "author", "body two");
    let latest = board.latest("topic").expect("latest present");
    assert_eq!(latest.number, 2);
}

#[test]
fn blackboard_topics_report_first_publish_order() {
    let mut board = Blackboard::new(10);
    board.publish("alpha", "author", "body");
    board.publish("beta", "author", "body");
    board.publish("gamma", "author", "body");
    assert_eq!(board.topics(), vec!["alpha", "beta", "gamma"]);
}
