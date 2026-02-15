use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::BTreeMap;

/// Parse and execute a transform command on a JSON value.
///
/// Transform commands start with `:` and operate on the current value.
/// Supported commands: :keys, :values, :count, :flatten, :pick, :omit, :sort, :uniq, :group_by
pub fn apply_transform(value: &Value, command: &str) -> Result<Value> {
    let command = command.trim();
    let (cmd, args) = match command.split_once(' ') {
        Some((c, a)) => (c, a.trim()),
        None => (command, ""),
    };

    match cmd {
        ":keys" => transform_keys(value),
        ":values" => transform_values(value),
        ":count" => transform_count(value),
        ":flatten" => transform_flatten(value),
        ":pick" => transform_pick(value, args),
        ":omit" => transform_omit(value, args),
        ":sort" => transform_sort(value, args),
        ":uniq" => transform_uniq(value),
        ":group_by" => transform_group_by(value, args),
        _ => bail!("unknown transform command: {cmd}"),
    }
}

/// Return the keys of an object as an array of strings.
fn transform_keys(value: &Value) -> Result<Value> {
    match value {
        Value::Object(map) => {
            let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
            Ok(Value::Array(keys))
        }
        _ => bail!(":keys requires an object"),
    }
}

/// Return the values of an object as an array.
fn transform_values(value: &Value) -> Result<Value> {
    match value {
        Value::Object(map) => {
            let values: Vec<Value> = map.values().cloned().collect();
            Ok(Value::Array(values))
        }
        _ => bail!(":values requires an object"),
    }
}

/// Return the count of elements in an array or keys in an object.
fn transform_count(value: &Value) -> Result<Value> {
    match value {
        Value::Array(arr) => Ok(Value::Number(arr.len().into())),
        Value::Object(map) => Ok(Value::Number(map.len().into())),
        _ => bail!(":count requires an array or object"),
    }
}

/// Flatten nested arrays one level.
fn transform_flatten(value: &Value) -> Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut result = Vec::new();
            for item in arr {
                match item {
                    Value::Array(inner) => result.extend(inner.clone()),
                    other => result.push(other.clone()),
                }
            }
            Ok(Value::Array(result))
        }
        _ => bail!(":flatten requires an array"),
    }
}

/// Pick specific fields from objects in an array.
/// Usage: `:pick name,email`
fn transform_pick(value: &Value, args: &str) -> Result<Value> {
    if args.is_empty() {
        bail!(":pick requires field names (e.g., :pick name,email)");
    }
    let fields: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

    match value {
        Value::Array(arr) => {
            let result: Vec<Value> = arr
                .iter()
                .map(|item| {
                    if let Value::Object(map) = item {
                        let picked: serde_json::Map<String, Value> = map
                            .iter()
                            .filter(|(k, _)| fields.contains(&k.as_str()))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Value::Object(picked)
                    } else {
                        item.clone()
                    }
                })
                .collect();
            Ok(Value::Array(result))
        }
        Value::Object(map) => {
            let picked: serde_json::Map<String, Value> = map
                .iter()
                .filter(|(k, _)| fields.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(Value::Object(picked))
        }
        _ => bail!(":pick requires an array of objects or an object"),
    }
}

/// Omit specific fields from objects in an array.
/// Usage: `:omit metadata,internal`
fn transform_omit(value: &Value, args: &str) -> Result<Value> {
    if args.is_empty() {
        bail!(":omit requires field names (e.g., :omit metadata,internal)");
    }
    let fields: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

    match value {
        Value::Array(arr) => {
            let result: Vec<Value> = arr
                .iter()
                .map(|item| {
                    if let Value::Object(map) = item {
                        let omitted: serde_json::Map<String, Value> = map
                            .iter()
                            .filter(|(k, _)| !fields.contains(&k.as_str()))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Value::Object(omitted)
                    } else {
                        item.clone()
                    }
                })
                .collect();
            Ok(Value::Array(result))
        }
        Value::Object(map) => {
            let omitted: serde_json::Map<String, Value> = map
                .iter()
                .filter(|(k, _)| !fields.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(Value::Object(omitted))
        }
        _ => bail!(":omit requires an array of objects or an object"),
    }
}

/// Sort an array of objects by a field.
/// Usage: `:sort name` or `:sort age`
fn transform_sort(value: &Value, args: &str) -> Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut sorted = arr.clone();
            if args.is_empty() {
                // Sort primitives by their string representation
                sorted.sort_by_key(|a| a.to_string());
            } else {
                let field = args.trim();
                sorted.sort_by(|a, b| {
                    let a_val = a.get(field).unwrap_or(&Value::Null);
                    let b_val = b.get(field).unwrap_or(&Value::Null);
                    compare_values(a_val, b_val)
                });
            }
            Ok(Value::Array(sorted))
        }
        _ => bail!(":sort requires an array"),
    }
}

