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

#[test]
fn test_transform_chained_pick_then_sort() {
    // Test single-call chaining: ":pick name,age :sort age"
    let data = json!([
        {"name": "Charlie", "age": 35, "city": "Denver"},
        {"name": "Alice", "age": 25, "city": "Portland"},
        {"name": "Bob", "age": 30, "city": "Seattle"}
    ]);

    let result = apply_transform(&data, ":pick name,age :sort age").unwrap();
    assert_eq!(
        result,
        json!([
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 30},
            {"name": "Charlie", "age": 35}
        ])
    );
}

#[test]
fn test_transform_chained_sort_then_pick() {
    // Reverse order: sort first, then pick a single field
    let data = json!([
        {"name": "Charlie", "age": 35},
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 30}
    ]);

    let result = apply_transform(&data, ":sort age :pick name").unwrap();
    assert_eq!(
        result,
        json!([
            {"name": "Alice"},
            {"name": "Bob"},
            {"name": "Charlie"}
        ])
    );
}

#[test]
fn test_transform_chained_three_steps() {
    // Three transforms: pick, sort, then count
    let data = json!([
        {"name": "Charlie", "age": 35, "city": "Denver"},
        {"name": "Alice", "age": 25, "city": "Portland"},
        {"name": "Bob", "age": 30, "city": "Seattle"}
    ]);

    let result = apply_transform(&data, ":pick name,age :sort age :count").unwrap();
    assert_eq!(result, json!(3));
}

// --- Filter integration tests ---

#[test]
fn test_filter_books_by_price_inline() {
    // Test inline filter predicate in query: .store.books[price < 15]
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books[price < 15]").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();
    let arr = books.as_array().unwrap();
    assert_eq!(arr.len(), 2); // Great Gatsby (10.99) and 1984 (8.99)
    assert!(arr.iter().all(|b| b["price"].as_f64().unwrap() < 15.0));
}

#[test]
fn test_filter_users_by_role_inline() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".users[role == \"user\"]").unwrap();
    let result = traverse(&data, &segments);
    let users = result.value.unwrap();
    let arr = users.as_array().unwrap();
    assert_eq!(arr.len(), 2); // Bob and Charlie
}

#[test]
fn test_filter_transform_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let filtered = apply_transform(&books, ":filter price < 15").unwrap();
    let arr = filtered.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_filter_transform_chained_with_pick_and_sort() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let result =
        apply_transform(&books, ":filter price < 15 :pick title,price :sort price").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // Sorted by price ascending: 1984 (8.99), then Great Gatsby (10.99)
    assert_eq!(arr[0]["title"], "1984");
    assert_eq!(arr[1]["title"], "The Great Gatsby");
}

#[test]
fn test_filter_inline_with_transform() {
    // Test combining inline filter in path with a transform command
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books[price < 15]").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let picked = apply_transform(&books, ":pick title,price").unwrap();
    let arr = picked.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // Each entry should only have title and price
    for item in arr {
        let obj = item.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("title"));
        assert!(obj.contains_key("price"));
    }
}

#[test]
fn test_filter_count() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books[price > 10]").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let count = apply_transform(&books, ":count").unwrap();
    assert_eq!(count, json!(2)); // Great Gatsby (10.99) and Clean Code (32.99)
}

// --- Aggregate transform integration tests ---

#[test]
fn test_sum_book_prices_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let total = apply_transform(&books, ":sum price").unwrap();
    // 10.99 + 8.99 + 32.99 = 52.97
    assert_eq!(total.as_f64().unwrap(), 52.97);
}

#[test]
fn test_avg_book_prices_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let avg = apply_transform(&books, ":avg price").unwrap();
    let avg_val = avg.as_f64().unwrap();
    // (10.99 + 8.99 + 32.99) / 3 ≈ 17.6567
    assert!((avg_val - 17.6567).abs() < 0.01);
}

#[test]
fn test_min_max_book_prices_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    let min = apply_transform(&books, ":min price").unwrap();
    assert_eq!(min.as_f64().unwrap(), 8.99);

    let max = apply_transform(&books, ":max price").unwrap();
    assert_eq!(max.as_f64().unwrap(), 32.99);
}

#[test]
fn test_filter_then_sum_on_fixture() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();

    let segments = parse(".store.books").unwrap();
    let result = traverse(&data, &segments);
    let books = result.value.unwrap();

    // Sum prices of books cheaper than $15
    let total = apply_transform(&books, ":filter price < 15 :sum price").unwrap();
    // 10.99 + 8.99 = 19.98
    assert_eq!(total.as_f64().unwrap(), 19.98);
}
