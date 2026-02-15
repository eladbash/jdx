use anyhow::{bail, Result};
use serde_json::Value;

/// Parse a CSV string into a JSON array of objects.
/// The first row is treated as headers.
pub fn parse(content: &str) -> Result<Value> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let mut obj = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
            // Try to parse as number or bool, fall back to string
            let value = if let Ok(n) = field.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = field.parse::<f64>() {
                serde_json::Number::from_f64(f).map_or(Value::String(field.into()), Value::Number)
            } else if field == "true" {
                Value::Bool(true)
            } else if field == "false" {
                Value::Bool(false)
            } else if field.is_empty() {
                Value::Null
            } else {
                Value::String(field.to_string())
            };
            obj.insert(key, value);
        }
        rows.push(Value::Object(obj));
    }

    Ok(Value::Array(rows))
}

/// Serialize a JSON array of objects to CSV format.
pub fn serialize(value: &Value) -> Result<String> {
    let arr = match value {
        Value::Array(arr) => arr,
        _ => bail!(
            "CSV output requires a JSON array of objects.\n\
             Hint: use -Q to query into an array first, e.g.:\n  \
             jdx -Q '.items' --output csv data.json"
        ),
    };

    if arr.is_empty() {
        return Ok(String::new());
    }

    // Collect all unique headers from all objects
    let mut headers = Vec::new();
    for item in arr {
        if let Value::Object(map) = item {
            for key in map.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(&headers)?;

    for item in arr {
        if let Value::Object(map) = item {
            let row: Vec<String> = headers
                .iter()
                .map(|h| match map.get(h) {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Null) | None => String::new(),
                    Some(v) => v.to_string(),
                })
                .collect();
            wtr.write_record(&row)?;
        }
    }

    let bytes = wtr.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_csv() {
        let csv = "name,age\nAlice,30\nBob,25\n";
        let result = parse(csv).unwrap();
        assert_eq!(
            result,
            json!([
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ])
        );
    }

    #[test]
    fn test_parse_csv_with_booleans() {
        let csv = "name,active\nAlice,true\nBob,false\n";
        let result = parse(csv).unwrap();
        assert_eq!(
            result,
            json!([
                {"name": "Alice", "active": true},
                {"name": "Bob", "active": false}
            ])
        );
    }

    #[test]
    fn test_roundtrip() {
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);
        let csv_str = serialize(&data).unwrap();
        let parsed = parse(&csv_str).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn test_serialize_non_array() {
        let data = json!({"name": "Alice"});
        assert!(serialize(&data).is_err());
    }
}
