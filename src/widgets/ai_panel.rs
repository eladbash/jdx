use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// AI panel widget for natural language querying.
pub struct AiPanelWidget<'a> {
    /// The NL input query
    pub input: &'a str,
    /// The AI response (path expression)
    pub response: Option<&'a str>,
    /// Optional explanation
    pub explanation: Option<&'a str>,
    /// Whether waiting for a response
    pub loading: bool,
    /// Error message if any
    pub error: Option<&'a str>,
}

impl<'a> Widget for AiPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("AI Query")
            .border_style(Style::default().fg(Color::Magenta));

        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();

        // Input line
        let input_style = Style::default().fg(Color::White);
        lines.push(Line::from(vec![
            Span::styled(
                "Ask: ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(self.input, input_style),
        ]));

        // Loading indicator
        if self.loading {
            lines.push(Line::from(Span::styled(
                "⏳ Thinking...",
                Style::default().fg(Color::Yellow),
            )));
        }

        // Response
        if let Some(response) = self.response {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "Path: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    response,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Explanation
        if let Some(explanation) = self.explanation {
            lines.push(Line::from(Span::styled(
                explanation,
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

        // Help hint
        if self.input.is_empty() && self.response.is_none() && !self.loading {
            lines.push(Line::from(Span::styled(
                "Type a question like \"find all users older than 30\"",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }
}
