//! Startup menu overlay for a bare `vibe` launch.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{list_cursor, theme};
use crate::app::App;
use crate::startup_menu::OPTIONS;

pub fn draw(app: &mut App, f: &mut Frame, area: Rect) {
    let width = 44u16.min(area.width);
    let height = (OPTIONS.len() as u16 + 4).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let box_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, box_area);
    let lines: Vec<Line> = OPTIONS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let text = format!("  {}. {label}", i + 1);
            if i == app.startup_menu.selected {
                let bar = list_cursor::padded(&text, width.saturating_sub(4) as usize);
                Line::from(Span::styled(bar, list_cursor::style()))
            } else {
                Line::from(Span::styled(text, theme::text(theme::foreground())))
            }
        })
        .collect();
    let block = Block::default().borders(Borders::ALL).title(" Vibe ");
    f.render_widget(Paragraph::new(lines).block(block), box_area);
}
