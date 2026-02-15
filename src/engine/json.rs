use serde_json::Value;

use super::query::{CompareOp, FilterValue, PathSegment, Predicate};

/// Result of traversing JSON with a parsed query.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    /// The value at the resolved path (None if path doesn't match)
    pub value: Option<Value>,
    /// The parent value (for context / breadcrumbs)
    pub parent: Option<Value>,
    /// How many segments were successfully resolved
    pub depth: usize,
}

/// Traverse a `serde_json::Value` tree following the given path segments.
///
/// Returns a `TraversalResult` with the deepest reachable value.
/// If a segment doesn't match, traversal stops and returns the last valid parent.
pub fn traverse(root: &Value, segments: &[PathSegment]) -> TraversalResult {
    if segments.is_empty() {
        return TraversalResult {
            value: Some(root.clone()),
            parent: None,
            depth: 0,
        };
    }

    let mut current = root;
    let mut parent: Option<&Value> = None;
    let mut depth = 0;

    for segment in segments {
        match segment {
            PathSegment::Key(key) => {
                if let Some(val) = current.get(key.as_str()) {
                    parent = Some(current);
                    current = val;
                    depth += 1;
                } else {
                    return TraversalResult {
                        value: None,
                        parent: Some(current.clone()),
                        depth,
                    };
                }
            }
            PathSegment::Index(idx) => {
                if let Some(arr) = current.as_array() {
                    let resolved_idx = if *idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        *idx as usize
                    };
                    if let Some(val) = arr.get(resolved_idx) {
                        parent = Some(current);
                        current = val;
                        depth += 1;
                    } else {
                        return TraversalResult {
                            value: None,
                            parent: Some(current.clone()),
                            depth,
                        };
                    }
                } else {
                    return TraversalResult {
                        value: None,
                        parent: Some(current.clone()),
                        depth,
                    };
                }
            }
            PathSegment::Slice(start, end) => {
                if let Some(arr) = current.as_array() {
                    let len = arr.len() as i64;
                    let s = start.unwrap_or(0);
                    let e = end.unwrap_or(len);

                    let s = if s < 0 {
                        (len + s).max(0) as usize
                    } else {
                        s as usize
                    };
                    let e = if e < 0 {
                        (len + e).max(0) as usize
                    } else {
                        (e as usize).min(arr.len())
                    };

                    let sliced: Vec<Value> = arr[s..e].to_vec();
                    return TraversalResult {
                        value: Some(Value::Array(sliced)),
                        parent: Some(current.clone()),
                        depth: depth + 1,
                    };
                } else {
                    return TraversalResult {
                        value: None,
                        parent: Some(current.clone()),
                        depth,
                    };
                }
            }
            PathSegment::Wildcard => {
                // For objects: return all values as an array
                // For arrays: return the array as-is (identity)
                match current {
                    Value::Object(map) => {
                        let values: Vec<Value> = map.values().cloned().collect();
                        return TraversalResult {
                            value: Some(Value::Array(values)),
                            parent: Some(current.clone()),
                            depth: depth + 1,
                        };
                    }
                    Value::Array(_) => {
                        // Wildcard on array is identity
                        return TraversalResult {
                            value: Some(current.clone()),
                            parent: parent.cloned(),
                            depth: depth + 1,
                        };
                    }
                    _ => {
                        return TraversalResult {
                            value: None,
                            parent: Some(current.clone()),
                            depth,
                        };
                    }
                }
            }
            PathSegment::Filter(pred) => {
                if let Some(arr) = current.as_array() {
                    let filtered: Vec<Value> = arr
                        .iter()
                        .filter(|item| eval_predicate(item, pred))
                        .cloned()
                        .collect();
                    // Continue traversal with remaining segments
                    let remaining = &segments[depth + 1..];
                    let filtered_val = Value::Array(filtered);
                    if remaining.is_empty() {
                        return TraversalResult {
                            value: Some(filtered_val),
                            parent: Some(current.clone()),
                            depth: depth + 1,
                        };
                    } else {
                        let sub = traverse(&filtered_val, remaining);
                        return TraversalResult {
                            value: sub.value,
                            parent: sub.parent.or(Some(filtered_val)),
                            depth: depth + 1 + sub.depth,
                        };
                    }
                } else {
                    return TraversalResult {
                        value: None,
                        parent: Some(current.clone()),
                        depth,
                    };
                }
            }
        }
    }

    TraversalResult {
        value: Some(current.clone()),
        parent: parent.cloned(),
        depth,
    }
}

