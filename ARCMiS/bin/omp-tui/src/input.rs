//! `input` maps terminal keys to store and command actions.

use crate::store::Store;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Outcome of one terminal event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    None,
    Redraw,
    Submit {
        message: String,
        streaming: bool,
    },
    Abort,
    Quit,
}

/// Handle one crossterm event against the composer buffer.
pub fn handle_event(store: &mut Store, event: &Event) -> InputAction {
    match event {
        Event::Key(key) => handle_key(store, *key),
        Event::Resize(_, _) => InputAction::Redraw,
        _ => InputAction::None,
    }
}

fn handle_key(store: &mut Store, key: KeyEvent) -> InputAction {
    if key.kind == KeyEventKind::Release {
        return InputAction::None;
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => InputAction::Quit,
        KeyCode::Esc => InputAction::Abort,
        KeyCode::Enter => {
            let message = store.composer.trim().to_string();
            store.composer.clear();
            if message.is_empty() {
                return InputAction::None;
            }
            InputAction::Submit {
                streaming: store.busy,
                message,
            }
        },
        KeyCode::Backspace => {
            store.composer.pop();
            InputAction::Redraw
        },
        KeyCode::Char(character) => {
            store.composer.push(character);
            InputAction::Redraw
        },
        _ => InputAction::None,
    }
}
