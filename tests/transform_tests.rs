use jdx::engine::json::traverse;
use jdx::engine::query::parse;
use jdx::engine::transform::apply_transform;
use serde_json::json;

/// Test transforms on fixture data (query + transform combined).

#[test]
fn test_transform_on_fixture_pick() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Navigate to .users then pick name
    let segments = parse(".users").unwrap();
    let result = traverse(&data, &segments);
    let users = result.value.unwrap();

    let picked = apply_transform(&users, ":pick name").unwrap();
    let arr = picked.as_array().unwrap();
    assert_eq!(arr[0], json!({"name": "Alice"}));
    assert_eq!(arr[1], json!({"name": "Bob"}));
}

#[test]
fn test_transform_sort_by_name() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".users").unwrap();
    let result = traverse(&data, &segments);
    let users = result.value.unwrap();

    let sorted = apply_transform(&users, ":sort name").unwrap();
    let arr = sorted.as_array().unwrap();
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[1]["name"], "Bob");
    assert_eq!(arr[2]["name"], "Charlie");
}

#[test]
fn test_transform_count_on_array() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let count = apply_transform(&books, ":count").unwrap();
    assert_eq!(count, json!(3));
}

#[test]
fn test_transform_keys_on_store() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store").unwrap();
    let result = traverse(&data, &segments);
    let store = result.value.unwrap();

    let keys = apply_transform(&store, ":keys").unwrap();
    let arr = keys.as_array().unwrap();
    assert!(arr.contains(&json!("name")));
    assert!(arr.contains(&json!("books")));
    assert!(arr.contains(&json!("location")));
}

#[test]
fn test_transform_group_by_role() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".users").unwrap();
    let result = traverse(&data, &segments);
    let users = result.value.unwrap();

    let grouped = apply_transform(&users, ":group_by role").unwrap();
    let obj = grouped.as_object().unwrap();
    assert_eq!(obj["admin"].as_array().unwrap().len(), 1);
    assert_eq!(obj["user"].as_array().unwrap().len(), 2);
}

#[test]
fn test_transform_flatten_tags() {
    let data = json!([
        ["fiction", "classic"],
        ["fiction", "dystopian"],
        ["programming", "best-practices"]
    ]);
    let result = apply_transform(&data, ":flatten").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 6);
}

#[test]
fn test_transform_omit_fields() {
    let data = json!([
        {"name": "Alice", "age": 30, "secret": "x"},
        {"name": "Bob", "age": 25, "secret": "y"}
    ]);
    let result = apply_transform(&data, ":omit secret").unwrap();
    let arr = result.as_array().unwrap();
    assert!(!arr[0].as_object().unwrap().contains_key("secret"));
    assert!(!arr[1].as_object().unwrap().contains_key("secret"));
}

#[test]
fn test_transform_uniq() {
    let data = json!(["a", "b", "a", "c", "b"]);
    let result = apply_transform(&data, ":uniq").unwrap();
    assert_eq!(result, json!(["a", "b", "c"]));
}

#[test]
fn test_transform_chain_pick_and_sort() {
    // Simulate what the app does: query -> first transform
    let data = json!([
        {"name": "Charlie", "age": 35},
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 30}
    ]);

    // First sort
    let sorted = apply_transform(&data, ":sort name").unwrap();
    // Then pick
    let picked = apply_transform(&sorted, ":pick name").unwrap();
    assert_eq!(
        picked,
        json!([
            {"name": "Alice"},
            {"name": "Bob"},
            {"name": "Charlie"}
        ])
    );
}
