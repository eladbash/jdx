use anyhow::Result;
use arboard::Clipboard;

/// Copy a string to the system clipboard.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Copy a JSON value (pretty-printed) to the clipboard.
pub fn copy_value(value: &serde_json::Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    copy_to_clipboard(&text)
}

/// Copy a jq-compatible query path to the clipboard.
pub fn copy_query(query: &str) -> Result<()> {
    copy_to_clipboard(query)
}

/// Copy a path as a JavaScript expression (e.g., `data.users[0].name`).
pub fn copy_path_js(query: &str) -> Result<()> {
    // jdx uses `.users[0].name` which is already JS-compatible
    let js_path = format!("data{query}");
    copy_to_clipboard(&js_path)
}

/// Copy a path as a Python expression (e.g., `data["users"][0]["name"]`).
pub fn copy_path_python(query: &str) -> Result<()> {
    let mut result = "data".to_string();
    let mut chars = query.chars().peekable();

    // Skip leading dot
    if chars.peek() == Some(&'.') {
        chars.next();
    }

    let mut current_key = String::new();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !current_key.is_empty() {
                    result.push_str(&format!("[\"{current_key}\"]"));
                    current_key.clear();
                }
            }
            '[' => {
                if !current_key.is_empty() {
                    result.push_str(&format!("[\"{current_key}\"]"));
                    current_key.clear();
                }
                result.push('[');
                // Copy everything until ]
                for c2 in chars.by_ref() {
                    result.push(c2);
                    if c2 == ']' {
                        break;
                    }
                }
            }
            _ => current_key.push(c),
        }
    }
    if !current_key.is_empty() {
        result.push_str(&format!("[\"{current_key}\"]"));
    }

    copy_to_clipboard(&result)
}
