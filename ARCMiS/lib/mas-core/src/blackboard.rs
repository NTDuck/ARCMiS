//! `blackboard` holds per-topic state as bounded revision chains (ADR 0031).
//!
//! Each topic keeps every published revision. `latest` reads the head.
//! The revision bound trims from the front. Memory stays flat, and the
//! trim keeps history order.

/// One published revision on one topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// Monotonic revision number within the topic.
    pub number: usize,
    /// Team or role that published.
    pub author: String,
    /// Body text.
    pub body: String,
}

/// Per-topic revision chain, bounded to `capacity` recent entries.
#[derive(Debug)]
struct Topic {
    revisions: Vec<Revision>,
    /// Total revisions ever published (keeps numbering monotonic after
    /// trimming).
    published: usize,
    capacity: usize,
}

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
        let entry = self.topic_entry(topic);
        entry.published += 1;
        let number = entry.published;
        entry.revisions.push(Revision {
            number,
            author: author.to_owned(),
            body: body.to_owned(),
        });
        if entry.capacity > 0 && entry.revisions.len() > entry.capacity {
            let excess = entry.revisions.len() - entry.capacity;
            entry.revisions.drain(..excess);
        }
        number
    }

    /// The head revision, if any.
    pub fn latest(&self, topic: &str) -> Option<&Revision> {
        self.topic(topic).and_then(|entry| entry.revisions.last())
    }

    /// All retained revisions for one topic, oldest first.
    pub fn revisions(&self, topic: &str) -> &[Revision] {
        self.topic(topic).map_or(&[], |entry| &entry.revisions)
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
        self.topics.push((
            name.to_owned(),
            Topic {
                revisions: Vec::new(),
                published: 0,
                capacity: self.capacity,
            },
        ));
        let last = self.topics.len() - 1;
        &mut self.topics[last].1
    }
}
