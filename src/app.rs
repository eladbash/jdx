use std::collections::HashSet;
use std::sync::mpsc;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use serde_json::Value;

use crate::ai::ollama::OllamaProvider;
use crate::ai::openai::OpenAiProvider;
use crate::ai::service::{AiQuery, AiResponse, AiService};
use crate::config::AppConfig;
use crate::engine::json::{get_available_keys, pretty_print, traverse};
use crate::engine::query::{self, get_last_keyword};
use crate::engine::schema::{format_schema, infer_schema};
use crate::engine::suggestion::Suggester;
use crate::engine::transform::apply_transform;
use crate::history::History;
use crate::keys::{map_key_event, Action};
use crate::modes::AppMode;
use crate::widgets::ai_panel::AiPanelWidget;
use crate::widgets::candidate_popup::CandidatePopupWidget;
use crate::widgets::help_overlay::HelpOverlayWidget;
use crate::widgets::json_view::JsonViewWidget;
use crate::widgets::query_input::QueryInputWidget;
use crate::widgets::status_bar::StatusBarWidget;
use crate::widgets::tree_view::{build_tree, TreeViewWidget};

/// The main application state.
pub struct App {
    /// The root JSON data
    pub data: Value,
    /// Current query string
    pub query: String,
    /// Cursor position in the query
    pub cursor: usize,
    /// Current application mode
    pub mode: AppMode,
    /// Vertical scroll offset for JSON view
    pub scroll: u16,
    /// Whether to show key-only mode
    pub key_mode: bool,
    /// Candidate popup visible
    pub show_candidates: bool,
    /// Currently selected candidate index
    pub candidate_idx: usize,
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
    /// Tree view: expanded paths
    pub tree_expanded: HashSet<String>,
    /// Tree view: selected node index
    pub tree_selected: usize,
    /// Tree view: scroll offset
    pub tree_scroll: u16,
    /// Schema text (cached)
    pub schema_text: Option<String>,
    /// AI input
    pub ai_input: String,
    /// AI text answer
    pub ai_response: Option<String>,
    /// AI suggested query (optional)
    pub ai_suggested_query: Option<String>,
    /// AI loading state
    pub ai_loading: bool,
    /// AI error
    pub ai_error: Option<String>,
    /// AI input cursor position
    pub ai_cursor: usize,
    /// AI panel scroll offset
    pub ai_scroll: u16,
    /// Split view enabled
    pub split_view: bool,
    /// Monochrome mode
    pub monochrome: bool,
    /// Receiver for async AI results
    ai_result_rx: Option<mpsc::Receiver<Result<AiResponse, String>>>,
}

impl App {
    pub fn new(data: Value, query_output_mode: bool, monochrome: bool) -> Self {
        let config = crate::config::load_config();
        let history = History::load();

        Self {
            data,
            query: ".".into(),
            cursor: 1,
            mode: AppMode::Query,
            scroll: 0,
            key_mode: false,
            show_candidates: false,
            candidate_idx: 0,
            should_quit: false,
            confirmed: false,
            query_output_mode,
            suggester: Suggester::new(),
            config,
            history,
            status_message: None,
            tree_expanded: HashSet::new(),
            tree_selected: 0,
            tree_scroll: 0,
            schema_text: None,
            ai_input: String::new(),
            ai_response: None,
            ai_suggested_query: None,
            ai_loading: false,
            ai_error: None,
            ai_cursor: 0,
            ai_scroll: 0,
            split_view: false,
            monochrome,
            ai_result_rx: None,
        }
    }

    /// Poll for AI results from background thread (non-blocking).
    pub fn poll_ai_result(&mut self) {
        if let Some(rx) = &self.ai_result_rx {
            match rx.try_recv() {
                Ok(Ok(response)) => {
                    self.ai_loading = false;
                    self.ai_response = Some(response.answer);
                    self.ai_suggested_query = response.suggested_query;
                    // Stay in AI mode to show the answer — user can press Enter
                    // to apply suggested query or Esc to go back
                    self.ai_result_rx = None;
                }
                Ok(Err(err)) => {
                    self.ai_loading = false;
                    self.ai_error = Some(err);
                    self.ai_result_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still waiting — keep polling
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ai_loading = false;
                    self.ai_error = Some("AI request was interrupted".into());
                    self.ai_result_rx = None;
                }
            }
        }
    }

