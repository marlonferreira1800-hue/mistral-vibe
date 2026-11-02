//! Startup menu keys: arrows move/clamp, digits 1..=3 pick, Enter picks,
//! Esc closes, Quit exits, other printable keys go to the composer.

use crossterm::event::{KeyCode, KeyModifiers};
use vibe_rs::app::App;
use vibe_rs::startup_menu;

use crate::support::{key, stub};

#[test]
fn arrows_move_and_clamp_at_the_ends() {
    let mut app = App::default();
    startup_menu::open(&mut app);
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Down)));
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Down)));
    assert_eq!(app.startup_menu.selected, 2, "clamp en bas");
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Up)));
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Up)));
    assert_eq!(app.startup_menu.selected, 0, "clamp en haut");
}

#[test]
fn digit_1_closes_as_new_session_and_digit_3_quits() {
    let mut app = App::default();
    startup_menu::open(&mut app);
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Char('1'))));
    assert!(!app.startup_menu.open, "1 = New session ferme");

    startup_menu::open(&mut app);
    assert!(startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Char('3'))), "3 = Quit exit");
}

#[test]
fn esc_and_enter_on_new_session_close_the_menu() {
    let mut app = App::default();
    startup_menu::open(&mut app);
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Esc)));
    assert!(!app.startup_menu.open);

    startup_menu::open(&mut app);
    assert!(!startup_menu::handle_key(&mut app, &stub(), key(KeyCode::Enter)));
    assert!(!app.startup_menu.open);
}

#[test]
fn printable_keys_go_to_composer_but_digits_select() {
    for code in [KeyCode::Char('1'), KeyCode::Char('2'), KeyCode::Char('3')] {
        assert!(!startup_menu::wants_text(&key(code)), "{code:?} = option menu");
        assert!(startup_menu::digit_option(&key(code)).is_some());
    }
    assert!(startup_menu::digit_option(&key(KeyCode::Char('4'))).is_none(), "4 = hors menu");
    for code in [KeyCode::Char('a'), KeyCode::Char('j'), KeyCode::Char('q'), KeyCode::Char('4'), KeyCode::Char(' ')] {
        let mut ev = key(code);
        assert!(startup_menu::wants_text(&ev), "{code:?}");
        ev.modifiers = KeyModifiers::SHIFT;
        assert!(startup_menu::wants_text(&ev), "{code:?} + SHIFT");
    }
    for code in [KeyCode::Up, KeyCode::Down, KeyCode::Enter, KeyCode::Esc] {
        assert!(!startup_menu::wants_text(&key(code)), "{code:?}");
    }
    let mut ctrl = key(KeyCode::Char('c'));
    ctrl.modifiers = KeyModifiers::CONTROL;
    assert!(!startup_menu::wants_text(&ctrl), "Ctrl+C reste global");
}
