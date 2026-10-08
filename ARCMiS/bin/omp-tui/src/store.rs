//! `store` is the shared application state the event loop and renderer read.

use crate::pane::PaneState;

/// Which pane receives a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSlot {
    Analyzer,
    Planning,
    Translator,
    Validator,
}

/// Full UI state. The renderer owns no state of its own.
pub struct Store {
    pub orchestrator: PaneState,
    pub workers: Vec<(PaneSlot, PaneState)>,
    pub composer: String,
    pub busy: bool,
    pub notice: String,
}

impl Store {
    /// Five panes: orchestrator plus the four paper workers.
    pub fn new(titles: [String; 5]) -> Self {
        let orchestrator = PaneState::new(titles[0].clone(), 800);
        let workers = vec![
            (PaneSlot::Analyzer, PaneState::new(titles[1].clone(), 400)),
            (PaneSlot::Planning, PaneState::new(titles[2].clone(), 400)),
            (PaneSlot::Translator, PaneState::new(titles[3].clone(), 400)),
            (PaneSlot::Validator, PaneState::new(titles[4].clone(), 400)),
        ];
        Self {
            orchestrator,
            workers,
            composer: String::new(),
            busy: false,
            notice: String::new(),
        }
    }

    /// Route by dotted subagent id. The top segment selects the worker.
    pub fn worker_slot_for(&self, agent_id: &str) -> Option<PaneSlot> {
        let top = agent_id.split('.').next().unwrap_or(agent_id);
        let lower = top.to_ascii_lowercase();
        if lower.contains("analyzer") {
            Some(PaneSlot::Analyzer)
        } else if lower.contains("planning") {
            Some(PaneSlot::Planning)
        } else if lower.contains("translator") {
            Some(PaneSlot::Translator)
        } else if lower.contains("validator") {
            Some(PaneSlot::Validator)
        } else {
            None
        }
    }

    /// Mutable pane for one slot.
    pub fn pane_mut(&mut self, slot: PaneSlot) -> &mut PaneState {
        self.workers
            .iter_mut()
            .find(|(candidate, _)| *candidate == slot)
            .map(|(_, pane)| pane)
            .expect("worker slot exists")
    }
}
