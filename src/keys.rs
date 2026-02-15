use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// High-level actions derived from keyboard input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Insert a character into the query
    InsertChar(char),
    /// Delete character before cursor
    Backspace,
    /// Delete character under cursor
    Delete,
    /// Move cursor left
    CursorLeft,
    /// Move cursor right
    CursorRight,
    /// Move cursor to start of query
    CursorHome,
    /// Move cursor to end of query
    CursorEnd,
    /// Delete entire query
    ClearQuery,
    /// Delete word backward
    DeleteWordBackward,
    /// Accept current suggestion / cycle candidates
    Tab,
    /// Cycle candidates backward
    BackTab,
    /// Confirm selection and exit
    Confirm,
    /// Cancel / exit without output
    Quit,
    /// Scroll JSON view down one line
    ScrollDown,
    /// Scroll JSON view up one line
    ScrollUp,
    /// Scroll JSON view down one page
    PageDown,
    /// Scroll JSON view up one page
    PageUp,
    /// Scroll to top
    ScrollToTop,
    /// Scroll to bottom
    ScrollToBottom,
    /// Toggle key-only mode
    ToggleKeyMode,
    /// Switch to tree view mode
    SwitchToTree,
    /// Switch to AI mode
    SwitchToAi,
    /// Switch to schema view
    SwitchToSchema,
    /// Toggle help overlay
    ToggleHelp,
    /// Copy current value to clipboard
    CopyValue,
    /// Copy current jq-compatible query to clipboard
    CopyQuery,
    /// Toggle split panel layout
    ToggleSplitView,
    /// Search history (Ctrl+R)
    SearchHistory,
    /// Add bookmark (Ctrl+B)
    AddBookmark,
    /// Go to bookmark (Ctrl+G)
    GoToBookmark,
    /// No-op
    None,
}

/// Map a crossterm KeyEvent to an Action based on the current app mode.
pub fn map_key_event(event: KeyEvent) -> Action {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);

    match event.code {
        // Navigation
        KeyCode::Left if !ctrl => Action::CursorLeft,
        KeyCode::Right if !ctrl => Action::CursorRight,
        KeyCode::Home => Action::CursorHome,
        KeyCode::End => Action::CursorEnd,

        // Ctrl shortcuts
        KeyCode::Char('a') if ctrl => Action::CursorHome,
        KeyCode::Char('e') if ctrl => Action::CursorEnd,
        KeyCode::Char('u') if ctrl => Action::ClearQuery,
        KeyCode::Char('w') if ctrl => Action::DeleteWordBackward,
        KeyCode::Char('f') if ctrl => Action::CursorRight,
        KeyCode::Char('b') if ctrl => Action::CursorLeft,
        KeyCode::Char('j') if ctrl => Action::ScrollDown,
        KeyCode::Char('k') if ctrl => Action::ScrollUp,
        KeyCode::Char('n') if ctrl => Action::PageDown,
        KeyCode::Char('p') if ctrl => Action::PageUp,
        KeyCode::Char('g') if ctrl => Action::ScrollToBottom,
        KeyCode::Char('t') if ctrl => Action::ScrollToTop,
        KeyCode::Char('l') if ctrl => Action::ToggleKeyMode,
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('y') if ctrl => Action::CopyValue,
        KeyCode::Char('r') if ctrl => Action::SearchHistory,
        KeyCode::Char('d') if ctrl => Action::AddBookmark,

        // Tab completion
        KeyCode::Tab => Action::Tab,
        KeyCode::BackTab => Action::BackTab,

        // Editing
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Delete => Action::Delete,
        KeyCode::Char(c) => Action::InsertChar(c),

        // Confirm / Cancel
        KeyCode::Enter => Action::Confirm,
        KeyCode::Esc => Action::Quit,

        _ => Action::None,
    }
}
