use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use jdx::keys::{map_key_event, Action};

/// Helper to create a plain key event (no modifiers).
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

/// Helper to create a Ctrl+<char> key event.
fn ctrl(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

// --- Basic navigation ---

#[test]
fn test_left_arrow() {
    assert_eq!(map_key_event(key(KeyCode::Left)), Action::CursorLeft);
}

#[test]
fn test_right_arrow() {
    assert_eq!(map_key_event(key(KeyCode::Right)), Action::CursorRight);
}

#[test]
fn test_home_key() {
    assert_eq!(map_key_event(key(KeyCode::Home)), Action::CursorHome);
}

#[test]
fn test_end_key() {
    assert_eq!(map_key_event(key(KeyCode::End)), Action::CursorEnd);
}

// --- Ctrl shortcuts ---

#[test]
fn test_ctrl_a_home() {
    assert_eq!(map_key_event(ctrl('a')), Action::CursorHome);
}

#[test]
fn test_ctrl_e_end() {
    assert_eq!(map_key_event(ctrl('e')), Action::CursorEnd);
}

#[test]
fn test_ctrl_u_clear() {
    assert_eq!(map_key_event(ctrl('u')), Action::ClearQuery);
}

#[test]
fn test_ctrl_w_delete_word() {
    assert_eq!(map_key_event(ctrl('w')), Action::DeleteWordBackward);
}

#[test]
fn test_ctrl_f_cursor_right() {
    assert_eq!(map_key_event(ctrl('f')), Action::CursorRight);
}

#[test]
fn test_ctrl_b_cursor_left() {
    assert_eq!(map_key_event(ctrl('b')), Action::CursorLeft);
}

#[test]
fn test_ctrl_j_scroll_down() {
    assert_eq!(map_key_event(ctrl('j')), Action::ScrollDown);
}

#[test]
fn test_ctrl_k_scroll_up() {
    assert_eq!(map_key_event(ctrl('k')), Action::ScrollUp);
}

#[test]
fn test_ctrl_n_page_down() {
    assert_eq!(map_key_event(ctrl('n')), Action::PageDown);
}

#[test]
fn test_ctrl_p_page_up() {
    assert_eq!(map_key_event(ctrl('p')), Action::PageUp);
}

#[test]
fn test_ctrl_g_scroll_bottom() {
    assert_eq!(map_key_event(ctrl('g')), Action::ScrollToBottom);
}

#[test]
fn test_ctrl_t_scroll_top() {
    assert_eq!(map_key_event(ctrl('t')), Action::ScrollToTop);
}

#[test]
fn test_ctrl_l_toggle_key_mode() {
    assert_eq!(map_key_event(ctrl('l')), Action::ToggleKeyMode);
}

#[test]
fn test_ctrl_c_quit() {
    assert_eq!(map_key_event(ctrl('c')), Action::Quit);
}

#[test]
fn test_ctrl_y_copy_value() {
    assert_eq!(map_key_event(ctrl('y')), Action::CopyValue);
}

#[test]
fn test_ctrl_r_search_history() {
    assert_eq!(map_key_event(ctrl('r')), Action::SearchHistory);
}

#[test]
fn test_ctrl_d_add_bookmark() {
    assert_eq!(map_key_event(ctrl('d')), Action::AddBookmark);
}

// --- Tab completion ---

#[test]
fn test_tab() {
    assert_eq!(map_key_event(key(KeyCode::Tab)), Action::Tab);
}

#[test]
fn test_backtab() {
    assert_eq!(map_key_event(key(KeyCode::BackTab)), Action::BackTab);
}

// --- Mode switches ---

#[test]
fn test_slash_switch_to_ai() {
    assert_eq!(map_key_event(key(KeyCode::Char('/'))), Action::SwitchToAi);
}

#[test]
fn test_uppercase_s_switch_to_schema() {
    assert_eq!(
        map_key_event(key(KeyCode::Char('S'))),
        Action::SwitchToSchema
    );
}

#[test]
fn test_ctrl_s_toggle_split_view() {
    assert_eq!(map_key_event(ctrl('s')), Action::ToggleSplitView);
}

#[test]
fn test_ctrl_backslash_toggle_split_view() {
    let event = KeyEvent {
        code: KeyCode::Char('\\'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert_eq!(map_key_event(event), Action::ToggleSplitView);
}

// --- Editing ---

#[test]
fn test_backspace() {
    assert_eq!(map_key_event(key(KeyCode::Backspace)), Action::Backspace);
}

#[test]
fn test_delete() {
    assert_eq!(map_key_event(key(KeyCode::Delete)), Action::Delete);
}

#[test]
fn test_regular_char_inserts() {
    assert_eq!(
        map_key_event(key(KeyCode::Char('a'))),
        Action::InsertChar('a')
    );
    assert_eq!(
        map_key_event(key(KeyCode::Char('z'))),
        Action::InsertChar('z')
    );
    assert_eq!(
        map_key_event(key(KeyCode::Char('0'))),
        Action::InsertChar('0')
    );
    assert_eq!(
        map_key_event(key(KeyCode::Char('.'))),
        Action::InsertChar('.')
    );
    assert_eq!(
        map_key_event(key(KeyCode::Char('['))),
        Action::InsertChar('[')
    );
}

// --- Confirm / Cancel ---

#[test]
fn test_enter_confirm() {
    assert_eq!(map_key_event(key(KeyCode::Enter)), Action::Confirm);
}

#[test]
fn test_esc_quit() {
    assert_eq!(map_key_event(key(KeyCode::Esc)), Action::Quit);
}

// --- No-op for unhandled keys ---

#[test]
fn test_f1_is_none() {
    assert_eq!(map_key_event(key(KeyCode::F(1))), Action::None);
}

#[test]
fn test_insert_is_none() {
    assert_eq!(map_key_event(key(KeyCode::Insert)), Action::None);
}
