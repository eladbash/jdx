use jdx::engine::schema::{format_schema, infer_schema, SchemaType};
use serde_json::json;

#[test]
fn test_schema_simple_object() {
    let data = json!({"name": "Alice", "age": 30, "active": true});
    let schema = infer_schema(&data, 10);
    if let SchemaType::Object { fields } = schema {
        assert_eq!(fields.len(), 3);
        assert!(matches!(fields["name"].schema, SchemaType::String { .. }));
        assert!(matches!(fields["age"].schema, SchemaType::Number { .. }));
        assert!(matches!(fields["active"].schema, SchemaType::Bool));
    } else {
        panic!("expected object schema");
    }
}

#[test]
fn test_schema_array_homogeneous() {
    let data = json!([
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 35}
    ]);
    let schema = infer_schema(&data, 10);
    if let SchemaType::Array {
        items,
        len_min,
        len_max,
        ..
    } = schema
    {
        assert_eq!(len_min, 2);
        assert_eq!(len_max, 2);
        if let SchemaType::Object { fields } = *items {
            assert!(!fields["name"].optional);
            assert!(!fields["age"].optional);
        } else {
            panic!("expected object items");
        }
    } else {
        panic!("expected array schema");
    }
}

#[test]
fn test_schema_optional_fields() {
    let data = json!([
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 35, "email": "bob@test.com"},
        {"name": "Charlie"}
    ]);
    let schema = infer_schema(&data, 10);
    if let SchemaType::Array { items, .. } = schema {
        if let SchemaType::Object { fields } = *items {
            assert!(!fields["name"].optional, "name should be required");
            assert!(
                fields["age"].optional,
                "age should be optional (missing in Charlie)"
            );
            assert!(fields["email"].optional, "email should be optional");
        } else {
            panic!("expected object items");
        }
    } else {
        panic!("expected array schema");
    }
}

#[test]
fn test_schema_mixed_types() {
    let data = json!([1, "two", null, true]);
    let schema = infer_schema(&data, 10);
    if let SchemaType::Array { items, .. } = schema {
        assert!(matches!(*items, SchemaType::Union(_)));
    } else {
        panic!("expected array schema");
    }
}

#[test]
fn test_schema_nested_objects() {
    let data = json!({
        "user": {
            "profile": {
                "avatar": "url",
                "bio": "text"
            }
        }
    });
    let schema = infer_schema(&data, 10);
    if let SchemaType::Object { fields } = schema {
        if let SchemaType::Object {
            fields: user_fields,
        } = &fields["user"].schema
        {
            if let SchemaType::Object {
                fields: profile_fields,
            } = &user_fields["profile"].schema
            {
                assert!(matches!(
                    profile_fields["avatar"].schema,
                    SchemaType::String { .. }
                ));
            } else {
                panic!("expected object for profile");
            }
        } else {
            panic!("expected object for user");
        }
    } else {
        panic!("expected object schema");
    }
}

#[test]
fn test_schema_empty_array() {
    let data = json!([]);
    let schema = infer_schema(&data, 10);
    if let SchemaType::Array {
        len_min,
        len_max,
        items,
        ..
    } = schema
    {
        assert_eq!(len_min, 0);
        assert_eq!(len_max, 0);
        assert!(matches!(*items, SchemaType::Unknown));
    } else {
        panic!("expected array schema");
    }
}

#[test]
fn test_schema_number_range() {
    let data = json!([
        {"score": 10},
        {"score": 50},
        {"score": 90}
    ]);
    let schema = infer_schema(&data, 10);
    if let SchemaType::Array { items, .. } = schema {
        if let SchemaType::Object { fields } = *items {
            if let SchemaType::Number { min, max } = fields["score"].schema {
                assert_eq!(min, Some(10.0));
                assert_eq!(max, Some(90.0));
            } else {
                panic!("expected number schema for score");
            }
        } else {
            panic!("expected object items");
        }
    } else {
        panic!("expected array schema");
    }
}

#[test]
fn test_schema_fixture_nested() {
    let content = std::fs::read_to_string("fixtures/nested.json").unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    let schema = infer_schema(&data, 10);

    // Should be an object with "store", "users", "version", "active"
    if let SchemaType::Object { fields } = &schema {
        assert!(fields.contains_key("store"));
        assert!(fields.contains_key("users"));
        assert!(fields.contains_key("version"));
        assert!(fields.contains_key("active"));
    } else {
        panic!("expected object schema for nested fixture");
    }

    // Should be formattable without panic
    let formatted = format_schema(&schema, 0);
    assert!(formatted.contains("store"));
    assert!(formatted.contains("users"));
}

#[test]
fn test_format_schema_null() {
    assert_eq!(format_schema(&SchemaType::Null, 0), "null");
}

#[test]
fn test_format_schema_bool() {
    assert_eq!(format_schema(&SchemaType::Bool, 0), "bool");
}

#[test]
fn test_format_schema_unknown() {
    assert_eq!(format_schema(&SchemaType::Unknown, 0), "unknown");
}
