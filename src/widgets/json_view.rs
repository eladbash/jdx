use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use serde_json::Value;

/// Syntax-highlighted, scrollable JSON viewport.
pub struct JsonViewWidget<'a> {
    /// The JSON value to display
    pub value: Option<&'a Value>,
    /// Vertical scroll offset
    pub scroll: u16,
    /// Whether to show only keys (key-only mode)
    pub key_mode: bool,
    /// Optional title for the border
    pub title: &'a str,
    /// Whether to use monochrome
    pub monochrome: bool,
}

impl<'a> Widget for JsonViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.title)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = match self.value {
            Some(value) => {
                if self.key_mode {
                    key_mode_lines(value, self.monochrome)
                } else {
                    highlight_json_lines(value, self.monochrome)
                }
            }
            None => vec![Line::from(Span::styled(
                "No data",
                Style::default().fg(Color::DarkGray),
            ))],
        };

        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .wrap(Wrap { trim: false });

        paragraph.render(inner, buf);
    }
}

/// Render a JSON value into syntax-highlighted lines.
pub fn highlight_json_lines(value: &Value, monochrome: bool) -> Vec<Line<'static>> {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    pretty
        .lines()
        .map(|line| highlight_json_line(line, monochrome))
        .collect()
}

/// Highlight a single line of pretty-printed JSON.
fn highlight_json_line(line: &str, monochrome: bool) -> Line<'static> {
    if monochrome {
        return Line::from(line.to_string());
    }

    let mut spans = Vec::new();
    let chars = line.chars().peekable();
    let mut current = String::new();
    let mut in_key = false;
    let mut in_string = false;
    let mut after_colon = false;

    for c in chars {
        match c {
            '"' => {
                if in_string || in_key {
                    current.push(c);
                    let style = if in_key {
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                    in_string = false;
                    in_key = false;
                } else {
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            current.clone(),
                            Style::default().fg(Color::White),
                        ));
                        current.clear();
                    }
                    current.push(c);
                    if after_colon {
                        in_string = true;
                        after_colon = false;
                    } else {
                        in_key = true;
                    }
                }
            }
            ':' if !in_string && !in_key => {
                current.push(c);
                spans.push(Span::styled(
                    current.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                current.clear();
                after_colon = true;
            }
            '{' | '}' | '[' | ']' | ',' if !in_string && !in_key => {
                if !current.is_empty() {
                    spans.push(Span::styled(
                        current.clone(),
                        Style::default().fg(Color::White),
                    ));
                    current.clear();
                }
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                after_colon = false;
            }
            _ if !in_string && !in_key => {
                current.push(c);
                // Check for keywords/values at word boundaries
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        let trimmed = current.trim();
        let style = if trimmed == "null" {
            Style::default().fg(Color::DarkGray)
        } else if trimmed == "true" || trimmed == "false" {
            Style::default().fg(Color::Yellow)
        } else if trimmed.parse::<f64>().is_ok() {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

/// Render only the keys available at the current level.
fn key_mode_lines(value: &Value, monochrome: bool) -> Vec<Line<'static>> {
    let key_style = if monochrome {
        Style::default()
    } else {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    };

    match value {
        Value::Object(map) => map
            .keys()
            .map(|k| Line::from(Span::styled(format!(".{k}"), key_style)))
            .collect(),
        Value::Array(arr) => (0..arr.len())
            .map(|i| Line::from(Span::styled(format!("[{i}]"), key_style)))
            .collect(),
        _ => vec![Line::from(Span::styled(
            "(primitive value — no keys)",
            Style::default().fg(Color::DarkGray),
        ))],
    }
}
