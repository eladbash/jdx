use anyhow::{bail, Result};
use serde_json::Value;

/// Parse a TOML string into a serde_json::Value.
pub fn parse(content: &str) -> Result<Value> {
    let toml_value: toml::Value = content.parse()?;
    let json_value = toml_to_json(toml_value);
    Ok(json_value)
}

/// Serialize a JSON value to TOML format.
pub fn serialize(value: &Value) -> Result<String> {
    let toml_value = json_to_toml(value)?;
    Ok(toml::to_string_pretty(&toml_value)?)
}

fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, Value> = table
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect();
            Value::Object(map)
        }
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
    }
}

fn json_to_toml(value: &Value) -> Result<toml::Value> {
    match value {
        Value::Null => Ok(toml::Value::String("null".into())),
        Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                bail!("unsupported number for TOML")
            }
        }
        Value::String(s) => Ok(toml::Value::String(s.clone())),
        Value::Array(arr) => {
            let items: Result<Vec<toml::Value>> = arr.iter().map(json_to_toml).collect();
            Ok(toml::Value::Array(items?))
        }
        Value::Object(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map {
                table.insert(k.clone(), json_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
name = "Alice"
age = 30
"#;
        let result = parse(toml_str).unwrap();
        assert_eq!(result, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_parse_toml_nested() {
        let toml_str = r#"
[server]
host = "localhost"
port = 8080
"#;
        let result = parse(toml_str).unwrap();
        assert_eq!(
            result,
            json!({"server": {"host": "localhost", "port": 8080}})
        );
    }

    #[test]
    fn test_roundtrip() {
        let data = json!({"name": "Bob", "active": true, "count": 42});
        let toml_str = serialize(&data).unwrap();
        let parsed = parse(&toml_str).unwrap();
        assert_eq!(parsed, data);
    }
}
