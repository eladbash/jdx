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

    assert_eq!(app.query.text, ".");
    assert_eq!(app.query.cursor, 1);
    assert!(!app.should_quit);

    // Type "name"
    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query.text, ".name");

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
    assert_eq!(app.query.text, ".name");

    // Ctrl+U to clear
    app.handle_event(ctrl_key('u'));
    assert_eq!(app.query.text, ".");
    assert_eq!(app.query.cursor, 1);
}

#[test]
fn test_app_backspace() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query.text, ".name");

    app.handle_event(key(KeyCode::Backspace));
    assert_eq!(app.query.text, ".nam");
    assert_eq!(app.query.cursor, 4);
}

#[test]
fn test_app_cursor_movement() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query.cursor, 5); // ".name" has 5 chars

    // Move left
    app.handle_event(key(KeyCode::Left));
    assert_eq!(app.query.cursor, 4);

    // Move to home
    app.handle_event(ctrl_key('a'));
    assert_eq!(app.query.cursor, 0);

    // Move to end
    app.handle_event(ctrl_key('e'));
    assert_eq!(app.query.cursor, 5);
}

#[test]
fn test_app_scroll() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert_eq!(app.query.scroll, 0);

    app.handle_event(ctrl_key('j')); // scroll down
    assert_eq!(app.query.scroll, 1);

    app.handle_event(ctrl_key('k')); // scroll up
    assert_eq!(app.query.scroll, 0);
}

#[test]
fn test_app_key_mode_toggle() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert!(!app.query.key_mode);

    app.handle_event(ctrl_key('l'));
    assert!(app.query.key_mode);

    app.handle_event(ctrl_key('l'));
    assert!(!app.query.key_mode);
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

// --- Mode switching tests ---

#[test]
fn test_mode_switch_to_ai_via_slash() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert_eq!(app.mode, AppMode::Query);

    // '/' switches to AI mode
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Ai);
}

#[test]
fn test_mode_switch_ai_back_to_query_via_esc() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // Enter AI mode
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Ai);

    // Esc goes back to query mode
    app.handle_event(key(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_mode_switch_to_schema() {
    let data = json!({"users": [{"name": "Alice"}]});
    let mut app = App::new(data, false, true);

    assert_eq!(app.mode, AppMode::Query);

    // 'S' switches to schema mode
    app.handle_event(key(KeyCode::Char('S')));
    assert_eq!(app.mode, AppMode::Schema);
    assert!(app.schema.text.is_some());
}

#[test]
fn test_mode_switch_schema_back_to_query_via_esc() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    app.handle_event(key(KeyCode::Char('S')));
    assert_eq!(app.mode, AppMode::Schema);

    app.handle_event(key(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_mode_switch_to_tree_via_split_view() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    assert_eq!(app.mode, AppMode::Query);
    assert!(!app.split_view);

    // Ctrl+S toggles split view and enters tree mode
    app.handle_event(ctrl_key('s'));
    assert!(app.split_view);
    assert_eq!(app.mode, AppMode::Tree);
}

#[test]
fn test_tree_mode_back_to_query_via_esc() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // Enter tree mode via split
    app.handle_event(ctrl_key('s'));
    assert_eq!(app.mode, AppMode::Tree);

    // Esc goes back
    app.handle_event(key(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_tree_mode_back_via_q() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // Enter tree mode via split
    app.handle_event(ctrl_key('s'));
    assert_eq!(app.mode, AppMode::Tree);

    // 'q' goes back
    app.handle_event(key(KeyCode::Char('q')));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_help_dismisses_to_query() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // '?' toggles help in any non-AI mode
    app.handle_event(key(KeyCode::Char('?')));
    assert_eq!(app.mode, AppMode::Help);

    // '?' again goes back to query (toggle)
    app.handle_event(key(KeyCode::Char('?')));
    assert_eq!(app.mode, AppMode::Query);
}

#[test]
fn test_ai_mode_typing() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // Enter AI mode
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Ai);

    // Type into AI input
    for c in "hello".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.ai.input, "hello");
    assert_eq!(app.ai.cursor, 5);
}

#[test]
fn test_ai_mode_backspace() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    // Enter AI mode
    app.handle_event(key(KeyCode::Char('/')));
    for c in "test".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.ai.input, "test");

    app.handle_event(key(KeyCode::Backspace));
    assert_eq!(app.ai.input, "tes");
}

#[test]
fn test_ctrl_c_quits_from_any_mode() {
    // From AI mode
    let data = json!({"a": 1});
    let mut app = App::new(data.clone(), false, true);
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Ai);
    app.handle_event(ctrl_key('c'));
    assert!(app.should_quit);

    // From Tree mode
    let mut app = App::new(data.clone(), false, true);
    app.handle_event(ctrl_key('s'));
    assert_eq!(app.mode, AppMode::Tree);
    app.handle_event(ctrl_key('c'));
    assert!(app.should_quit);

    // From Schema mode
    let mut app = App::new(data, false, true);
    app.handle_event(key(KeyCode::Char('S')));
    assert_eq!(app.mode, AppMode::Schema);
    app.handle_event(ctrl_key('c'));
    assert!(app.should_quit);
}

#[test]
fn test_tree_navigation() {
    let data = json!({"alpha": {"nested": 1}, "beta": 2});
    let mut app = App::new(data, false, true);

    // Enter tree mode
    app.handle_event(ctrl_key('s'));
    assert_eq!(app.mode, AppMode::Tree);
    assert_eq!(app.tree.selected, 0);

    // Move down
    app.handle_event(key(KeyCode::Down));
    assert_eq!(app.tree.selected, 1);

    // Move up
    app.handle_event(key(KeyCode::Up));
    assert_eq!(app.tree.selected, 0);
}

#[test]
fn test_delete_key() {
    let data = json!({"name": "Alice"});
    let mut app = App::new(data, false, true);

    for c in "name".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.query.text, ".name");

    // Move cursor left, then delete
    app.handle_event(key(KeyCode::Left));
    app.handle_event(key(KeyCode::Delete));
    assert_eq!(app.query.text, ".nam");
}

#[test]
fn test_page_up_down() {
    let data = json!({"a": 1});
    let mut app = App::new(data, false, true);

    app.handle_event(ctrl_key('n')); // page down
    assert_eq!(app.query.scroll, 20);

    app.handle_event(ctrl_key('p')); // page up
    assert_eq!(app.query.scroll, 0);
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
