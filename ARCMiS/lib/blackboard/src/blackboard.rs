//! The blackboard itself.

use crate::revision::Revision;
use crate::topic::Topic;

/// The blackboard itself.
#[derive(Debug, Default)]
pub struct Blackboard {
    topics: Vec<(String, Topic)>,
    capacity: usize,
}

impl Blackboard {
    /// A blackboard holding `capacity` recent revisions per topic.
    /// Zero means unbounded.
    pub fn new(capacity: usize) -> Self {
        Self {
            topics: Vec::new(),
            capacity,
        }
    }

    /// Publish one revision. Trims past the capacity bound, keeping the
    /// most recent.
    pub fn publish(&mut self, topic: &str, author: &str, body: &str) -> usize {
        self.topic_entry(topic).append(author, body)
    }

    /// The head revision, if any.
    pub fn latest(&self, topic: &str) -> Option<&Revision> {
        self.topic(topic).and_then(|entry| entry.latest())
    }

    /// All retained revisions for one topic, oldest first.
    pub fn revisions(&self, topic: &str) -> &[Revision] {
        self.topic(topic).map_or(&[], |entry| entry.revisions())
    }

    /// Every topic name, in first-publish order.
    pub fn topics(&self) -> Vec<&str> {
        self.topics.iter().map(|(name, _)| name.as_str()).collect()
    }

    fn topic(&self, name: &str) -> Option<&Topic> {
        self.topics.iter().find(|(key, _)| key == name).map(|(_, entry)| entry)
    }

    fn topic_entry(&mut self, name: &str) -> &mut Topic {
        if let Some(index) = self.topics.iter().position(|(key, _)| key == name) {
            return &mut self.topics[index].1;
        }
        self.topics.push((name.to_owned(), Topic::new(self.capacity)));
        let last = self.topics.len() - 1;
        &mut self.topics[last].1
    }
}
