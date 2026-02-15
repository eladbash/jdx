use anyhow::Result;
use serde_json::Value;

/// Parse a YAML string into a serde_json::Value.
pub fn parse(content: &str) -> Result<Value> {
    let value: Value = serde_yaml::from_str(content)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_yaml() {
        let yaml = "name: Alice\nage: 30\n";
        let result = parse(yaml).unwrap();
        assert_eq!(result, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_parse_yaml_list() {
        let yaml = "- 1\n- 2\n- 3\n";
        let result = parse(yaml).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_parse_yaml_nested() {
        let yaml = "users:\n  - name: Alice\n  - name: Bob\n";
        let result = parse(yaml).unwrap();
        assert_eq!(
            result,
            json!({"users": [{"name": "Alice"}, {"name": "Bob"}]})
        );
    }
}