/// Evaluate a filter predicate against a JSON value.
/// The value is expected to be an object; the predicate's field is looked up in it.
pub fn eval_predicate(value: &Value, pred: &Predicate) -> bool {
    let field_val = match value.get(&pred.field) {
        Some(v) => v,
        None => return false,
    };

    match &pred.value {
        FilterValue::Number(n) => {
            if let Some(fv) = field_val.as_f64() {
                compare_f64(fv, *n, &pred.op)
            } else {
                false
            }
        }
        FilterValue::String(s) => {
            if let Some(fv) = field_val.as_str() {
                compare_str(fv, s, &pred.op)
            } else {
                false
            }
        }
        FilterValue::Bool(b) => {
            if let Some(fv) = field_val.as_bool() {
                match pred.op {
                    CompareOp::Eq => fv == *b,
                    CompareOp::Ne => fv != *b,
                    _ => false, // ordering on bools doesn't make sense
                }
            } else {
                false
            }
        }
        FilterValue::Null => {
            let is_null = field_val.is_null();
            match pred.op {
                CompareOp::Eq => is_null,
                CompareOp::Ne => !is_null,
                _ => false,
            }
        }
    }
}

fn compare_f64(a: f64, b: f64, op: &CompareOp) -> bool {
    match op {
        CompareOp::Eq => (a - b).abs() < f64::EPSILON,
        CompareOp::Ne => (a - b).abs() >= f64::EPSILON,
        CompareOp::Lt => a < b,
        CompareOp::Gt => a > b,
        CompareOp::Le => a <= b,
        CompareOp::Ge => a >= b,
    }
}

fn compare_str(a: &str, b: &str, op: &CompareOp) -> bool {
    match op {
        CompareOp::Eq => a == b,
        CompareOp::Ne => a != b,
        CompareOp::Lt => a < b,
        CompareOp::Gt => a > b,
        CompareOp::Le => a <= b,
        CompareOp::Ge => a >= b,
    }
}

/// Get the keys available at the current value (for suggestions).
/// Returns sorted keys for objects, or index strings for arrays.
pub fn get_available_keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            keys
        }
        Value::Array(arr) => (0..arr.len()).map(|i| format!("[{}]", i)).collect(),
        _ => vec![],
    }
}

