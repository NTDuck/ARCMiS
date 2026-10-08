//! `pane` holds the render state of one agent pane.

/// Sliding transcript window. Older lines drop off the front.
#[derive(Debug, Clone)]
pub struct PaneState {
    pub title: String,
    lines: std::collections::VecDeque<String>,
    capacity: usize,
    pub status: String,
    pub current_activity: String,
    pub tokens: u64,
    pub cost: f64,
    pub agent_id: Option<String>,
}

impl PaneState {
    /// New pane with a fixed transcript capacity.
    pub fn new(title: impl Into<String>, capacity: usize) -> Self {
        Self {
            title: title.into(),
            lines: std::collections::VecDeque::with_capacity(capacity),
            capacity,
            status: "waiting".to_string(),
            current_activity: String::new(),
            tokens: 0,
            cost: 0.0,
            agent_id: None,
        }
    }

    /// Append one line. Trims to capacity.
    pub fn push_line(&mut self, line: impl Into<String>) {
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line.into());
    }

    /// Copy of visible lines for rendering.
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }
}
