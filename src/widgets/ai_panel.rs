use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// AI panel widget for natural language querying.
/// Always visible in the layout; border highlights when focused.
pub struct AiPanelWidget<'a> {
    /// The NL input query
    pub input: &'a str,
    /// Cursor position in the AI input
    pub cursor: usize,
    /// The AI text answer
    pub response: Option<&'a str>,
    /// Optional suggested jdx query
    pub suggested_query: Option<&'a str>,
    /// Whether waiting for a response
    pub loading: bool,
    /// Error message if any
    pub error: Option<&'a str>,
    /// Whether this panel currently has input focus
    pub focused: bool,
    /// Scroll offset for the response text
    pub scroll: u16,
}

impl<'a> AiPanelWidget<'a> {
    /// Compute the screen position where the cursor should be drawn when focused.
    pub fn cursor_position(&self, area: Rect) -> (u16, u16) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let prompt_len = "Ask: ".len() as u16;
        let x = inner.x + prompt_len + self.cursor as u16;
        let y = inner.y; // Input is always the first line inside the block
        (x.min(inner.x + inner.width.saturating_sub(1)), y)
    }
}

impl<'a> Widget for AiPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Magenta
        } else {
            Color::DarkGray
        };

        let title = if self.focused {
            " AI Assistant "
        } else {
            " AI Assistant [/] "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();

        // Input line styling depends on focus
        let prompt_style = if self.focused {
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let input_style = if self.focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled("Ask: ", prompt_style),
            Span::styled(self.input, input_style),
        ]));

        // Loading indicator
        if self.loading {
            lines.push(Line::from(Span::styled(
                "⏳ Thinking...",
                Style::default().fg(Color::Yellow),
            )));
        }

        // Text answer
        if let Some(response) = self.response {
            lines.push(Line::from(""));
            for answer_line in response.lines() {
                lines.push(Line::from(Span::styled(
                    answer_line.to_string(),
                    Style::default().fg(Color::White),
                )));
            }
        }

        // Suggested query
        if let Some(query) = self.suggested_query {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "Query: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    query,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                "Press Enter to apply, Esc to go back",
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Error
        if let Some(error) = self.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {error}"),
                Style::default().fg(Color::Red),
            )));
        }

        // Help hint when empty
        if self.input.is_empty() && self.response.is_none() && !self.loading {
            let hint = if self.focused {
                "Type a question about your data..."
            } else {
                "Press / to ask AI about your data"
            };
            lines.push(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }
}
