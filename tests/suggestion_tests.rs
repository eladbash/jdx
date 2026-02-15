use jdx::engine::json::{get_available_keys, traverse};
use jdx::engine::query::{get_last_keyword, parse};
use jdx::engine::suggestion::Suggester;

/// End-to-end: query + suggestion on fixture JSON files.

#[test]
fn test_suggestion_on_nested_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suggester = Suggester::new();

    // At the root level, should suggest "store", "users", "version", "active"
    let keys = get_available_keys(&data);
    let candidates = suggester.get_candidates(&keys, "");
    assert_eq!(candidates.len(), 4);

    // Typing "st" should match "store"
    let candidates = suggester.get_candidates(&keys, "st");
    assert!(candidates.iter().any(|c| c.text == "store"));

    // Typing "us" should match "users"
    let candidates = suggester.get_candidates(&keys, "us");
    assert!(candidates.iter().any(|c| c.text == "users"));
}

#[test]
fn test_suggestion_after_navigation() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suggester = Suggester::new();

    // Navigate to .store, then suggest keys at that level
    let segments = parse(".store").unwrap();
    let result = traverse(&data, &segments);
    let store_val = result.value.unwrap();
    let keys = get_available_keys(&store_val);

    assert!(keys.contains(&"name".to_string()));
    assert!(keys.contains(&"books".to_string()));
    assert!(keys.contains(&"location".to_string()));

    // Typing "bo" should match "books"
    let candidates = suggester.get_candidates(&keys, "bo");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].text, "books");
}

#[test]
fn test_completion_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suggester = Suggester::new();

    // At root, typing "ver" should complete to "version"
    let keys = get_available_keys(&data);
    let completion = suggester.get_completion(&keys, "ver");
    assert_eq!(completion, Some(("sion".into(), "version".into())));
}

#[test]
fn test_suggestion_on_array() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suggester = Suggester::new();

    // Navigate to .users (an array)
    let segments = parse(".users").unwrap();
    let result = traverse(&data, &segments);
    let users_val = result.value.unwrap();
    let keys = get_available_keys(&users_val);

    // Should suggest [0], [1], [2]
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], "[0]");

    // Fuzzy matching on array indices
    let candidates = suggester.get_candidates(&keys, "[1");
    assert!(candidates.iter().any(|c| c.text == "[1]"));
}

#[test]
fn test_get_last_keyword_for_suggestions() {
    assert_eq!(get_last_keyword(".store.bo"), "bo");
    assert_eq!(get_last_keyword(".store."), "");
    assert_eq!(get_last_keyword(".store.books[0]."), "");
    assert_eq!(get_last_keyword(".store.books[0].ti"), "ti");
}

#[test]
fn test_suggestion_on_large_array() {
    let content = std::fs::read_to_string("fixtures/large_array.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suggester = Suggester::new();

    // The large array has 1000 elements
    let keys = get_available_keys(&data);
    assert_eq!(keys.len(), 1000);

    // Navigate into first element and check keys
    let segments = parse(".[0]").unwrap();
    let result = traverse(&data, &segments);
    let first = result.value.unwrap();
    let item_keys = get_available_keys(&first);
    assert!(item_keys.contains(&"id".to_string()));
    assert!(item_keys.contains(&"name".to_string()));
    assert!(item_keys.contains(&"score".to_string()));
    assert!(item_keys.contains(&"active".to_string()));

    // Fuzzy match on the first element's keys
    let candidates = suggester.get_candidates(&item_keys, "sc");
    assert!(candidates.iter().any(|c| c.text == "score"));
}
