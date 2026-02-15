use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use serde_json::Value;

/// A node in the collapsible tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub key: String,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
    pub value_preview: String,
    pub path: String,
}

/// Build a flat list of visible tree nodes from a JSON value.
pub fn build_tree(
    value: &Value,
    expanded_paths: &std::collections::HashSet<String>,
) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    build_tree_recursive(value, "", 0, expanded_paths, &mut nodes);
    nodes
}

fn build_tree_recursive(
    value: &Value,
    path: &str,
    depth: usize,
    expanded_paths: &std::collections::HashSet<String>,
    nodes: &mut Vec<TreeNode>,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let child_path = if path.is_empty() {
                    format!(".{key}")
                } else {
                    format!("{path}.{key}")
                };
                let has_children = matches!(val, Value::Object(m) if !m.is_empty())
                    || matches!(val, Value::Array(a) if !a.is_empty());
                let expanded = expanded_paths.contains(&child_path);
                let preview = value_preview(val);

                nodes.push(TreeNode {
                    key: key.clone(),
                    depth,
                    expanded,
                    has_children,
                    value_preview: preview,
                    path: child_path.clone(),
                });

                if expanded && has_children {
                    build_tree_recursive(val, &child_path, depth + 1, expanded_paths, nodes);
                }
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                let has_children = matches!(val, Value::Object(m) if !m.is_empty())
                    || matches!(val, Value::Array(a) if !a.is_empty());
                let expanded = expanded_paths.contains(&child_path);
                let preview = value_preview(val);

                nodes.push(TreeNode {
                    key: format!("[{i}]"),
                    depth,
                    expanded,
                    has_children,
                    value_preview: preview,
                    path: child_path.clone(),
                });

                if expanded && has_children {
                    build_tree_recursive(val, &child_path, depth + 1, expanded_paths, nodes);
                }
            }
        }
        _ => {}
    }
}

fn value_preview(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if s.len() > 30 {
                format!("\"{}...\"", &s[..27])
            } else {
                format!("\"{s}\"")
            }
        }
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(map) => format!("{{{} keys}}", map.len()),
    }
}

/// Tree view widget with collapsible nodes.
pub struct TreeViewWidget<'a> {
    pub nodes: &'a [TreeNode],
    pub selected: usize,
    pub scroll: u16,
}

impl<'a> Widget for TreeViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Tree")
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines: Vec<Line> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let indent = "  ".repeat(node.depth);
                let icon = if node.has_children {
                    if node.expanded {
                        "▼ "
                    } else {
                        "▶ "
                    }
                } else {
                    "  "
                };

                let is_selected = i == self.selected;
                let key_style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                };
                let preview_style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                Line::from(vec![
                    Span::raw(indent),
                    Span::raw(icon),
                    Span::styled(&node.key, key_style),
                    Span::styled(format!(": {}", node.value_preview), preview_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).scroll((self.scroll, 0));
        paragraph.render(inner, buf);
    }
}
