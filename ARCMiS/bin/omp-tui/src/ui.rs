//! `ui` renders the five panes and the composer.

use crate::store::{PaneSlot, Store};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

/// Draw one full frame.
pub fn draw(frame: &mut Frame, store: &Store) {
    let root = Layout::vertical([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)]).split(frame.area());
    draw_panes(frame, store, root[0]);
    draw_composer(frame, store, root[1]);
    draw_status(frame, store, root[2]);
}

fn draw_panes(frame: &mut Frame, store: &Store, area: Rect) {
    let columns = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    draw_orchestrator(frame, store, columns[0]);
    draw_worker_grid(frame, store, columns[1]);
}

fn draw_orchestrator(frame: &mut Frame, store: &Store, area: Rect) {
    let items = store.orchestrator.lines().into_iter().map(Line::from).map(ListItem::new).collect::<Vec<_>>();
    let block = List::new(items)
        .block(titled_block(&store.orchestrator.title, &store.orchestrator.status))
        .style(Style::default().fg(Color::White));
    frame.render_widget(block, area);
}

fn draw_worker_grid(frame: &mut Frame, store: &Store, area: Rect) {
    let rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    let columns = [
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]),
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]),
    ];
    let slots = [
        (columns[0][0], PaneSlot::Analyzer),
        (columns[0][1], PaneSlot::Planning),
        (columns[1][0], PaneSlot::Translator),
        (columns[1][1], PaneSlot::Validator),
    ];
    for (rect, slot) in slots {
        let Some((_, pane)) = store.workers.iter().find(|(candidate, _)| *candidate == slot) else {
            continue;
        };
        let items = pane.lines().into_iter().map(Line::from).map(ListItem::new).collect::<Vec<_>>();
        let block =
            List::new(items).block(titled_block(&pane.title, &pane.status)).style(Style::default().fg(Color::Gray));
        frame.render_widget(block, rect);
    }
}

fn titled_block<'a>(title: &'a str, status: &'a str) -> Block<'a> {
    let header =
        Span::styled(format!(" {title} [{status}] "), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    Block::default().borders(Borders::ALL).title(header)
}

fn draw_composer(frame: &mut Frame, store: &Store, area: Rect) {
    let hint = if store.busy {
        "steer"
    } else {
        "prompt"
    };
    let spans = vec![
        Span::styled(format!(" > [{hint}] "), Style::default().fg(Color::Yellow)),
        Span::raw(store.composer.clone()),
    ];
    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, store: &Store, area: Rect) {
    let (tokens, cost) =
        store.workers.iter().fold((0u64, 0.0f64), |(tokens, cost), (_, pane)| (tokens + pane.tokens, cost + pane.cost));
    let notice = if store.notice.is_empty() {
        String::new()
    } else {
        format!(" | {}", store.notice)
    };
    let line = Line::from(Span::styled(
        format!(" esc abort · ctrl+c quit · workers: {tokens} tok, {cost:.4} USD{notice}",),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(line), area);
}
