//! Integration tests for the mailbox crate.

use mas_mailbox::Mailbox;
use mas_mailbox::Mailboxes;
use mas_mailbox::Message;

fn message(from: &str, body: &str) -> Message {
    Message {
        from: from.to_owned(),
        body: body.to_owned(),
    }
}

#[test]
fn mailbox_send_and_receive_round_trip() {
    let mut mailbox = Mailbox::new(10);
    assert!(mailbox.send(message("alice", "hello")));
    let received = mailbox.receive().expect("message present");
    assert_eq!(received.from, "alice");
    assert_eq!(received.body, "hello");
}

#[test]
fn bounded_mailbox_refuses_when_full() {
    let mut mailbox = Mailbox::new(1);
    assert!(mailbox.send(message("alice", "first")));
    assert!(!mailbox.send(message("alice", "second")));
    assert_eq!(mailbox.len(), 1);
}

#[test]
fn unbounded_mailbox_accepts_everything() {
    let mut mailbox = Mailbox::new(0);
    for index in 0..100 {
        assert!(mailbox.send(message("alice", &format!("msg {index}"))));
    }
    assert_eq!(mailbox.len(), 100);
}

#[test]
fn mailboxes_deliver_to_addresses() {
    let mut boxes = Mailboxes::new(10);
    assert!(boxes.deliver("alice", message("bob", "hi")));
    assert!(boxes.deliver("alice", message("carol", "hello")));
    let drained = boxes.drain("alice");
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].from, "bob");
    assert_eq!(drained[1].from, "carol");
}

#[test]
fn mailboxes_drain_empty_address_returns_empty() {
    let mut boxes = Mailboxes::new(10);
    let drained = boxes.drain("nobody");
    assert!(drained.is_empty());
}