    /// Get the current traversal result.
    fn current_value(&self) -> Option<Value> {
        // Check for transform command
        if let Some(transform_start) = self.find_transform() {
            let path_part = &self.query[..transform_start].trim_end();
            let transform_part = &self.query[transform_start..];

            let segments = query::parse(path_part).unwrap_or_default();
            let result = traverse(&self.data, &segments);
            if let Some(val) = result.value {
                return apply_transform(&val, transform_part).ok();
            }
            return None;
        }

        let segments = query::parse(&self.query).unwrap_or_default();
        let result = traverse(&self.data, &segments);
        result.value
    }

    /// Check if query contains a transform command and return its start position.
    fn find_transform(&self) -> Option<usize> {
        // Look for " :" pattern that indicates a transform
        for (i, c) in self.query.chars().enumerate() {
            if c == ':' && i > 0 {
                let prev = self.query.as_bytes().get(i - 1).copied();
                if prev == Some(b' ') || prev == Some(b'|') {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Get the parent value (for suggestions).
    fn parent_value(&self) -> Value {
        let segments = query::parse(&self.query).unwrap_or_default();
        if segments.is_empty() {
            return self.data.clone();
        }
        // Navigate to the parent (all segments except the last complete one)
        let last_keyword = get_last_keyword(&self.query);
        if last_keyword.is_empty() {
            // We're right after a dot, so parent is the result of all segments
            let result = traverse(&self.data, &segments);
            return result.value.unwrap_or(self.data.clone());
        }
        // Drop the last (partial) segment
        let parent_segments = &segments[..segments.len().saturating_sub(1)];
        let result = traverse(&self.data, parent_segments);
        result.value.unwrap_or(self.data.clone())
    }

    /// Get stats string for the status bar.
    fn stats(&self) -> String {
        match self.current_value() {
            Some(Value::Object(map)) => format!("{} keys", map.len()),
            Some(Value::Array(arr)) => format!("{} items", arr.len()),
            Some(Value::String(s)) => format!("{} chars", s.len()),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Null) => "null".into(),
            None => "no match".into(),
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

    fn handle_query_event(&mut self, key: event::KeyEvent) {
        let action = map_key_event(key);
        match action {
            Action::InsertChar(c) => {
                self.query.insert(self.cursor, c);
                self.cursor += 1;
                self.show_candidates = false;
                self.scroll = 0;
                self.status_message = None;
            }
            Action::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.query.remove(self.cursor);
                    self.show_candidates = false;
                }
            }
            Action::Delete => {
                if self.cursor < self.query.len() {
                    self.query.remove(self.cursor);
                }
            }
            Action::CursorLeft => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            Action::CursorRight => {
                self.cursor = (self.cursor + 1).min(self.query.len());
            }
            Action::CursorHome => {
                self.cursor = 0;
            }
            Action::CursorEnd => {
                self.cursor = self.query.len();
            }
            Action::ClearQuery => {
                self.query = ".".into();
                self.cursor = 1;
                self.show_candidates = false;
            }
            Action::DeleteWordBackward => {
                // Delete from cursor back to previous `.` or `[`
                if self.cursor > 0 {
                    let mut target = self.cursor - 1;
                    while target > 0 {
                        let c = self.query.as_bytes()[target] as char;
                        if c == '.' || c == '[' {
                            break;
                        }
                        target -= 1;
                    }
                    self.query.drain(target..self.cursor);
                    self.cursor = target;
                }
            }
            Action::Tab => {
                self.handle_tab(false);
            }
            Action::BackTab => {
                self.handle_tab(true);
            }
            Action::Confirm => {
                if self.show_candidates {
                    self.apply_candidate();
                } else {
                    self.history.add_query(&self.query);
                    let _ = self.history.save();
                    self.confirmed = true;
                    self.should_quit = true;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::ScrollDown => {
                self.scroll = self.scroll.saturating_add(1);
            }
            Action::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Action::PageDown => {
                self.scroll = self.scroll.saturating_add(20);
            }
            Action::PageUp => {
                self.scroll = self.scroll.saturating_sub(20);
            }
            Action::ScrollToTop => {
                self.scroll = 0;
            }
            Action::ScrollToBottom => {
                self.scroll = u16::MAX / 2; // Will be clamped by renderer
            }
            Action::ToggleKeyMode => {
                self.key_mode = !self.key_mode;
            }
            Action::CopyValue => {
                if let Some(val) = self.current_value() {
                    match crate::clipboard::copy_value(&val) {
                        Ok(()) => self.status_message = Some("Copied value to clipboard".into()),
                        Err(e) => self.status_message = Some(format!("Copy failed: {e}")),
                    }
                }
            }
            Action::CopyQuery => match crate::clipboard::copy_query(&self.query) {
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
                self.schema_text = Some(format_schema(&schema, 0));
                self.mode = AppMode::Schema;
            }
            Action::ToggleHelp => {
                self.mode = AppMode::Help;
            }
            Action::ToggleSplitView => {
                self.split_view = !self.split_view;
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
                self.history.add_bookmark(&label, &self.query);
                let _ = self.history.save();
                self.status_message = Some(format!("Bookmarked as '{label}'"));
            }
            _ => {}
        }
    }

    fn handle_tab(&mut self, reverse: bool) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if candidates.is_empty() {
            self.show_candidates = false;
            return;
        }

        if !self.show_candidates {
            self.show_candidates = true;
            self.candidate_idx = 0;
        } else if reverse {
            self.candidate_idx = if self.candidate_idx == 0 {
                candidates.len() - 1
            } else {
                self.candidate_idx - 1
            };
        } else {
            self.candidate_idx = (self.candidate_idx + 1) % candidates.len();
        }
    }

    fn apply_candidate(&mut self) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if let Some(candidate) = candidates.get(self.candidate_idx) {
            // Remove the partial keyword from the query
            let keyword_len = last_keyword.len();
            let trim_from = self.cursor.saturating_sub(keyword_len);
            self.query.drain(trim_from..self.cursor);
            self.cursor = trim_from;

            // Insert the full candidate
            let text = &candidate.text;
            if text.starts_with('[') {
                // Array index — insert as-is
                self.query.insert_str(self.cursor, text);
                self.cursor += text.len();
            } else {
                self.query.insert_str(self.cursor, text);
                self.cursor += text.len();
            }
        }

        self.show_candidates = false;
        self.candidate_idx = 0;
    }

    fn handle_tree_event(&mut self, key: event::KeyEvent) {
        let nodes = build_tree(&self.data, &self.tree_expanded);

        match key.code {
            KeyCode::Up => {
                self.tree_selected = self.tree_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.tree_selected + 1 < nodes.len() {
                    self.tree_selected += 1;
                }
            }
            KeyCode::Right | KeyCode::Enter => {
                if let Some(node) = nodes.get(self.tree_selected) {
                    if node.has_children {
                        self.tree_expanded.insert(node.path.clone());
                    }
                    // Update query to match the tree path
                    self.query = node.path.clone();
                    self.cursor = self.query.len();
                }
            }
            KeyCode::Left => {
                if let Some(node) = nodes.get(self.tree_selected) {
                    self.tree_expanded.remove(&node.path);
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
                        self.tree_scroll = self.tree_scroll.saturating_add(1);
                    }
                    Action::ScrollUp => {
                        self.tree_scroll = self.tree_scroll.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_ai_event(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Query;
            }
            KeyCode::Backspace => {
                if self.ai_response.is_some() {
                    self.ai_response = None;
                    self.ai_suggested_query = None;
                    self.ai_error = None;
                }
                if self.ai_cursor > 0 {
                    self.ai_cursor -= 1;
                    self.ai_input.remove(self.ai_cursor);
                }
            }
            KeyCode::Delete => {
                if self.ai_cursor < self.ai_input.len() {
                    self.ai_input.remove(self.ai_cursor);
                }
            }
            KeyCode::Enter => {
                // If there's a suggested query from a previous response, apply it
                if let Some(ref query) = self.ai_suggested_query.clone() {
                    self.query = query.clone();
                    self.cursor = self.query.len();
                    self.scroll = 0;
                    self.mode = AppMode::Query;
                    return;
                }
                // Otherwise, dispatch a new AI query
                self.dispatch_ai_query();
            }
            KeyCode::Left if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ai_cursor = self.ai_cursor.saturating_sub(1);
            }
            KeyCode::Right if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ai_cursor = (self.ai_cursor + 1).min(self.ai_input.len());
            }
            KeyCode::Home => {
                self.ai_cursor = 0;
            }
            KeyCode::End => {
                self.ai_cursor = self.ai_input.len();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // If user starts typing after a response, clear old response
                if self.ai_response.is_some() {
                    self.ai_response = None;
                    self.ai_suggested_query = None;
                    self.ai_error = None;
                }
                self.ai_input.insert(self.ai_cursor, c);
                self.ai_cursor += 1;
            }
            _ => {
                let action = map_key_event(key);
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::CursorHome => self.ai_cursor = 0,
                    Action::CursorEnd => self.ai_cursor = self.ai_input.len(),
                    Action::CursorLeft => {
                        self.ai_cursor = self.ai_cursor.saturating_sub(1);
                    }
                    Action::CursorRight => {
                        self.ai_cursor = (self.ai_cursor + 1).min(self.ai_input.len());
                    }
                    Action::ClearQuery => {
                        self.ai_input.clear();
                        self.ai_cursor = 0;
                        self.ai_response = None;
                        self.ai_suggested_query = None;
                        self.ai_error = None;
                    }
                    Action::DeleteWordBackward => {
                        if self.ai_cursor > 0 {
                            let mut target = self.ai_cursor - 1;
                            while target > 0 && self.ai_input.as_bytes()[target] != b' ' {
                                target -= 1;
                            }
                            self.ai_input.drain(target..self.ai_cursor);
                            self.ai_cursor = target;
                        }
                    }
                    Action::ScrollDown => {
                        self.ai_scroll = self.ai_scroll.saturating_add(1);
                    }
                    Action::ScrollUp => {
                        self.ai_scroll = self.ai_scroll.saturating_sub(1);
                    }
                    Action::PageDown => {
                        self.ai_scroll = self.ai_scroll.saturating_add(10);
                    }
                    Action::PageUp => {
                        self.ai_scroll = self.ai_scroll.saturating_sub(10);
                    }
                    _ => {}
                }
            }
        }
    }

    fn dispatch_ai_query(&mut self) {
        let question = self.ai_input.trim().to_string();
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
                    self.ai_error = Some(format!(
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
                self.ai_error =
                    Some("AI disabled. Set ai.provider in ~/.config/jdx/config.toml".into());
                return;
            }
            other => {
                self.ai_error = Some(format!("Unknown AI provider: {other}"));
                return;
            }
        };

        // Set loading state
        self.ai_loading = true;
        self.ai_error = None;
        self.ai_response = None;
        self.ai_suggested_query = None;
        self.ai_scroll = 0;

        // Spawn background thread for the async AI call
        let (tx, rx) = mpsc::channel();
        self.ai_result_rx = Some(rx);

        let query = AiQuery {
            question,
            schema_summary,
            data_context,
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(service.query(&query));
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }

    fn handle_schema_event(&mut self, key: event::KeyEvent) {
        let action = map_key_event(key);
        match action {
            Action::Quit => {
                self.mode = AppMode::Query;
            }
            Action::ScrollDown => {
                self.scroll = self.scroll.saturating_add(1);
            }
            Action::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Render the application UI.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        if self.split_view {
            self.render_split(frame, area);
        } else {
            self.render_single(frame, area);
        }

        // Help overlay on top of everything
        if self.mode == AppMode::Help {
            let overlay = HelpOverlayWidget {
                mode: AppMode::Query,
            };
            frame.render_widget(overlay, area);
        }
    }

    fn render_single(&self, frame: &mut Frame, area: Rect) {
        // New layout: [results (fill)] [ai_panel (dynamic)] [query_input (1)] [status_bar (1)]
        let ai_height = (area.height / 4).clamp(6, 12);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),            // results panel
                Constraint::Length(ai_height), // AI panel (always visible)
                Constraint::Length(1),         // query input
                Constraint::Length(1),         // status bar
            ])
            .split(area);

        // Results panel — depends on mode
        match self.mode {
            AppMode::Schema => self.render_schema_view(frame, chunks[0]),
            _ => self.render_json_view(frame, chunks[0]),
        }

        // AI panel — always visible
        self.render_ai_panel(frame, chunks[1]);

        // Query input
        self.render_query_input(frame, chunks[2]);

        // Status bar
        self.render_status_bar(frame, chunks[3]);

        // Candidate popup (floating, on top of results area)
        if self.show_candidates && self.mode == AppMode::Query {
            self.render_candidates(frame, chunks[0]);
        }
    }

    fn render_split(&self, frame: &mut Frame, area: Rect) {
        // Layout: [tree|json (fill)] [ai_panel (dynamic)] [query_input (1)] [status_bar (1)]
        let ai_height = (area.height / 4).clamp(6, 12);

        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),            // results area (tree + json)
                Constraint::Length(ai_height), // AI panel (always visible)
                Constraint::Length(1),         // query input
                Constraint::Length(1),         // status bar
            ])
            .split(area);

        // Split the results area horizontally: tree | json
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(v_chunks[0]);

        // Tree view on the left
        let nodes = build_tree(&self.data, &self.tree_expanded);
        let tree = TreeViewWidget {
            nodes: &nodes,
            selected: self.tree_selected,
            scroll: self.tree_scroll,
        };
        frame.render_widget(tree, h_chunks[0]);

        // JSON view on the right
        self.render_json_view(frame, h_chunks[1]);

        // AI panel — always visible
        self.render_ai_panel(frame, v_chunks[1]);

        // Query input
        self.render_query_input(frame, v_chunks[2]);

        // Status bar
        self.render_status_bar(frame, v_chunks[3]);

        if self.show_candidates && self.mode == AppMode::Query {
            self.render_candidates(frame, v_chunks[0]);
        }
    }

    fn render_query_input(&self, frame: &mut Frame, area: Rect) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query);
        let completion = self.suggester.get_completion(&keys, &last_keyword);
        let completion_text = completion.map(|(c, _)| c);
        // Store it so we can reference it
        let completion_ref = completion_text.as_deref();

        let is_error = query::parse(&self.query).is_err() && self.query.len() > 1;

        let query_focused = self.mode == AppMode::Query;
        let widget = QueryInputWidget {
            query: &self.query,
            cursor: self.cursor,
            completion: completion_ref,
            error: is_error,
            focused: query_focused,
        };

        let cursor_x = widget.cursor_x(area);
        frame.render_widget(widget, area);

        // Set cursor position
        if self.mode == AppMode::Query {
            frame.set_cursor_position((cursor_x, area.y));
        }
    }

    fn render_json_view(&self, frame: &mut Frame, area: Rect) {
        let value = self.current_value();
        let widget = JsonViewWidget {
            value: value.as_ref(),
            scroll: self.scroll,
            key_mode: self.key_mode,
            title: if self.key_mode { "Keys" } else { "JSON" },
            monochrome: self.monochrome,
        };
        frame.render_widget(widget, area);
    }

    fn render_schema_view(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

        let text = self
            .schema_text
            .as_deref()
            .unwrap_or("Press 'S' in query mode to generate schema");

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Schema")
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(text.to_string())
            .block(block)
            .scroll((self.scroll, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn render_ai_panel(&self, frame: &mut Frame, area: Rect) {
        let focused = self.mode == AppMode::Ai;
        let widget = AiPanelWidget {
            input: &self.ai_input,
            cursor: self.ai_cursor,
            response: self.ai_response.as_deref(),
            suggested_query: self.ai_suggested_query.as_deref(),
            loading: self.ai_loading,
            error: self.ai_error.as_deref(),
            focused,
            scroll: self.ai_scroll,
        };

        if focused {
            let (cx, cy) = widget.cursor_position(area);
            frame.render_widget(widget, area);
            frame.set_cursor_position((cx, cy));
        } else {
            frame.render_widget(widget, area);
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let widget = StatusBarWidget {
            mode: self.mode.label(),
            path: &self.query,
            stats: &self.stats(),
            message: self.status_message.as_deref(),
        };
        frame.render_widget(widget, area);
    }

    fn render_candidates(&self, frame: &mut Frame, area: Rect) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if !candidates.is_empty() {
            let widget = CandidatePopupWidget {
                candidates: &candidates,
                selected: self.candidate_idx,
                max_visible: self.config.display.max_candidates,
            };
            frame.render_widget(widget, area);
        }
    }

    /// Get the final output string when the user confirms.
    pub fn get_output(&self) -> String {
        if self.query_output_mode {
            self.query.clone()
        } else {
            match self.current_value() {
                Some(val) => pretty_print(&val),
                None => String::new(),
            }
        }
    }
}
