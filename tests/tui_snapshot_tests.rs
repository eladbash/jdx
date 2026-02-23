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
                focused: true,
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
                focused: true,
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

// --- CandidatePopupWidget tests ---

use jdx::engine::suggestion::Candidate;
use jdx::modes::AppMode;
use jdx::widgets::ai_panel::AiPanelWidget;
use jdx::widgets::candidate_popup::CandidatePopupWidget;
use jdx::widgets::help_overlay::HelpOverlayWidget;
use jdx::widgets::tree_view::{build_tree, TreeNode, TreeViewWidget};
use std::collections::HashSet;

#[test]
fn test_candidate_popup_renders() {
    let candidates = vec![
        Candidate {
            text: "users".into(),
            score: 100,
            match_indices: vec![0, 1],
        },
        Candidate {
            text: "username".into(),
            score: 80,
            match_indices: vec![0, 1],
        },
        Candidate {
            text: "uuid".into(),
            score: 60,
            match_indices: vec![0],
        },
    ];

    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = CandidatePopupWidget {
                candidates: &candidates,
                selected: 0,
                max_visible: 5,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Candidates"), "should show title");
    assert!(all_text.contains("users"), "should show first candidate");
    assert!(
        all_text.contains("username"),
        "should show second candidate"
    );
}

#[test]
fn test_candidate_popup_empty() {
    let candidates: Vec<Candidate> = vec![];

    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = CandidatePopupWidget {
                candidates: &candidates,
                selected: 0,
                max_visible: 5,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    // Empty candidates should render nothing
    assert!(
        !all_text.contains("Candidates"),
        "should not show popup when empty"
    );
}

#[test]
fn test_candidate_popup_selection() {
    let candidates = vec![
        Candidate {
            text: "alpha".into(),
            score: 100,
            match_indices: vec![],
        },
        Candidate {
            text: "beta".into(),
            score: 80,
            match_indices: vec![],
        },
    ];

    let backend = TestBackend::new(30, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = CandidatePopupWidget {
                candidates: &candidates,
                selected: 1,
                max_visible: 5,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("alpha"), "should show first candidate");
    assert!(
        all_text.contains("beta"),
        "should show second (selected) candidate"
    );
}

// --- AiPanelWidget tests ---

#[test]
fn test_ai_panel_empty_unfocused() {
    let backend = TestBackend::new(50, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = AiPanelWidget {
                input: "",
                cursor: 0,
                response: None,
                suggested_query: None,
                loading: false,
                error: None,
                focused: false,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("AI Assistant"), "should show title");
    assert!(
        all_text.contains("Press / to ask"),
        "should show hint when unfocused"
    );
}

#[test]
fn test_ai_panel_focused_empty() {
    let backend = TestBackend::new(50, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = AiPanelWidget {
                input: "",
                cursor: 0,
                response: None,
                suggested_query: None,
                loading: false,
                error: None,
                focused: true,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Ask:"), "should show prompt");
    assert!(
        all_text.contains("Type a question"),
        "should show focused hint"
    );
}

#[test]
fn test_ai_panel_loading() {
    let backend = TestBackend::new(50, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = AiPanelWidget {
                input: "How many users?",
                cursor: 15,
                response: None,
                suggested_query: None,
                loading: true,
                error: None,
                focused: true,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("How many users?"), "should show input");
    assert!(
        all_text.contains("Thinking"),
        "should show loading indicator"
    );
}

#[test]
fn test_ai_panel_with_response() {
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = AiPanelWidget {
                input: "How many users?",
                cursor: 15,
                response: Some("There are 3 users."),
                suggested_query: Some(".users :count"),
                loading: false,
                error: None,
                focused: true,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(
        all_text.contains("There are 3 users"),
        "should show response"
    );
    assert!(
        all_text.contains(".users :count"),
        "should show suggested query"
    );
    assert!(
        all_text.contains("Press Enter to apply"),
        "should show apply hint"
    );
}

#[test]
fn test_ai_panel_with_error() {
    let backend = TestBackend::new(60, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = AiPanelWidget {
                input: "test",
                cursor: 4,
                response: None,
                suggested_query: None,
                loading: false,
                error: Some("No AI provider configured"),
                focused: true,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Error:"), "should show error prefix");
    assert!(
        all_text.contains("No AI provider"),
        "should show error message"
    );
}

// --- TreeViewWidget tests ---

#[test]
fn test_tree_view_renders() {
    let data = json!({"name": "Alice", "age": 30, "tags": ["admin", "user"]});
    let expanded = HashSet::new();
    let nodes = build_tree(&data, &expanded);

    let backend = TestBackend::new(50, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = TreeViewWidget {
                nodes: &nodes,
                selected: 0,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Tree"), "should show title");
    assert!(all_text.contains("name"), "should show 'name' key");
    assert!(all_text.contains("age"), "should show 'age' key");
    assert!(all_text.contains("tags"), "should show 'tags' key");
}

#[test]
fn test_tree_view_expanded() {
    let data = json!({"info": {"city": "NYC", "zip": "10001"}});
    let mut expanded = HashSet::new();
    expanded.insert(".info".to_string());
    let nodes = build_tree(&data, &expanded);

    let backend = TestBackend::new(50, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = TreeViewWidget {
                nodes: &nodes,
                selected: 0,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("info"), "should show parent key");
    assert!(all_text.contains("city"), "should show expanded child key");
    assert!(all_text.contains("zip"), "should show expanded child key");
}

#[test]
fn test_tree_view_empty() {
    let nodes: Vec<TreeNode> = vec![];

    let backend = TestBackend::new(50, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = TreeViewWidget {
                nodes: &nodes,
                selected: 0,
                scroll: 0,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(
        all_text.contains("Tree"),
        "should show title even when empty"
    );
}

// --- HelpOverlayWidget tests ---

#[test]
fn test_help_overlay_query_mode() {
    let backend = TestBackend::new(70, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = HelpOverlayWidget {
                mode: AppMode::Query,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Help"), "should show help title");
    assert!(all_text.contains("Ctrl+C"), "should show quit binding");
    assert!(all_text.contains("Tab"), "should show tab binding");
    assert!(all_text.contains("Filters"), "should show filter hint");
    assert!(
        all_text.contains("Transforms"),
        "should show transform hint"
    );
}

#[test]
fn test_help_overlay_tree_mode() {
    let backend = TestBackend::new(70, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let widget = HelpOverlayWidget {
                mode: AppMode::Tree,
            };
            frame.render_widget(widget, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let all_text = buffer_to_string(&buf);
    assert!(all_text.contains("Help"), "should show help title");
    assert!(
        all_text.contains("Navigate tree"),
        "should show tree bindings"
    );
}
