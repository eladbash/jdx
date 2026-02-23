use crossterm::event::{self, KeyCode};

use crate::keys::{map_key_event, Action};
use crate::modes::AppMode;

use super::App;

impl App {
    pub(super) fn handle_schema_event(&mut self, key: event::KeyEvent) {
        // Esc goes back to query mode; Ctrl+C quits the app
        if key.code == KeyCode::Esc {
            self.mode = AppMode::Query;
            return;
        }
        let action = map_key_event(key);
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::ScrollDown => {
                self.query.scroll = self.query.scroll.saturating_add(1);
            }
            Action::ScrollUp => {
                self.query.scroll = self.query.scroll.saturating_sub(1);
            }
            _ => {}
        }
    }
}
