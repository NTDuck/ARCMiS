//! Bounded FIFO queue for one agent address.

use std::collections::VecDeque;

use crate::message::Message;

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
