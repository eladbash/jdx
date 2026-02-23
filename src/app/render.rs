use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::engine::json::get_available_keys;
use crate::engine::query::{self, get_last_keyword};
use crate::modes::AppMode;
use crate::widgets::ai_panel::AiPanelWidget;
use crate::widgets::candidate_popup::CandidatePopupWidget;
use crate::widgets::help_overlay::HelpOverlayWidget;
use crate::widgets::json_view::JsonViewWidget;
use crate::widgets::query_input::QueryInputWidget;
use crate::widgets::status_bar::StatusBarWidget;
use crate::widgets::tree_view::{build_tree, TreeViewWidget};

use super::App;

impl App {
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
        let ai_height = (area.height / 4).clamp(6, 12);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(ai_height),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        // Results panel — depends on mode
        match self.mode {
            AppMode::Schema => self.render_schema_view(frame, chunks[0]),
            _ => self.render_json_view(frame, chunks[0]),
        }

        self.render_ai_panel(frame, chunks[1]);
        self.render_query_input(frame, chunks[2]);
        self.render_status_bar(frame, chunks[3]);

        // Candidate popup (floating, on top of results area)
        if self.query.show_candidates && self.mode == AppMode::Query {
            self.render_candidates(frame, chunks[0]);
        }
    }

    fn render_split(&self, frame: &mut Frame, area: Rect) {
        let ai_height = (area.height / 4).clamp(6, 12);

        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(ai_height),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        // Split the results area horizontally: tree | json
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(v_chunks[0]);

        // Tree view on the left
        let nodes = build_tree(&self.data, &self.tree.expanded);
        let tree = TreeViewWidget {
            nodes: &nodes,
            selected: self.tree.selected,
            scroll: self.tree.scroll,
        };
        frame.render_widget(tree, h_chunks[0]);

        // JSON view on the right
        self.render_json_view(frame, h_chunks[1]);

        self.render_ai_panel(frame, v_chunks[1]);
        self.render_query_input(frame, v_chunks[2]);
        self.render_status_bar(frame, v_chunks[3]);

        if self.query.show_candidates && self.mode == AppMode::Query {
            self.render_candidates(frame, v_chunks[0]);
        }
    }

    fn render_query_input(&self, frame: &mut Frame, area: Rect) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query.text);
        let completion = self.suggester.get_completion(&keys, &last_keyword);
        let completion_text = completion.map(|(c, _)| c);
        let completion_ref = completion_text.as_deref();

        let query_parse_result = query::parse(&self.query.text);
        let is_error = query_parse_result.is_err() && self.query.text.len() > 1;

        let query_focused = self.mode == AppMode::Query;
        let widget = QueryInputWidget {
            query: &self.query.text,
            cursor: self.query.cursor,
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
        let value = self.current_value().unwrap_or(None);
        let widget = JsonViewWidget {
            value: value.as_ref(),
            scroll: self.query.scroll,
            key_mode: self.query.key_mode,
            title: if self.query.key_mode { "Keys" } else { "JSON" },
            monochrome: self.monochrome,
        };
        frame.render_widget(widget, area);
    }

    fn render_schema_view(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

        let text = self
            .schema
            .text
            .as_deref()
            .unwrap_or("Press 'S' in query mode to generate schema");

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Schema")
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(text.to_string())
            .block(block)
            .scroll((self.query.scroll, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    fn render_ai_panel(&self, frame: &mut Frame, area: Rect) {
        let focused = self.mode == AppMode::Ai;
        let widget = AiPanelWidget {
            input: &self.ai.input,
            cursor: self.ai.cursor,
            response: self.ai.response.as_deref(),
            suggested_query: self.ai.suggested_query.as_deref(),
            loading: self.ai.loading,
            error: self.ai.error.as_deref(),
            focused,
            scroll: self.ai.scroll,
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
            path: &self.query.text,
            stats: &self.stats(),
            message: self.status_message.as_deref(),
        };
        frame.render_widget(widget, area);
    }

    fn render_candidates(&self, frame: &mut Frame, area: Rect) {
        let parent = self.parent_value();
        let keys = get_available_keys(&parent);
        let last_keyword = get_last_keyword(&self.query.text);
        let candidates = self.suggester.get_candidates(&keys, &last_keyword);

        if !candidates.is_empty() {
            let widget = CandidatePopupWidget {
                candidates: &candidates,
                selected: self.query.candidate_idx,
                max_visible: self.config.display.max_candidates,
            };
            frame.render_widget(widget, area);
        }
    }
}
