use anyhow::Result;
use serde_json::Value;

/// Parse a JSON string into a serde_json::Value.
pub fn parse(content: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(content)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_object() {
        let result = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(result, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_parse_array() {
        let result = parse("[1, 2, 3]").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse("{invalid}");
        assert!(result.is_err());
    }
}
