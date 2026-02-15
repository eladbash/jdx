use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::BTreeMap;

use super::json::eval_predicate;
use super::query::parse_predicate;

/// Parse and execute one or more chained transform commands on a JSON value.
///
/// Transform commands start with `:` and operate on the current value.
/// Multiple commands can be chained: `:pick name,age :sort age`
/// Supported commands: :keys, :values, :count, :flatten, :pick, :omit, :sort, :uniq, :group_by
pub fn apply_transform(value: &Value, command: &str) -> Result<Value> {
    let commands = split_chain(command);
    let mut result = value.clone();
    for single_cmd in commands {
        result = apply_single_transform(&result, &single_cmd)?;
    }
    Ok(result)
}

/// Split a (possibly chained) transform string into individual commands.
/// e.g. ":pick name,age :sort age" → [":pick name,age", ":sort age"]
fn split_chain(input: &str) -> Vec<String> {
    let input = input.trim();
    let mut commands = Vec::new();
    let mut current_start = 0;

    // Walk through the string looking for ` :` boundaries (space then colon)
    // that start a new transform command.
    let bytes = input.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b':' && bytes[i - 1] == b' ' {
            // Check if this is the very first command (starts at position 0 with `:`)
            if i > current_start {
                let chunk = input[current_start..i].trim();
                if !chunk.is_empty() {
                    commands.push(chunk.to_string());
                }
                current_start = i;
            }
        }
    }
    // Push the last segment
    let chunk = input[current_start..].trim();
    if !chunk.is_empty() {
        commands.push(chunk.to_string());
    }
    commands
}