/// Remove consecutive duplicate values from an array.
fn transform_uniq(value: &Value) -> Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut seen = Vec::new();
            for item in arr {
                let s = serde_json::to_string(item).unwrap_or_default();
                if !seen
                    .iter()
                    .any(|(_, existing_s): &(Value, String)| existing_s == &s)
                {
                    seen.push((item.clone(), s));
                }
            }
            let result: Vec<Value> = seen.into_iter().map(|(v, _)| v).collect();
            Ok(Value::Array(result))
        }
        _ => bail!(":uniq requires an array"),
    }
}

/// Group array elements by a field value.
/// Usage: `:group_by type`
fn transform_group_by(value: &Value, args: &str) -> Result<Value> {
    if args.is_empty() {
        bail!(":group_by requires a field name (e.g., :group_by type)");
    }
    let field = args.trim();

    match value {
        Value::Array(arr) => {
            let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for item in arr {
                let key = match item.get(field) {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => "null".to_string(),
                };
                groups.entry(key).or_default().push(item.clone());
            }
            let result: serde_json::Map<String, Value> = groups
                .into_iter()
                .map(|(k, v)| (k, Value::Array(v)))
                .collect();
            Ok(Value::Object(result))
        }
        _ => bail!(":group_by requires an array"),
    }
}

/// Compare two JSON values for sorting.
fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a = a.as_f64().unwrap_or(0.0);
            let b = b.as_f64().unwrap_or(0.0);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(a), Value::String(b)) => a.cmp(b),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_keys() {
        let data = json!({"b": 2, "a": 1});
        let result = apply_transform(&data, ":keys").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_values() {
        let data = json!({"a": 1, "b": 2});
        let result = apply_transform(&data, ":values").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_count_array() {
        let data = json!([1, 2, 3]);
        let result = apply_transform(&data, ":count").unwrap();
        assert_eq!(result, json!(3));
    }

    #[test]
    fn test_count_object() {
        let data = json!({"a": 1, "b": 2});
        let result = apply_transform(&data, ":count").unwrap();
        assert_eq!(result, json!(2));
    }

    #[test]
    fn test_flatten() {
        let data = json!([[1, 2], [3, 4], 5]);
        let result = apply_transform(&data, ":flatten").unwrap();
        assert_eq!(result, json!([1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_pick() {
        let data = json!([
            {"name": "Alice", "age": 30, "email": "a@test.com"},
            {"name": "Bob", "age": 25, "email": "b@test.com"}
        ]);
        let result = apply_transform(&data, ":pick name,email").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0], json!({"name": "Alice", "email": "a@test.com"}));
        assert_eq!(arr[1], json!({"name": "Bob", "email": "b@test.com"}));
    }

    #[test]
    fn test_omit() {
        let data = json!({"name": "Alice", "age": 30, "secret": "x"});
        let result = apply_transform(&data, ":omit secret").unwrap();
        assert_eq!(result, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_sort_by_field() {
        let data = json!([
            {"name": "Charlie", "age": 35},
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 30}
        ]);
        let result = apply_transform(&data, ":sort name").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[1]["name"], "Bob");
        assert_eq!(arr[2]["name"], "Charlie");
    }

    #[test]
    fn test_sort_primitives() {
        let data = json!([3, 1, 2]);
        let result = apply_transform(&data, ":sort").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_uniq() {
        let data = json!([1, 2, 2, 3, 1, 3]);
        let result = apply_transform(&data, ":uniq").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_group_by() {
        let data = json!([
            {"type": "fruit", "name": "apple"},
            {"type": "veg", "name": "carrot"},
            {"type": "fruit", "name": "banana"}
        ]);
        let result = apply_transform(&data, ":group_by type").unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj["fruit"].as_array().unwrap().len(), 2);
        assert_eq!(obj["veg"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_unknown_command() {
        let data = json!([1, 2, 3]);
        let result = apply_transform(&data, ":unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_keys_on_non_object() {
        let data = json!([1, 2]);
        let result = apply_transform(&data, ":keys");
        assert!(result.is_err());
    }

    #[test]
    fn test_flatten_on_non_array() {
        let data = json!({"a": 1});
        let result = apply_transform(&data, ":flatten");
        assert!(result.is_err());
    }

    #[test]
    fn test_pick_empty_args() {
        let data = json!([{"a": 1}]);
        let result = apply_transform(&data, ":pick");
        assert!(result.is_err());
    }

    #[test]
    fn test_group_by_missing_field() {
        let data = json!([
            {"type": "a", "name": "x"},
            {"name": "y"}
        ]);
        let result = apply_transform(&data, ":group_by type").unwrap();
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("a"));
        assert!(obj.contains_key("null"));
    }
}
