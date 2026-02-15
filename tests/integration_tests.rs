use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use serde_json::json;

use jdx::app::App;
use jdx::modes::AppMode;

/// Create a KeyEvent for testing.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn ctrl_key(c: char) -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// Test: basic app lifecycle — create, type a query, confirm, get output.
#[test]
fn test_app_basic_lifecycle() {
    let data = json!({"name": "Alice", "age": 30});
    let mut app = App::new(data, false, true);

    assert_eq!(app.query, ".");
    assert_eq!(app.cursor, 1);
    assert!(!app.should_quit);

    // Type "name"
    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query, ".name");

    // Confirm
    app.handle_event(key(KeyCode::Enter));
    assert!(app.should_quit);
    assert!(app.confirmed);

    let output = app.get_output();
    assert_eq!(output, "\"Alice\"");
}

#[test]
fn test_app_query_output_mode() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, true, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }

    app.handle_event(key(KeyCode::Enter));
    let output = app.get_output();
    assert_eq!(output, ".name");
}

#[test]
fn test_app_clear_query() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query, ".name");

    // Ctrl+U to clear
    app.handle_event(ctrl_key('u'));
    assert_eq!(app.query, ".");
    assert_eq!(app.cursor, 1);
}

#[test]
fn test_app_backspace() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query, ".name");

    app.handle_event(key(KeyCode::Backspace));
    assert_eq!(app.query, ".nam");
    assert_eq!(app.cursor, 4);
}

#[test]
fn test_app_cursor_movement() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.cursor, 5); // ".name" has 5 chars

    // Move left
    app.handle_event(key(KeyCode::Left));
    assert_eq!(app.cursor, 4);

    // Move to home
    app.handle_event(ctrl_key('a'));
    assert_eq!(app.cursor, 0);

    // Move to end
    app.handle_event(ctrl_key('e'));
    assert_eq!(app.cursor, 5);
}

#[test]
fn test_app_scroll() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert_eq!(app.scroll, 0);

    app.handle_event(ctrl_key('j')); // scroll down
    assert_eq!(app.scroll, 1);

    app.handle_event(ctrl_key('k')); // scroll up
    assert_eq!(app.scroll, 0);
}

#[test]
fn test_app_key_mode_toggle() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert!(!app.key_mode);

    app.handle_event(ctrl_key('l'));
    assert!(app.key_mode);

    app.handle_event(ctrl_key('l'));
    assert!(!app.key_mode);
}

#[test]
fn test_app_quit_with_esc() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    app.handle_event(key(KeyCode::Esc));
    assert!(app.should_quit);
    assert!(!app.confirmed);
}

#[test]
fn test_app_quit_with_ctrl_c() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    app.handle_event(ctrl_key('c'));
    assert!(app.should_quit);
    assert!(!app.confirmed);
}

#[test]
fn test_app_nested_navigation() {
    let data = json!({"store": {"books": [{"title": "1984"}]}});
    let mut app = App::new(data, false, true);

    // Type "store.books[0].title"
    for c in "store.books[0].title".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }

    app.handle_event(key(KeyCode::Enter));
    let output = app.get_output();
    assert_eq!(output, "\"1984\"");
}

#[test]
fn test_app_help_toggle() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert_eq!(app.mode, AppMode::Query);

    // Press '?' to open help
    app.handle_event(key(KeyCode::Char('?')));
    assert_eq!(app.mode, AppMode::Help);

    // Any key dismisses help
    app.handle_event(key(KeyCode::Char('a')));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_app_bookmark() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    for c in "a".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }

    // Ctrl+D to bookmark
    app.handle_event(ctrl_key('d'));
    assert!(app.status_message.is_some());
    assert!(app.status_message.as_ref().unwrap().contains("Bookmarked"));
}

#[test]
fn test_app_missing_path_output() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "missing".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }

    app.handle_event(key(KeyCode::Enter));
    let output = app.get_output();
    assert!(output.is_empty());
}
