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

// --- New Phase 3 transforms: :reverse, :upper, :lower, :split, :join ---

#[test]
fn test_reverse_array() {
    let data = json!([1, 2, 3, 4, 5]);
    let result = apply_transform(&data, ":reverse").unwrap();
    assert_eq!(result, json!([5, 4, 3, 2, 1]));
}

#[test]
fn test_reverse_string() {
    let data = json!("hello");
    let result = apply_transform(&data, ":reverse").unwrap();
    assert_eq!(result, json!("olleh"));
}

#[test]
fn test_reverse_empty_array() {
    let data = json!([]);
    let result = apply_transform(&data, ":reverse").unwrap();
    assert_eq!(result, json!([]));
}

#[test]
fn test_reverse_on_non_array_or_string() {
    let data = json!(42);
    assert!(apply_transform(&data, ":reverse").is_err());
}

#[test]
fn test_upper_string() {
    let data = json!("hello world");
    let result = apply_transform(&data, ":upper").unwrap();
    assert_eq!(result, json!("HELLO WORLD"));
}

#[test]
fn test_upper_array_of_strings() {
    let data = json!(["alice", "bob", "carol"]);
    let result = apply_transform(&data, ":upper").unwrap();
    assert_eq!(result, json!(["ALICE", "BOB", "CAROL"]));
}

#[test]
fn test_upper_mixed_array() {
    // Non-strings should pass through unchanged
    let data = json!(["hello", 42, true]);
    let result = apply_transform(&data, ":upper").unwrap();
    assert_eq!(result, json!(["HELLO", 42, true]));
}

#[test]
fn test_upper_on_number() {
    let data = json!(42);
    assert!(apply_transform(&data, ":upper").is_err());
}

#[test]
fn test_lower_string() {
    let data = json!("HELLO WORLD");
    let result = apply_transform(&data, ":lower").unwrap();
    assert_eq!(result, json!("hello world"));
}

#[test]
fn test_lower_array_of_strings() {
    let data = json!(["ALICE", "BOB"]);
    let result = apply_transform(&data, ":lower").unwrap();
    assert_eq!(result, json!(["alice", "bob"]));
}

#[test]
fn test_split_string() {
    let data = json!("a,b,c");
    let result = apply_transform(&data, ":split ,").unwrap();
    assert_eq!(result, json!(["a", "b", "c"]));
}

#[test]
fn test_split_by_dash() {
    let data = json!("2024-01-15");
    let result = apply_transform(&data, ":split -").unwrap();
    assert_eq!(result, json!(["2024", "01", "15"]));
}

#[test]
fn test_split_no_delimiter() {
    let data = json!("hello");
    assert!(apply_transform(&data, ":split").is_err());
}

#[test]
fn test_split_on_non_string() {
    let data = json!(42);
    assert!(apply_transform(&data, ":split ,").is_err());
}

#[test]
fn test_join_array() {
    let data = json!(["a", "b", "c"]);
    let result = apply_transform(&data, ":join ,").unwrap();
    assert_eq!(result, json!("a,b,c"));
}

#[test]
fn test_join_with_dash() {
    let data = json!(["hello", "world"]);
    let result = apply_transform(&data, ":join -").unwrap();
    assert_eq!(result, json!("hello-world"));
}

#[test]
fn test_join_default_separator() {
    let data = json!(["x", "y", "z"]);
    let result = apply_transform(&data, ":join").unwrap();
    assert_eq!(result, json!("x,y,z"));
}

#[test]
fn test_join_mixed_types() {
    let data = json!(["hello", 42, true]);
    let result = apply_transform(&data, ":join -").unwrap();
    assert_eq!(result, json!("hello-42-true"));
}

#[test]
fn test_join_on_non_array() {
    let data = json!("hello");
    assert!(apply_transform(&data, ":join ,").is_err());
}

#[test]
fn test_chain_split_then_reverse_then_join() {
    let data = json!("a-b-c");
    let result = apply_transform(&data, ":split - :reverse :join -").unwrap();
    assert_eq!(result, json!("c-b-a"));
}

#[test]
fn test_chain_upper_then_split() {
    let data = json!("hello,world");
    let result = apply_transform(&data, ":upper :split ,").unwrap();
    assert_eq!(result, json!(["HELLO", "WORLD"]));
}

// --- Compound filter tests (AND/OR) ---

#[test]
fn test_compound_filter_and_inline() {
    let data = json!({
        "items": [
            {"name": "A", "price": 5, "stock": 10},
            {"name": "B", "price": 15, "stock": 20},
            {"name": "C", "price": 8, "stock": 0},
            {"name": "D", "price": 3, "stock": 5}
        ]
    });
    let segments = parse(".items[price < 10 && stock > 0]").unwrap();
    let result = traverse(&data, &segments);
    let arr = result.value.unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "A");
    assert_eq!(arr[1]["name"], "D");
}

#[test]
fn test_compound_filter_or_inline() {
    let data = json!({
        "users": [
            {"name": "Alice", "role": "admin"},
            {"name": "Bob", "role": "user"},
            {"name": "Carol", "role": "moderator"},
            {"name": "Dave", "role": "user"}
        ]
    });
    let segments = parse(".users[role == \"admin\" || role == \"moderator\"]").unwrap();
    let result = traverse(&data, &segments);
    let arr = result.value.unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[1]["name"], "Carol");
}

#[test]
fn test_compound_filter_and_with_transform() {
    let data = json!([
        {"name": "A", "price": 5, "category": "food"},
        {"name": "B", "price": 15, "category": "food"},
        {"name": "C", "price": 8, "category": "drink"},
        {"name": "D", "price": 3, "category": "food"}
    ]);
    let result = apply_transform(
        &data,
        ":filter price < 10 && category == \"food\" :pick name,price",
    )
    .unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], json!({"name": "A", "price": 5}));
    assert_eq!(arr[1], json!({"name": "D", "price": 3}));
}

#[test]
fn test_compound_filter_or_with_transform() {
    let data = json!([
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 35},
        {"name": "Carol", "age": 45},
        {"name": "Dave", "age": 15}
    ]);
    let result = apply_transform(&data, ":filter age < 20 || age > 40 :pick name").unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], json!({"name": "Carol"}));
    assert_eq!(arr[1], json!({"name": "Dave"}));
}

#[test]
fn test_compound_filter_mixed_and_or() {
    // OR has lower precedence than AND: a || b && c => a || (b && c)
    let data = json!([
        {"name": "A", "price": 100, "featured": true},
        {"name": "B", "price": 5, "featured": false},
        {"name": "C", "price": 8, "featured": false},
        {"name": "D", "price": 3, "featured": true}
    ]);
    // featured == true || price < 10 && price > 4
    // Matches: A (featured=true), B (price 5 in range), C (price 8 in range), D (featured=true)
    let segments = parse(".[featured == true || price < 10 && price > 4]").unwrap();
    let result = traverse(&data, &segments);
    let arr = result.value.unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0]["name"], "A");
    assert_eq!(arr[1]["name"], "B");
    assert_eq!(arr[2]["name"], "C");
    assert_eq!(arr[3]["name"], "D");
}
