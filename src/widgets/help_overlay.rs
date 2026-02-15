use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::modes::AppMode;

/// Help overlay showing keybindings for the current mode.
pub struct HelpOverlayWidget {
    pub mode: AppMode,
}

impl Widget for HelpOverlayWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Center the help popup
        let width = 60.min(area.width.saturating_sub(4));
        let height = 24.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        Clear.render(popup, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Help — {} Mode ", self.mode.label()))
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(popup);
        block.render(popup, buf);

        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::White);

        let bindings = get_bindings(self.mode);
        let lines: Vec<Line> = bindings
            .into_iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(format!("{key:>14}  "), key_style),
                    Span::styled(desc, desc_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }
}

fn get_bindings(mode: AppMode) -> Vec<(&'static str, &'static str)> {
    let mut bindings = vec![
        ("?", "Toggle this help"),
        ("Esc / Ctrl+C", "Quit"),
        ("Enter", "Confirm and output result"),
    ];

    match mode {
        AppMode::Query => {
            bindings.extend([
                ("Tab", "Complete / cycle candidates"),
                ("Shift+Tab", "Cycle candidates backward"),
                ("Ctrl+L", "Toggle key-only mode"),
                ("Ctrl+U", "Clear query"),
                ("Ctrl+W", "Delete word backward"),
                ("Ctrl+J/K", "Scroll down/up"),
                ("Ctrl+N/P", "Page down/up"),
                ("Ctrl+T/G", "Scroll to top/bottom"),
                ("Ctrl+Y", "Copy current value"),
                ("Ctrl+R", "Search history"),
                ("Ctrl+D", "Add bookmark"),
            ]);
        }
        AppMode::Tree => {
            bindings.extend([
                ("↑/↓", "Navigate tree"),
                ("→/Enter", "Expand node"),
                ("←", "Collapse node"),
                ("Ctrl+J/K", "Scroll"),
            ]);
        }
        AppMode::Ai => {
            bindings.extend([("Enter", "Send query to AI"), ("Esc", "Back to query mode")]);
        }
        AppMode::Schema => {
            bindings.extend([("Ctrl+J/K", "Scroll"), ("Esc", "Back to query mode")]);
        }
        AppMode::Help => {}
    }

    bindings
}
