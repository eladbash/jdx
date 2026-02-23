use crossterm::event::{self, KeyCode};

use crate::keys::{map_key_event, Action};
use crate::modes::AppMode;
use crate::widgets::tree_view::build_tree;

use super::App;

impl App {
    pub(super) fn handle_tree_event(&mut self, key: event::KeyEvent) {
        let nodes = build_tree(&self.data, &self.tree.expanded);

        match key.code {
            KeyCode::Up => {
                self.tree.selected = self.tree.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.tree.selected + 1 < nodes.len() {
                    self.tree.selected += 1;
                }
            }
            KeyCode::Right | KeyCode::Enter => {
                if let Some(node) = nodes.get(self.tree.selected) {
                    if node.has_children {
                        self.tree.expanded.insert(node.path.clone());
                    }
                    // Update query to match the tree path
                    self.query.text = node.path.clone();
                    self.query.cursor = self.query.text.len();
                }
            }
            KeyCode::Left => {
                if let Some(node) = nodes.get(self.tree.selected) {
                    self.tree.expanded.remove(&node.path);
                }
            }
            KeyCode::Esc => {
                self.mode = AppMode::Query;
            }
            KeyCode::Char('q') => {
                self.mode = AppMode::Query;
            }
            _ => {
                let action = map_key_event(key);
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::ScrollDown => {
                        self.tree.scroll = self.tree.scroll.saturating_add(1);
                    }
                    Action::ScrollUp => {
                        self.tree.scroll = self.tree.scroll.saturating_sub(1);
                    }
                    Action::ToggleSplitView => {
                        self.split_view = false;
                        self.mode = AppMode::Query;
                    }
                    _ => {}
                }
            }
        }
    }
}