/// Apply a single transform command (no chaining).
fn apply_single_transform(value: &Value, command: &str) -> Result<Value> {
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
        ":filter" => transform_filter(value, args),
        ":sum" => transform_sum(value, args),
        ":avg" => transform_avg(value, args),
        ":min" => transform_min(value, args),
        ":max" => transform_max(value, args),
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

/// Filter array elements by a predicate.
/// Usage: `:filter price < 10` or `:filter name == "Alice"`
fn transform_filter(value: &Value, args: &str) -> Result<Value> {
    if args.is_empty() {
        bail!(":filter requires a predicate (e.g., :filter price < 10)");
    }

    let pred =
        parse_predicate(args).map_err(|e| anyhow::anyhow!(":filter invalid predicate: {e}"))?;

    match value {
        Value::Array(arr) => {
            let filtered: Vec<Value> = arr
                .iter()
                .filter(|item| eval_predicate(item, &pred))
                .cloned()
                .collect();
            Ok(Value::Array(filtered))
        }
        _ => bail!(":filter requires an array"),
    }
}

/// Sum numeric values in an array, or sum a specific field from objects.
/// Usage: `:sum` or `:sum price`
fn transform_sum(value: &Value, args: &str) -> Result<Value> {
    let nums = extract_numbers(value, args, ":sum")?;
    let total: f64 = nums.iter().sum();
    Ok(number_to_value(total))
}

/// Average numeric values in an array, or average a specific field from objects.
/// Usage: `:avg` or `:avg price`
fn transform_avg(value: &Value, args: &str) -> Result<Value> {
    let nums = extract_numbers(value, args, ":avg")?;
    if nums.is_empty() {
        return Ok(Value::Null);
    }
    let total: f64 = nums.iter().sum();
    let avg = total / nums.len() as f64;
    Ok(number_to_value(avg))
}

/// Minimum value in an array, or minimum of a specific field from objects.
/// Usage: `:min` or `:min price`
fn transform_min(value: &Value, args: &str) -> Result<Value> {
    let nums = extract_numbers(value, args, ":min")?;
    match nums.iter().copied().reduce(f64::min) {
        Some(v) => Ok(number_to_value(v)),
        None => Ok(Value::Null),
    }
}

/// Maximum value in an array, or maximum of a specific field from objects.
/// Usage: `:max` or `:max price`
fn transform_max(value: &Value, args: &str) -> Result<Value> {
    let nums = extract_numbers(value, args, ":max")?;
    match nums.iter().copied().reduce(f64::max) {
        Some(v) => Ok(number_to_value(v)),
        None => Ok(Value::Null),
    }
}

/// Extract numeric values from an array. If `field` is given, extract from objects.
fn extract_numbers(value: &Value, args: &str, cmd_name: &str) -> Result<Vec<f64>> {
    let field = args.trim();
    match value {
        Value::Array(arr) => {
            let mut nums = Vec::new();
            for item in arr {
                let v = if field.is_empty() {
                    item
                } else {
                    item.get(field).unwrap_or(&Value::Null)
                };
                if let Some(n) = v.as_f64() {
                    nums.push(n);
                }
            }
            Ok(nums)
        }
        _ => bail!("{cmd_name} requires an array"),
    }
}

/// Convert an f64 to a JSON Value, using integer representation when possible.
fn number_to_value(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < (i64::MAX as f64) {
        Value::Number(serde_json::Number::from(n as i64))
    } else {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null)
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

    // --- :filter transform tests ---

    #[test]
    fn test_filter_number_lt() {
        let data = json!([
            {"name": "A", "price": 5},
            {"name": "B", "price": 15},
            {"name": "C", "price": 8}
        ]);
        let result = apply_transform(&data, ":filter price < 10").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "A");
        assert_eq!(arr[1]["name"], "C");
    }

    #[test]
    fn test_filter_string_eq() {
        let data = json!([
            {"name": "Alice", "role": "admin"},
            {"name": "Bob", "role": "user"}
        ]);
        let result = apply_transform(&data, ":filter role == \"admin\"").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Alice");
    }

    #[test]
    fn test_filter_ge() {
        let data = json!([
            {"score": 85},
            {"score": 90},
            {"score": 95}
        ]);
        let result = apply_transform(&data, ":filter score >= 90").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_filter_chained_with_pick() {
        let data = json!([
            {"name": "Alice", "age": 25, "email": "a@t.com"},
            {"name": "Bob", "age": 35, "email": "b@t.com"},
            {"name": "Carol", "age": 40, "email": "c@t.com"}
        ]);
        let result = apply_transform(&data, ":filter age > 30 :pick name,age").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], json!({"name": "Bob", "age": 35}));
        assert_eq!(arr[1], json!({"name": "Carol", "age": 40}));
    }

    #[test]
    fn test_filter_empty_args() {
        let data = json!([1, 2, 3]);
        let result = apply_transform(&data, ":filter");
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_on_non_array() {
        let data = json!({"a": 1});
        let result = apply_transform(&data, ":filter a > 0");
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_bool_value() {
        let data = json!([
            {"name": "Alice", "active": true},
            {"name": "Bob", "active": false}
        ]);
        let result = apply_transform(&data, ":filter active == true").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Alice");
    }

    #[test]
    fn test_filter_chained_with_sort_and_count() {
        let data = json!([
            {"name": "A", "price": 5},
            {"name": "B", "price": 15},
            {"name": "C", "price": 3},
            {"name": "D", "price": 8}
        ]);
        let result = apply_transform(&data, ":filter price < 10 :sort price :count").unwrap();
        assert_eq!(result, json!(3));
    }

    // --- :sum, :avg, :min, :max tests ---

    #[test]
    fn test_sum_primitives() {
        let data = json!([1, 2, 3, 4]);
        let result = apply_transform(&data, ":sum").unwrap();
        assert_eq!(result, json!(10));
    }

    #[test]
    fn test_sum_by_field() {
        let data = json!([
            {"name": "A", "price": 10.99},
            {"name": "B", "price": 8.99},
            {"name": "C", "price": 32.99}
        ]);
        let result = apply_transform(&data, ":sum price").unwrap();
        // 10.99 + 8.99 + 32.99 = 52.97
        assert_eq!(result.as_f64().unwrap(), 52.97);
    }

    #[test]
    fn test_avg_primitives() {
        let data = json!([10, 20, 30]);
        let result = apply_transform(&data, ":avg").unwrap();
        assert_eq!(result, json!(20));
    }

    #[test]
    fn test_avg_by_field() {
        let data = json!([
            {"score": 80},
            {"score": 90},
            {"score": 100}
        ]);
        let result = apply_transform(&data, ":avg score").unwrap();
        assert_eq!(result, json!(90));
    }

    #[test]
    fn test_avg_empty() {
        let data = json!([]);
        let result = apply_transform(&data, ":avg").unwrap();
        assert_eq!(result, json!(null));
    }

    #[test]
    fn test_min_primitives() {
        let data = json!([5, 2, 8, 1, 9]);
        let result = apply_transform(&data, ":min").unwrap();
        assert_eq!(result, json!(1));
    }

    #[test]
    fn test_min_by_field() {
        let data = json!([
            {"name": "A", "price": 10.99},
            {"name": "B", "price": 8.99},
            {"name": "C", "price": 32.99}
        ]);
        let result = apply_transform(&data, ":min price").unwrap();
        assert_eq!(result.as_f64().unwrap(), 8.99);
    }

    #[test]
    fn test_max_primitives() {
        let data = json!([5, 2, 8, 1, 9]);
        let result = apply_transform(&data, ":max").unwrap();
        assert_eq!(result, json!(9));
    }

    #[test]
    fn test_max_by_field() {
        let data = json!([
            {"name": "A", "price": 10.99},
            {"name": "B", "price": 8.99},
            {"name": "C", "price": 32.99}
        ]);
        let result = apply_transform(&data, ":max price").unwrap();
        assert_eq!(result.as_f64().unwrap(), 32.99);
    }

    #[test]
    fn test_sum_chained_with_filter() {
        let data = json!([
            {"name": "A", "price": 5},
            {"name": "B", "price": 15},
            {"name": "C", "price": 8}
        ]);
        let result = apply_transform(&data, ":filter price < 10 :sum price").unwrap();
        assert_eq!(result, json!(13));
    }

    #[test]
    fn test_sum_on_non_array() {
        let data = json!({"a": 1});
        let result = apply_transform(&data, ":sum");
        assert!(result.is_err());
    }
}
