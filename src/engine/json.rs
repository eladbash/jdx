use serde_json::Value;

use super::query::PathSegment;

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
        }
    }

    TraversalResult {
        value: Some(current.clone()),
        parent: parent.cloned(),
        depth,
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
}
