//! Startup menu shown on a bare interactive `vibe` launch: start a new
//! session, resume one (opens the saved-session picker), or quit.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::server::Client;

pub const OPTIONS: [&str; 3] = ["New session", "Resume session…", "Quit"];

#[derive(Debug, Default, Clone)]
pub struct StartupMenu {
    pub open: bool,
    pub selected: usize,
}

pub fn open(app: &mut App) {
    app.startup_menu.open = true;
    app.startup_menu.selected = 0;
}

pub fn close(app: &mut App) {
    app.startup_menu.open = false;
}

/// A digit 1..=OPTIONS.len() (unmodified) picks that menu entry directly.
pub fn digit_option(key: &KeyEvent) -> Option<usize> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            (idx < OPTIONS.len()).then_some(idx)
        }
        _ => None,
    }
}

/// A printable key that is not a menu digit dismisses the menu and lands in
/// the composer.
pub fn wants_text(key: &KeyEvent) -> bool {
    if digit_option(key).is_some() {
        return false;
    }
    let plain = key.modifiers.is_empty() || key.modifiers == crossterm::event::KeyModifiers::SHIFT;
    plain && matches!(key.code, KeyCode::Char(_))
}

/// Pick one entry; returns true when the app should exit (Quit).
fn select(app: &mut App, client: &Arc<Client>, index: usize) -> bool {
    match index {
        // New session: the freshly attached session is already live.
        0 => close(app),
        // Resume: hand over to the saved-session picker.
        1 => {
            close(app);
            crate::resume_picker::open(app, client);
        }
        // Quit.
        _ => return true,
    }
    false
}

/// Handle one key; returns true when the app should exit (Quit).
/// Non-digit printable keys are NOT handled here: the event loop closes the
/// menu and forwards them to the composer (see `wants_text`).
pub fn handle_key(app: &mut App, client: &Arc<Client>, key: KeyEvent) -> bool {
    if !app.startup_menu.open {
        return false;
    }
    if let Some(index) = digit_option(&key) {
        return select(app, client, index);
    }
    match key.code {
        KeyCode::Up => {
            app.startup_menu.selected = app.startup_menu.selected.saturating_sub(1);
        }
        KeyCode::Down => {
            app.startup_menu.selected = (app.startup_menu.selected + 1).min(OPTIONS.len() - 1);
        }
        KeyCode::Enter => return select(app, client, app.startup_menu.selected),
        KeyCode::Esc => close(app),
        _ => {}
    }
    false
}
