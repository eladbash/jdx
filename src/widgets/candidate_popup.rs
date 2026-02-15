use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Widget},
};

use crate::engine::suggestion::Candidate;

/// Floating autocomplete popup showing ranked candidates.
pub struct CandidatePopupWidget<'a> {
    /// The candidates to display
    pub candidates: &'a [Candidate],
    /// Index of the currently selected candidate
    pub selected: usize,
    /// Maximum number of visible items
    pub max_visible: usize,
}

impl<'a> Widget for CandidatePopupWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.candidates.is_empty() || area.height < 3 {
            return;
        }

        let visible_count = self
            .candidates
            .len()
            .min(self.max_visible)
            .min(area.height as usize - 2);

        // Calculate popup dimensions
        let max_width = self
            .candidates
            .iter()
            .take(visible_count)
            .map(|c| c.text.len())
            .max()
            .unwrap_or(10)
            + 4; // padding

        let popup_width = (max_width as u16).min(area.width.saturating_sub(2));
        let popup_height = visible_count as u16 + 2; // +2 for borders

        // Position: below the query line, left-aligned
        let popup_area = Rect::new(area.x, area.y, popup_width, popup_height.min(area.height));

        // Clear the background
        Clear.render(popup_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title("Candidates");

        let items: Vec<ListItem> = self
            .candidates
            .iter()
            .take(visible_count)
            .enumerate()
            .map(|(i, candidate)| {
                let spans = highlight_candidate(candidate, i == self.selected);
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(block);
        list.render(popup_area, buf);
    }
}

/// Highlight a candidate, showing fuzzy match positions in bold.
fn highlight_candidate(candidate: &Candidate, selected: bool) -> Vec<Span<'static>> {
    let base_style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let highlight_style = if selected {
        base_style.add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    if candidate.match_indices.is_empty() {
        return vec![Span::styled(format!(" {} ", candidate.text), base_style)];
    }

    let mut spans = vec![Span::styled(" ", base_style)];
    for (i, ch) in candidate.text.chars().enumerate() {
        let style = if candidate.match_indices.contains(&i) {
            highlight_style
        } else {
            base_style
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    spans.push(Span::styled(" ", base_style));
    spans
}
