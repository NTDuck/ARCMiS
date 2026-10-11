//! Integration tests for blackboard and mailbox bounds.

use mas_core::blackboard::Blackboard;
use mas_core::mailbox::Mailboxes;
use mas_core::mailbox::Message;

#[test]
fn blackboard_keeps_monotonic_numbers_past_trim() {
    let mut board = Blackboard::new(2);
    assert_eq!(board.publish("map", "analyst", "one"), 1);
    assert_eq!(board.publish("map", "analyst", "two"), 2);
    assert_eq!(board.publish("map", "architect", "three"), 3);
    let revisions = board.revisions("map");
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].number, 2);
    assert_eq!(revisions[1].number, 3);
    assert_eq!(board.latest("map").map(|entry| entry.body.as_str()), Some("three"));
}

#[test]
fn unbounded_blackboard_retains_everything() {
    let mut board = Blackboard::new(0);
    for index in 0..10 {
        board.publish("map", "analyst", &index.to_string());
    }
    assert_eq!(board.revisions("map").len(), 10);
}

#[test]
fn blackboard_topics_report_first_publish_order() {
    let mut board = Blackboard::new(4);
    board.publish("map", "analyst", "one");
    board.publish("brief", "architect", "two");
    assert_eq!(board.topics(), vec!["map", "brief"]);
}

#[test]
fn bounded_mailbox_refuses_when_full() {
    let mut mailboxes = Mailboxes::new(1);
    let message = Message {
        from: "lead".to_owned(),
        body: "hello".to_owned(),
    };
    assert!(mailboxes.deliver("translator", message.clone()));
    assert!(!mailboxes.deliver("translator", message));
    assert_eq!(mailboxes.drain("translator").len(), 1);
    assert!(mailboxes.drain("translator").is_empty());
}