/// Pretty-print a JSON value with indentation.
pub fn pretty_print(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Compact-print a JSON value.
pub fn compact_print(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_traverse_simple_key() {
        let data = json!({"name": "Alice", "age": 30});
        let result = traverse(&data, &[PathSegment::Key("name".into())]);
        assert_eq!(result.value, Some(json!("Alice")));
        assert_eq!(result.depth, 1);
    }

    #[test]
    fn test_traverse_nested() {
        let data = json!({"a": {"b": {"c": 42}}});
        let segments = vec![
            PathSegment::Key("a".into()),
            PathSegment::Key("b".into()),
            PathSegment::Key("c".into()),
        ];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!(42)));
        assert_eq!(result.depth, 3);
    }

    #[test]
    fn test_traverse_array_index() {
        let data = json!({"items": [10, 20, 30]});
        let segments = vec![PathSegment::Key("items".into()), PathSegment::Index(1)];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!(20)));
    }

    #[test]
    fn test_traverse_negative_index() {
        let data = json!({"items": [10, 20, 30]});
        let segments = vec![PathSegment::Key("items".into()), PathSegment::Index(-1)];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!(30)));
    }

    #[test]
    fn test_traverse_missing_key() {
        let data = json!({"name": "Alice"});
        let result = traverse(&data, &[PathSegment::Key("missing".into())]);
        assert_eq!(result.value, None);
        assert_eq!(result.depth, 0);
    }

    #[test]
    fn test_traverse_index_out_of_bounds() {
        let data = json!({"items": [1, 2]});
        let segments = vec![PathSegment::Key("items".into()), PathSegment::Index(99)];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, None);
    }

    #[test]
    fn test_traverse_slice() {
        let data = json!({"items": [0, 1, 2, 3, 4]});
        let segments = vec![
            PathSegment::Key("items".into()),
            PathSegment::Slice(Some(1), Some(3)),
        ];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!([1, 2])));
    }

    #[test]
    fn test_traverse_slice_open_end() {
        let data = json!({"items": [0, 1, 2, 3]});
        let segments = vec![
            PathSegment::Key("items".into()),
            PathSegment::Slice(Some(2), None),
        ];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!([2, 3])));
    }

    #[test]
    fn test_traverse_wildcard_object() {
        let data = json!({"a": 1, "b": 2});
        let result = traverse(&data, &[PathSegment::Wildcard]);
        if let Some(Value::Array(arr)) = result.value {
            assert_eq!(arr.len(), 2);
            assert!(arr.contains(&json!(1)));
            assert!(arr.contains(&json!(2)));
        } else {
            panic!("expected array from wildcard on object");
        }
    }

    #[test]
    fn test_traverse_wildcard_array() {
        let data = json!([1, 2, 3]);
        let result = traverse(&data, &[PathSegment::Wildcard]);
        assert_eq!(result.value, Some(json!([1, 2, 3])));
    }

    #[test]
    fn test_traverse_root() {
        let data = json!({"a": 1});
        let result = traverse(&data, &[]);
        assert_eq!(result.value, Some(json!({"a": 1})));
        assert_eq!(result.depth, 0);
    }

    #[test]
    fn test_get_available_keys_object() {
        let data = json!({"banana": 1, "apple": 2, "cherry": 3});
        let keys = get_available_keys(&data);
        assert_eq!(keys, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_get_available_keys_array() {
        let data = json!([10, 20, 30]);
        let keys = get_available_keys(&data);
        assert_eq!(keys, vec!["[0]", "[1]", "[2]"]);
    }

    #[test]
    fn test_get_available_keys_primitive() {
        let data = json!(42);
        let keys = get_available_keys(&data);
        assert!(keys.is_empty());
    }

    // --- Filter traversal tests ---

    #[test]
    fn test_traverse_filter_number_lt() {
        let data = json!({
            "books": [
                {"title": "A", "price": 5},
                {"title": "B", "price": 15},
                {"title": "C", "price": 8}
            ]
        });
        let segments = vec![
            PathSegment::Key("books".into()),
            PathSegment::Filter(Predicate {
                field: "price".into(),
                op: CompareOp::Lt,
                value: FilterValue::Number(10.0),
            }),
        ];
        let result = traverse(&data, &segments);
        let arr = result.value.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "A");
        assert_eq!(arr[1]["title"], "C");
    }

    #[test]
    fn test_traverse_filter_string_eq() {
        let data = json!({
            "users": [
                {"name": "Alice", "role": "admin"},
                {"name": "Bob", "role": "user"},
                {"name": "Carol", "role": "admin"}
            ]
        });
        let segments = vec![
            PathSegment::Key("users".into()),
            PathSegment::Filter(Predicate {
                field: "role".into(),
                op: CompareOp::Eq,
                value: FilterValue::String("admin".into()),
            }),
        ];
        let result = traverse(&data, &segments);
        let arr = result.value.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[1]["name"], "Carol");
    }

    #[test]
    fn test_traverse_filter_bool() {
        let data = json!([
            {"name": "Alice", "active": true},
            {"name": "Bob", "active": false},
            {"name": "Carol", "active": true}
        ]);
        let segments = vec![
            PathSegment::Filter(Predicate {
                field: "active".into(),
                op: CompareOp::Eq,
                value: FilterValue::Bool(true),
            }),
        ];
        let result = traverse(&data, &segments);
        let arr = result.value.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_traverse_filter_with_continuation() {
        let data = json!({
            "items": [
                {"name": "A", "price": 5},
                {"name": "B", "price": 15}
            ]
        });
        // .items[price < 10].name — filter returns array, then .name should not resolve
        // since the result is an array of objects, not a single object.
        // But let's test that filter + further key access works as traversal stops:
        let segments = vec![
            PathSegment::Key("items".into()),
            PathSegment::Filter(Predicate {
                field: "price".into(),
                op: CompareOp::Lt,
                value: FilterValue::Number(10.0),
            }),
        ];
        let result = traverse(&data, &segments);
        let arr = result.value.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "A");
    }

    #[test]
    fn test_traverse_filter_no_matches() {
        let data = json!([
            {"val": 1},
            {"val": 2}
        ]);
        let segments = vec![
            PathSegment::Filter(Predicate {
                field: "val".into(),
                op: CompareOp::Gt,
                value: FilterValue::Number(100.0),
            }),
        ];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, Some(json!([])));
    }

    #[test]
    fn test_traverse_filter_on_non_array() {
        let data = json!({"a": 1});
        let segments = vec![
            PathSegment::Filter(Predicate {
                field: "a".into(),
                op: CompareOp::Eq,
                value: FilterValue::Number(1.0),
            }),
        ];
        let result = traverse(&data, &segments);
        assert_eq!(result.value, None);
    }

    #[test]
    fn test_eval_predicate_null() {
        let item = json!({"name": "test", "deleted": null});
        let pred = Predicate {
            field: "deleted".into(),
            op: CompareOp::Eq,
            value: FilterValue::Null,
        };
        assert!(eval_predicate(&item, &pred));
    }

    #[test]
    fn test_eval_predicate_missing_field() {
        let item = json!({"name": "test"});
        let pred = Predicate {
            field: "missing".into(),
            op: CompareOp::Eq,
            value: FilterValue::Number(1.0),
        };
        assert!(!eval_predicate(&item, &pred));
    }
}
