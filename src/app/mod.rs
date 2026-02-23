mod ai_handler;
mod query_handler;
mod render;
mod schema_handler;
pub mod state;
mod tree_handler;

use std::sync::mpsc;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use serde_json::Value;

use crate::config::AppConfig;
use crate::engine::json::{pretty_print, traverse};
use crate::engine::query::{self, get_last_keyword};
use crate::engine::suggestion::Suggester;
// get_available_keys used in render.rs and query_handler.rs
use crate::engine::transform::apply_transform;
use crate::history::History;
use crate::modes::AppMode;

pub use state::{AiState, QueryState, SchemaState, TreeState};

/// The main application state.
pub struct App {
    /// The root JSON data
    pub data: Value,
    /// Query-mode state
    pub query: QueryState,
    /// Tree-view state
    pub tree: TreeState,
    /// AI-mode state
    pub ai: AiState,
    /// Schema-view state
    pub schema: SchemaState,
    /// Current application mode
    pub mode: AppMode,
    /// Whether the app should exit
    pub should_quit: bool,
    /// Whether the user confirmed (Enter) or cancelled (Esc/Ctrl-C)
    pub confirmed: bool,
    /// Query output mode (-q flag)
    pub query_output_mode: bool,
    /// Suggestion engine
    pub suggester: Suggester,
    /// Configuration
    pub config: AppConfig,
    /// Query history
    pub history: History,
    /// Status message (temporary)
    pub status_message: Option<String>,
    /// Split view enabled
    pub split_view: bool,
    /// Monochrome mode
    pub monochrome: bool,
    /// Receiver for streaming NDJSON lines from stdin
    stdin_rx: Option<mpsc::Receiver<Value>>,
    /// Whether stdin is still streaming data
    pub streaming: bool,
}

impl App {
    pub fn new(data: Value, query_output_mode: bool, monochrome: bool) -> Self {
        let (config, config_warning) = crate::config::load_config();
        let history = History::load();

        Self {
            data,
            query: QueryState::default(),
            tree: TreeState::default(),
            ai: AiState::default(),
            schema: SchemaState::default(),
            mode: AppMode::Query,
            should_quit: false,
            confirmed: false,
            query_output_mode,
            suggester: Suggester::new(),
            config,
            history,
            status_message: config_warning,
            split_view: false,
            monochrome,
            stdin_rx: None,
            streaming: false,
        }
    }

    /// Poll for AI results from background thread (non-blocking).
    pub fn poll_ai_result(&mut self) {
        if let Some(rx) = &self.ai.result_rx {
            match rx.try_recv() {
                Ok(Ok(response)) => {
                    self.ai.loading = false;
                    self.ai.response = Some(response.answer);
                    self.ai.suggested_query = response.suggested_query;
                    self.ai.result_rx = None;
                }
                Ok(Err(err)) => {
                    self.ai.loading = false;
                    self.ai.error = Some(err);
                    self.ai.result_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still waiting — keep polling
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ai.loading = false;
                    self.ai.error = Some("AI request was interrupted".into());
                    self.ai.result_rx = None;
                }
            }
        }
    }

    /// Set the receiver for streaming NDJSON lines.
    pub fn set_stdin_rx(&mut self, rx: mpsc::Receiver<Value>) {
        self.stdin_rx = Some(rx);
        self.streaming = true;
    }

    /// Poll for new NDJSON lines from the background stdin reader (non-blocking).
    pub fn poll_stdin(&mut self) {
        if let Some(rx) = &self.stdin_rx {
            let mut got_data = false;
            loop {
                match rx.try_recv() {
                    Ok(val) => {
                        if let Value::Array(ref mut arr) = self.data {
                            arr.push(val);
                        }
                        got_data = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.streaming = false;
                        self.stdin_rx = None;
                        break;
                    }
                }
            }
            if got_data {
                // Invalidate cached schema so it gets regenerated with new data
                self.schema.text = None;
            }
        }
    }

    /// Get the current traversal result.
    /// Returns `Ok(Some(value))` on success, `Ok(None)` if the path doesn't match,
    /// or `Err(message)` if the query has a syntax error.
    pub(crate) fn current_value(&self) -> Result<Option<Value>, String> {
        // Check for transform command
        if let Some(transform_start) = self.find_transform() {
            let path_part = &self.query.text[..transform_start].trim_end();
            let transform_part = &self.query.text[transform_start..];

            let segments = query::parse(path_part).map_err(|e| format!("Query error: {e}"))?;
            let result = traverse(&self.data, &segments);
            if let Some(val) = result.value {
                return apply_transform(&val, transform_part)
                    .map(Some)
                    .map_err(|e| format!("Transform error: {e}"));
            }
            return Ok(None);
        }

        let segments = query::parse(&self.query.text).map_err(|e| format!("Query error: {e}"))?;
        let result = traverse(&self.data, &segments);
        Ok(result.value)
    }

    /// Check if query contains a transform command and return its start position.
    fn find_transform(&self) -> Option<usize> {
        for (i, c) in self.query.text.chars().enumerate() {
            if c == ':' && i > 0 {
                let prev = self.query.text.as_bytes().get(i - 1).copied();
                if prev == Some(b' ') || prev == Some(b'|') {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Get the parent value (for suggestions).
    pub(crate) fn parent_value(&self) -> Value {
        let segments = query::parse(&self.query.text).unwrap_or_default();
        if segments.is_empty() {
            return self.data.clone();
        }
        let last_keyword = get_last_keyword(&self.query.text);
        if last_keyword.is_empty() {
            let result = traverse(&self.data, &segments);
            return result.value.unwrap_or(self.data.clone());
        }
        let parent_segments = &segments[..segments.len().saturating_sub(1)];
        let result = traverse(&self.data, parent_segments);
        result.value.unwrap_or(self.data.clone())
    }

    /// Get stats string for the status bar.
    pub(crate) fn stats(&self) -> String {
        let base = match self.current_value() {
            Ok(Some(Value::Object(map))) => format!("{} keys", map.len()),
            Ok(Some(Value::Array(arr))) => format!("{} items", arr.len()),
            Ok(Some(Value::String(s))) => format!("{} chars", s.len()),
            Ok(Some(Value::Number(n))) => n.to_string(),
            Ok(Some(Value::Bool(b))) => b.to_string(),
            Ok(Some(Value::Null)) => "null".into(),
            Ok(None) => "no match".into(),
            Err(_) => "syntax error".into(),
        };
        if self.streaming {
            format!("{base} (streaming...)")
        } else {
            base
        }
    }

    /// Handle a terminal event.
    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            // Special handling for '?' to toggle help in any mode
            if key.code == KeyCode::Char('?')
                && !key.modifiers.contains(KeyModifiers::CONTROL)
                && self.mode != AppMode::Ai
            {
                if self.mode == AppMode::Help {
                    self.mode = AppMode::Query;
                } else {
                    self.mode = AppMode::Help;
                }
                return;
            }

            match self.mode {
                AppMode::Query => self.handle_query_event(key),
                AppMode::Tree => self.handle_tree_event(key),
                AppMode::Ai => self.handle_ai_event(key),
                AppMode::Schema => self.handle_schema_event(key),
                AppMode::Help => {
                    // Any key dismisses help
                    self.mode = AppMode::Query;
                }
            }
        }
    }

    /// Get the final output string when the user confirms.
    pub fn get_output(&self) -> String {
        if self.query_output_mode {
            self.query.text.clone()
        } else {
            match self.current_value() {
                Ok(Some(val)) => pretty_print(&val),
                Ok(None) | Err(_) => String::new(),
            }
        }
    }
}
