use jdx::engine::json::traverse;
use jdx::engine::query::{parse, PathSegment};
use serde_json::json;

/// End-to-end: parse query string then traverse JSON.
#[test]
fn test_e2e_simple_key_access() {
    let data = json!({"name": "Alice", "age": 30});
    let segments = parse(".name").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("Alice")));
}

#[test]
fn test_e2e_nested_path() {
    let data = json!({"store": {"books": [{"title": "1984"}]}});
    let segments = parse(".store.books[0].title").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("1984")));
}

#[test]
fn test_e2e_root_returns_full_document() {
    let data = json!({"a": 1, "b": 2});
    let segments = parse(".").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(data));
}

#[test]
fn test_e2e_missing_path_returns_none() {
    let data = json!({"name": "Alice"});
    let segments = parse(".missing.deep.path").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, None);
}

#[test]
fn test_e2e_array_slice() {
    let data = json!({"items": [0, 1, 2, 3, 4, 5]});
    let segments = parse(".items[1:4]").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!([1, 2, 3])));
}

#[test]
fn test_e2e_negative_index() {
    let data = json!({"items": ["a", "b", "c"]});
    let segments = parse(".items[-1]").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("c")));
}

#[test]
fn test_e2e_wildcard_on_object() {
    let data = json!({"a": 1, "b": 2, "c": 3});
    let segments = parse(".*").unwrap();
    let result = traverse(&data, &segments);
    let arr = result.value.unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 3);
}

#[test]
fn test_e2e_quoted_key_with_dots() {
    let data = json!({"key.with.dots": "found"});
    let segments = parse(".[\"key.with.dots\"]").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("found")));
}

#[test]
fn test_e2e_fixture_nested() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Access a deep path
    let segments = parse(".store.books[1].metadata.publisher.name").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("Signet Classics")));
}

#[test]
fn test_e2e_fixture_nested_users() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".users[0].name").unwrap();
    let result = traverse(&data, &segments);
    assert_eq!(result.value, Some(json!("Alice")));
}

#[test]
fn test_parse_segments_match_types() {
    let segments = parse(".store.books[0].tags[*]").unwrap();
    assert_eq!(
        segments,
        vec![
            PathSegment::Key("store".into()),
            PathSegment::Key("books".into()),
            PathSegment::Index(0),
            PathSegment::Key("tags".into()),
            PathSegment::Wildcard,
        ]
    );
}
