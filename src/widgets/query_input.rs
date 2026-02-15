use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// The query input line at the top of the screen.
/// Shows: `[Filter]> .foo.bar` with optional ghost completion text.
pub struct QueryInputWidget<'a> {
    /// The current query string
    pub query: &'a str,
    /// Cursor position (character index)
    pub cursor: usize,
    /// Ghost completion text (shown dimmed after the query)
    pub completion: Option<&'a str>,
    /// Whether there's a validation error
    pub error: bool,
}

impl<'a> Widget for QueryInputWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let prompt = "[Filter]> ";
        let prompt_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let query_style = if self.error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };

        let completion_style = Style::default().fg(Color::DarkGray);

        let mut spans = vec![
            Span::styled(prompt, prompt_style),
            Span::styled(self.query, query_style),
        ];

        if let Some(completion) = self.completion {
            spans.push(Span::styled(completion, completion_style));
        }

        let line = Line::from(spans);
        let x = area.x;
        let y = area.y;
        buf.set_line(x, y, &line, area.width);
    }
}

impl<'a> QueryInputWidget<'a> {
    /// Get the screen X position where the cursor should be placed.
    pub fn cursor_x(&self, area: Rect) -> u16 {
        let prompt_len = "[Filter]> ".len();
        area.x + (prompt_len + self.cursor) as u16
    }
}
