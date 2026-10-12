//! Bounded direct-message queues for the MAS conductor (ADR 0031).
//!
//! One mailbox per agent address. The gateway polices tool use, not
//! messaging, so the mailbox accepts all senders. Queue capacity bounds
//! memory. The oldest message drops when the queue fills.

use std::collections::HashMap;
use std::collections::VecDeque;

/// One point-to-point message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Sender address.
    pub from: String,
    /// Body text.
    pub body: String,
}

/// Bounded FIFO queue for one agent address.
#[derive(Debug, Default)]
pub struct Mailbox {
    queue: VecDeque<Message>,
    capacity: usize,
}

impl Mailbox {
    /// A mailbox holding at most `capacity` messages. Zero means
    /// unbounded.
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity,
        }
    }

    /// Push one message. Returns `false` when the queue is full.
    pub fn send(&mut self, message: Message) -> bool {
        if self.capacity > 0 && self.queue.len() >= self.capacity {
            return false;
        }
        self.queue.push_back(message);
        true
    }

    /// Pop the oldest message.
    pub fn receive(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    /// Retained message count.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

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
