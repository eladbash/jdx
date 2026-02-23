use std::sync::mpsc;

use crossterm::event::{self, KeyCode, KeyModifiers};

use crate::ai::ollama::OllamaProvider;
use crate::ai::openai::OpenAiProvider;
use crate::ai::service::{AiQuery, AiService};
use crate::engine::schema::{format_schema, infer_schema};
use crate::keys::{map_key_event, Action};
use crate::modes::AppMode;

use super::App;

impl App {
    pub(super) fn handle_ai_event(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Query;
            }
            KeyCode::Backspace => {
                if self.ai.response.is_some() {
                    self.ai.response = None;
                    self.ai.suggested_query = None;
                    self.ai.error = None;
                }
                if self.ai.cursor > 0 {
                    self.ai.cursor -= 1;
                    self.ai.input.remove(self.ai.cursor);
                }
            }
            KeyCode::Delete => {
                if self.ai.cursor < self.ai.input.len() {
                    self.ai.input.remove(self.ai.cursor);
                }
            }
            KeyCode::Enter => {
                // If there's a suggested query from a previous response, apply it
                if let Some(ref suggested) = self.ai.suggested_query.clone() {
                    self.query.text = suggested.clone();
                    self.query.cursor = self.query.text.len();
                    self.query.scroll = 0;
                    self.mode = AppMode::Query;
                    return;
                }
                // Otherwise, dispatch a new AI query
                self.dispatch_ai_query();
            }
            KeyCode::Left if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ai.cursor = self.ai.cursor.saturating_sub(1);
            }
            KeyCode::Right if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ai.cursor = (self.ai.cursor + 1).min(self.ai.input.len());
            }
            KeyCode::Home => {
                self.ai.cursor = 0;
            }
            KeyCode::End => {
                self.ai.cursor = self.ai.input.len();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // If user starts typing after a response, clear old response
                if self.ai.response.is_some() {
                    self.ai.response = None;
                    self.ai.suggested_query = None;
                    self.ai.error = None;
                }
                self.ai.input.insert(self.ai.cursor, c);
                self.ai.cursor += 1;
            }
            _ => {
                let action = map_key_event(key);
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::CursorHome => self.ai.cursor = 0,
                    Action::CursorEnd => self.ai.cursor = self.ai.input.len(),
                    Action::CursorLeft => {
                        self.ai.cursor = self.ai.cursor.saturating_sub(1);
                    }
                    Action::CursorRight => {
                        self.ai.cursor = (self.ai.cursor + 1).min(self.ai.input.len());
                    }
                    Action::ClearQuery => {
                        self.ai.input.clear();
                        self.ai.cursor = 0;
                        self.ai.response = None;
                        self.ai.suggested_query = None;
                        self.ai.error = None;
                    }
                    Action::DeleteWordBackward => {
                        if self.ai.cursor > 0 {
                            let mut target = self.ai.cursor - 1;
                            while target > 0 && self.ai.input.as_bytes()[target] != b' ' {
                                target -= 1;
                            }
                            self.ai.input.drain(target..self.ai.cursor);
                            self.ai.cursor = target;
                        }
                    }
                    Action::ScrollDown => {
                        self.ai.scroll = self.ai.scroll.saturating_add(1);
                    }
                    Action::ScrollUp => {
                        self.ai.scroll = self.ai.scroll.saturating_sub(1);
                    }
                    Action::PageDown => {
                        self.ai.scroll = self.ai.scroll.saturating_add(10);
                    }
                    Action::PageUp => {
                        self.ai.scroll = self.ai.scroll.saturating_sub(10);
                    }
                    _ => {}
                }
            }
        }
    }

    pub(super) fn dispatch_ai_query(&mut self) {
        let question = self.ai.input.trim().to_string();
        if question.is_empty() {
            return;
        }

        // Build schema summary for context
        let schema = infer_schema(&self.data, self.config.display.schema_max_samples);
        let schema_summary = format_schema(&schema, 0);

        // Build data context (actual values, truncated if large)
        let data_context = crate::ai::prompts::truncate_data_for_prompt(&self.data, 4000);

        // Create provider from config
        let provider = &self.config.ai.provider;
        let model = self.config.ai.model.clone();
        let endpoint = if self.config.ai.endpoint.is_empty() {
            None
        } else {
            Some(self.config.ai.endpoint.clone())
        };
        let api_key = self.config.ai.api_key.clone();

        let service: AiService = match provider.as_str() {
            "ollama" => {
                let p = OllamaProvider::new(
                    if model.is_empty() {
                        "llama3.2".into()
                    } else {
                        model
                    },
                    endpoint,
                );
                AiService::with_provider(Box::new(p))
            }
            "openai" | "anthropic" => {
                if api_key.is_empty() {
                    self.ai.error = Some(format!(
                        "API key required for {provider}. Set ai.api_key in config.toml"
                    ));
                    return;
                }
                let p = OpenAiProvider::new(
                    api_key,
                    if model.is_empty() {
                        "gpt-4o-mini".into()
                    } else {
                        model
                    },
                    endpoint,
                );
                AiService::with_provider(Box::new(p))
            }
            "none" | "" => {
                self.ai.error =
                    Some("AI disabled. Set ai.provider in ~/.config/jdx/config.toml".into());
                return;
            }
            other => {
                self.ai.error = Some(format!("Unknown AI provider: {other}"));
                return;
            }
        };

        // Set loading state
        self.ai.loading = true;
        self.ai.error = None;
        self.ai.response = None;
        self.ai.suggested_query = None;
        self.ai.scroll = 0;

        // Spawn background thread for the async AI call
        let (tx, rx) = mpsc::channel();
        self.ai.result_rx = Some(rx);

        let query = AiQuery {
            question,
            schema_summary,
            data_context,
        };

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to start async runtime: {e}")));
                    return;
                }
            };
            let result = rt.block_on(service.query(&query));
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }
}
