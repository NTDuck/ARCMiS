//! Per-topic revision chain, bounded to `capacity` recent entries.

use crate::revision::Revision;

/// Per-topic revision chain, bounded to `capacity` recent entries.
#[derive(Debug)]
pub(crate) struct Topic {
    revisions: Vec<Revision>,
    /// Total revisions ever published (keeps numbering monotonic after
    /// trimming).
    published: usize,
    capacity: usize,
}

impl Topic {
    /// A topic holding at most `capacity` recent entries. Zero means
    /// unbounded.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            revisions: Vec::new(),
            published: 0,
            capacity,
        }
    }

    /// Append one revision. Trims past the capacity bound, keeping the
    /// most recent. Returns the assigned revision number.
    pub(crate) fn append(&mut self, author: &str, body: &str) -> usize {
        self.published += 1;
        let number = self.published;
        self.revisions.push(Revision {
            number,
            author: author.to_owned(),
            body: body.to_owned(),
        });
        if self.capacity > 0 && self.revisions.len() > self.capacity {
            let excess = self.revisions.len() - self.capacity;
            self.revisions.drain(..excess);
        }
        number
    }

    /// The head revision, if any.
    pub(crate) fn latest(&self) -> Option<&Revision> {
        self.revisions.last()
    }

    /// All retained revisions, oldest first.
    pub(crate) fn revisions(&self) -> &[Revision] {
        &self.revisions
    }
}
