use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Bottom status bar showing mode, path breadcrumb, and data stats.
pub struct StatusBarWidget<'a> {
    /// Current mode label (e.g., "QUERY", "TREE")
    pub mode: &'a str,
    /// Current path breadcrumb (e.g., ".users[0]")
    pub path: &'a str,
    /// Data stats (e.g., "3 keys" or "150 items")
    pub stats: &'a str,
    /// Optional message (e.g., "Copied to clipboard")
    pub message: Option<&'a str>,
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let bg = Color::DarkGray;

        // Fill background
        for x in area.x..area.x + area.width {
            buf[(x, area.y)].set_style(Style::default().bg(bg));
        }

        let mode_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let path_style = Style::default().fg(Color::White).bg(bg);
        let stats_style = Style::default().fg(Color::Yellow).bg(bg);
        let msg_style = Style::default().fg(Color::Green).bg(bg);

        let mut spans = vec![
            Span::styled(format!(" {} ", self.mode), mode_style),
            Span::styled(format!(" {} ", self.path), path_style),
        ];

        if let Some(msg) = self.message {
            spans.push(Span::styled(format!(" {msg} "), msg_style));
        }

        // Right-align stats
        let left_len: usize = spans.iter().map(|s| s.width()).sum();
        let stats_text = format!(" {} ", self.stats);
        let padding = (area.width as usize).saturating_sub(left_len + stats_text.len());
        if padding > 0 {
            spans.push(Span::styled(" ".repeat(padding), Style::default().bg(bg)));
        }
        spans.push(Span::styled(stats_text, stats_style));

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}
