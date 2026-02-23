use crossterm::event;

use crate::engine::json::get_available_keys;
use crate::engine::query::get_last_keyword;
use crate::engine::schema::{format_schema, infer_schema};
use crate::keys::{map_key_event, Action};
use crate::modes::AppMode;

use super::App;

impl App {
    pub(super) fn handle_query_event(&mut self, key: event::KeyEvent) {
        let action = map_key_event(key);
        match action {
            Action::InsertChar(c) => {
                self.query.text.insert(self.query.cursor, c);
                self.query.cursor += 1;
                self.query.show_candidates = false;
                self.query.scroll = 0;
                self.status_message = None;
            }
            Action::Backspace => {
                if self.query.cursor > 0 {
                    self.query.cursor -= 1;
                    self.query.text.remove(self.query.cursor);
                    self.query.show_candidates = false;
                }
            }
            Action::Delete => {
                if self.query.cursor < self.query.text.len() {
                    self.query.text.remove(self.query.cursor);
                }
            }
            Action::CursorLeft => {
                self.query.cursor = self.query.cursor.saturating_sub(1);
            }
            Action::CursorRight => {
                self.query.cursor = (self.query.cursor + 1).min(self.query.text.len());
            }
            Action::CursorHome => {
                self.query.cursor = 0;
            }
            Action::CursorEnd => {
                self.query.cursor = self.query.text.len();
            }
            Action::ClearQuery => {
                self.query.text = ".".into();
                self.query.cursor = 1;
                self.query.show_candidates = false;
            }
            Action::DeleteWordBackward => {
                // Delete from cursor back to previous `.` or `[`
                if self.query.cursor > 0 {
                    let mut target = self.query.cursor - 1;
                    while target > 0 {
                        let c = self.query.text.as_bytes()[target] as char;
                        if c == '.' || c == '[' {
                            break;
                        }
                        target -= 1;
                    }
                    self.query.text.drain(target..self.query.cursor);
                    self.query.cursor = target;
                }
            }
            Action::Tab => {
                self.handle_tab(false);
            }
            Action::BackTab => {
                self.handle_tab(true);
            }
            Action::Confirm => {
                if self.query.show_candidates {
                    self.apply_candidate();
                } else {
                    self.history.add_query(&self.query.text);
                    let _ = self.history.save();
                    self.confirmed = true;
                    self.should_quit = true;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::ScrollDown => {
                self.query.scroll = self.query.scroll.saturating_add(1);
            }
            Action::ScrollUp => {
                self.query.scroll = self.query.scroll.saturating_sub(1);
            }
            Action::PageDown => {
                self.query.scroll = self.query.scroll.saturating_add(20);
            }
            Action::PageUp => {
                self.query.scroll = self.query.scroll.saturating_sub(20);
            }
            Action::ScrollToTop => {
                self.query.scroll = 0;
            }
            Action::ScrollToBottom => {
                self.query.scroll = u16::MAX / 2; // Will be clamped by renderer
            }
            Action::ToggleKeyMode => {
                self.query.key_mode = !self.query.key_mode;
            }
            Action::CopyValue => match self.current_value() {
                Ok(Some(val)) => match crate::clipboard::copy_value(&val) {
                    Ok(()) => self.status_message = Some("Copied value to clipboard".into()),
                    Err(e) => self.status_message = Some(format!("Copy failed: {e}")),
                },
                Ok(None) => {
                    self.status_message = Some("No value to copy".into());
                }
                Err(e) => {
                    self.status_message = Some(e);
                }
            },
            Action::CopyQuery => match crate::clipboard::copy_query(&self.query.text) {
                Ok(()) => self.status_message = Some("Copied query to clipboard".into()),
                Err(e) => self.status_message = Some(format!("Copy failed: {e}")),
            },
            Action::SwitchToTree => {
                self.mode = AppMode::Tree;
            }
            Action::SwitchToAi => {
                self.mode = AppMode::Ai;
            }
            Action::SwitchToSchema => {
                let schema = infer_schema(&self.data, self.config.display.schema_max_samples);
                self.schema.text = Some(format_schema(&schema, 0));
                self.mode = AppMode::Schema;
            }
            Action::ToggleHelp => {
                self.mode = AppMode::Help;
            }
            Action::ToggleSplitView => {
                if self.split_view {
                    self.split_view = false;
                } else {
                    self.split_view = true;
                    self.mode = AppMode::Tree;
                }
            }
            Action::SearchHistory => {
                // Simple: show last N history items as candidates
                let history_items = self.history.search("");
                if !history_items.is_empty() {
                    self.status_message = Some(format!("History: {} entries", history_items.len()));
                }
            }
            Action::AddBookmark => {
                let label = format!("bookmark_{}", self.history.get_bookmarks().len());
                self.history.add_bookmark(&label, &self.query.text);
                let _ = self.history.save();
                self.status_message = Some(format!("Bookmarked as '{label}'"));
            }
            _ => {}
        }
    }

    pub(super) fn handle_tab(&mut self, reverse: bool) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query.text);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if candidates.is_empty() {
            self.query.show_candidates = false;
            return;
        }

        if !self.query.show_candidates {
            self.query.show_candidates = true;
            self.query.candidate_idx = 0;
        } else if reverse {
            self.query.candidate_idx = if self.query.candidate_idx == 0 {
                candidates.len() - 1
            } else {
                self.query.candidate_idx - 1
            };
        } else {
            self.query.candidate_idx = (self.query.candidate_idx + 1) % candidates.len();
        }
    }

    pub(super) fn apply_candidate(&mut self) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query.text);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if let Some(candidate) = candidates.get(self.query.candidate_idx) {
            // Remove the partial keyword from the query
            let keyword_len = last_keyword.len();
            let trim_from = self.query.cursor.saturating_sub(keyword_len);
            self.query.text.drain(trim_from..self.query.cursor);
            self.query.cursor = trim_from;

            // Insert the full candidate
            let text = &candidate.text;
            self.query.text.insert_str(self.query.cursor, text);
            self.query.cursor += text.len();
        }

        self.query.show_candidates = false;
        self.query.candidate_idx = 0;
    }
}
