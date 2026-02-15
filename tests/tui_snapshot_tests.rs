use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use serde_json::json;

use jdx::widgets::json_view::JsonViewWidget;
use jdx::widgets::query_input::QueryInputWidget;
use jdx::widgets::status_bar::StatusBarWidget;

/// Snapshot tests using Ratatui's TestBackend.
/// These verify that widgets render correctly by checking buffer contents.

#[test]
fn test_query_input_renders_prompt() {
    let backend = TestBackend::new(60, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = QueryInputWidget {
                query: ".users[0].name",
                cursor: 14,
                completion: None,
                error: false,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let line = buffer_line_to_string(&buf, 0);
    assert!(line.contains("[Filter]>"), "should contain prompt");
    assert!(line.contains(".users[0].name"), "should contain query");
}

#[test]
fn test_query_input_with_completion() {
    let backend = TestBackend::new(60, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = QueryInputWidget {
                query: ".us",
                cursor: 3,
                completion: Some("ers"),
                error: false,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let line = buffer_line_to_string(&buf, 0);
    assert!(line.contains(".us"), "should contain partial query");
    assert!(line.contains("ers"), "should contain completion");
}

#[test]
fn test_json_view_renders_value() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    let data = json!({"name": "Alice", "age": 30});

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = JsonViewWidget {
                value: Some(&data),
                scroll: 0,
                key_mode: false,
                title: "JSON",
                monochrome: true,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("name"), "should contain key 'name'");
    assert!(all_text.contains("Alice"), "should contain value 'Alice'");
}

#[test]
fn test_json_view_key_mode() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    let data = json!({"alpha": 1, "beta": 2, "gamma": 3});

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = JsonViewWidget {
                value: Some(&data),
                scroll: 0,
                key_mode: true,
                title: "Keys",
                monochrome: true,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains(".alpha"), "should show .alpha key");
    assert!(all_text.contains(".beta"), "should show .beta key");
    assert!(all_text.contains(".gamma"), "should show .gamma key");
    // Should NOT contain the values
    assert!(!all_text.contains("1 "), "should not show value 1");
}

#[test]
fn test_json_view_no_data() {
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = JsonViewWidget {
                value: None,
                scroll: 0,
                key_mode: false,
                title: "JSON",
                monochrome: true,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("No data"), "should show 'No data'");
}

#[test]
fn test_status_bar_renders() {
    let backend = TestBackend::new(60, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = StatusBarWidget {
                mode: "QUERY",
                path: ".users[0]",
                stats: "3 keys",
                message: None,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let line = buffer_line_to_string(&buf, 0);
    assert!(line.contains("QUERY"), "should contain mode");
    assert!(line.contains(".users[0]"), "should contain path");
    assert!(line.contains("3 keys"), "should contain stats");
}

#[test]
fn test_status_bar_with_message() {
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = StatusBarWidget {
                mode: "QUERY",
                path: ".",
                stats: "4 keys",
                message: Some("Copied to clipboard"),
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let line = buffer_line_to_string(&buf, 0);
    assert!(
        line.contains("Copied to clipboard"),
        "should contain message"
    );
}

// Helper: extract a line from a buffer as a string
fn buffer_line_to_string(buf: &Buffer, y: u16) -> String {
    let area = buf.area();
    (area.x..area.x + area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
}

// Helper: extract all buffer contents as a string
fn buffer_to_string(buf: &Buffer) -> String {
    let area = buf.area();
    let mut result = String::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            result.push_str(buf[(x, y)].symbol());
        }
        result.push('\n');
    }
    result
}
