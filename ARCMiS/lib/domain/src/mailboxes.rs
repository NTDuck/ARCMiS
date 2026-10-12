//! Address-to-mailbox switchboard for one run.

use std::collections::HashMap;

use crate::mailbox::Mailbox;
use crate::message::Message;

/// Address-to-mailbox switchboard for one run.
#[derive(Debug, Default)]
pub struct Mailboxes {
    boxes: HashMap<String, Mailbox>,
    capacity: usize,
}

impl Mailboxes {
    /// A switchboard creating mailboxes of `capacity` each.
    pub fn new(capacity: usize) -> Self {
        Self {
            boxes: HashMap::new(),
            capacity,
        }
    }

    /// Deliver one message. Creates the mailbox on first use. Returns
    /// `false` when the target mailbox is full.
    pub fn deliver(&mut self, to: &str, message: Message) -> bool {
        self.box_for(to).send(message)
    }

    /// Drain every message for one address, oldest first.
    pub fn drain(&mut self, to: &str) -> Vec<Message> {
        let mailbox = self.box_for(to);
        std::iter::from_fn(|| mailbox.receive()).collect()
    }

    fn box_for(&mut self, address: &str) -> &mut Mailbox {
        self.boxes.entry(address.to_owned()).or_insert_with(|| Mailbox::new(self.capacity))
    }
}
