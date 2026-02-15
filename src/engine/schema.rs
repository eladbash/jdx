use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Represents the inferred schema of a JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaType {
    Null,
    Bool,
    Number {
        min: Option<f64>,
        max: Option<f64>,
    },
    String {
        sample: Option<String>,
    },
    Array {
        len_min: usize,
        len_max: usize,
        items: Box<SchemaType>,
    },
    Object {
        fields: BTreeMap<String, FieldSchema>,
    },
    /// Multiple possible types (e.g., `string | null`)
    Union(BTreeSet<String>),
    /// Unknown / empty
    Unknown,
}

/// A field in an object schema, with optionality info.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    pub schema: SchemaType,
    pub optional: bool,
    pub count: usize,
}

/// Infer the schema of a JSON value.
///
/// For arrays, samples up to `max_samples` elements to build a merged schema.
pub fn infer_schema(value: &Value, max_samples: usize) -> SchemaType {
    match value {
        Value::Null => SchemaType::Null,
        Value::Bool(_) => SchemaType::Bool,
        Value::Number(n) => SchemaType::Number {
            min: n.as_f64(),
            max: n.as_f64(),
        },
        Value::String(s) => SchemaType::String {
            sample: Some(s.chars().take(30).collect()),
        },
        Value::Array(arr) => {
            if arr.is_empty() {
                return SchemaType::Array {
                    len_min: 0,
                    len_max: 0,
                    items: Box::new(SchemaType::Unknown),
                };
            }

            let samples = arr.iter().take(max_samples);
            let mut merged: Option<SchemaType> = None;

            for item in samples {
                let item_schema = infer_schema(item, max_samples);
                merged = Some(match merged {
                    None => item_schema,
                    Some(existing) => merge_schemas(existing, item_schema),
                });
            }

            SchemaType::Array {
                len_min: arr.len(),
                len_max: arr.len(),
                items: Box::new(merged.unwrap_or(SchemaType::Unknown)),
            }
        }
        Value::Object(map) => {
            let mut fields = BTreeMap::new();
            for (key, val) in map {
                fields.insert(
                    key.clone(),
                    FieldSchema {
                        schema: infer_schema(val, max_samples),
                        optional: false,
                        count: 1,
                    },
                );
            }
            SchemaType::Object { fields }
        }
    }
}

/// Merge two schemas together (used when sampling array elements).
fn merge_schemas(a: SchemaType, b: SchemaType) -> SchemaType {
    match (a, b) {
        // Same simple types
        (SchemaType::Null, SchemaType::Null) => SchemaType::Null,
        (SchemaType::Bool, SchemaType::Bool) => SchemaType::Bool,
        (
            SchemaType::Number {
                min: a_min,
                max: a_max,
            },
            SchemaType::Number {
                min: b_min,
                max: b_max,
            },
        ) => SchemaType::Number {
            min: match (a_min, b_min) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            },
            max: match (a_max, b_max) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            },
        },
        (SchemaType::String { .. }, SchemaType::String { sample }) => SchemaType::String { sample },
        // Merge objects
        (
            SchemaType::Object {
                fields: mut a_fields,
            },
            SchemaType::Object { fields: b_fields },
        ) => {
            let a_keys: BTreeSet<String> = a_fields.keys().cloned().collect();
            let b_keys: BTreeSet<String> = b_fields.keys().cloned().collect();

            // Mark fields that don't appear in both as optional
            for key in a_keys.difference(&b_keys) {
                if let Some(field) = a_fields.get_mut(key) {
                    field.optional = true;
                }
            }

            for (key, b_field) in b_fields {
                if let Some(a_field) = a_fields.get_mut(&key) {
                    a_field.schema = merge_schemas(a_field.schema.clone(), b_field.schema);
                    a_field.count += 1;
                } else {
                    a_fields.insert(
                        key,
                        FieldSchema {
                            schema: b_field.schema,
                            optional: true,
                            count: 1,
                        },
                    );
                }
            }

            SchemaType::Object { fields: a_fields }
        }
        // Merge arrays
        (
            SchemaType::Array {
                len_min: a_min,
                len_max: a_max,
                items: a_items,
            },
            SchemaType::Array {
                len_min: b_min,
                len_max: b_max,
                items: b_items,
            },
        ) => SchemaType::Array {
            len_min: a_min.min(b_min),
            len_max: a_max.max(b_max),
            items: Box::new(merge_schemas(*a_items, *b_items)),
        },
        // Different types → union
        (a, b) => {
            let mut types = BTreeSet::new();
            types.insert(type_name(&a));
            types.insert(type_name(&b));
            SchemaType::Union(types)
        }
    }
}

fn type_name(schema: &SchemaType) -> String {
    match schema {
        SchemaType::Null => "null".into(),
        SchemaType::Bool => "bool".into(),
        SchemaType::Number { .. } => "number".into(),
        SchemaType::String { .. } => "string".into(),
        SchemaType::Array { .. } => "array".into(),
        SchemaType::Object { .. } => "object".into(),
        SchemaType::Union(types) => types.iter().cloned().collect::<Vec<_>>().join(" | "),
        SchemaType::Unknown => "unknown".into(),
    }
}

/// Format a schema as a human-readable string.
pub fn format_schema(schema: &SchemaType, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match schema {
        SchemaType::Null => "null".into(),
        SchemaType::Bool => "bool".into(),
        SchemaType::Number { min, max } => match (min, max) {
            (Some(mn), Some(mx)) if (mn - mx).abs() < f64::EPSILON => format!("number  # {mn}"),
            (Some(mn), Some(mx)) => format!("number  # {mn}..{mx}"),
            _ => "number".into(),
        },
        SchemaType::String { sample } => match sample {
            Some(s) => format!("string  # \"{s}\""),
            None => "string".into(),
        },
        SchemaType::Array {
            len_min,
            len_max,
            items,
        } => {
            let len_info = if len_min == len_max {
                format!("{len_min}")
            } else {
                format!("{len_min}..{len_max}")
            };
            let inner = format_schema(items, indent + 1);
            format!("[{inner}]  # array of {len_info}")
        }
        SchemaType::Object { fields } => {
            if fields.is_empty() {
                return "{}".into();
            }
            let mut lines = vec!["{".to_string()];
            for (key, field) in fields {
                let opt = if field.optional { "?" } else { "" };
                let val = format_schema(&field.schema, indent + 1);
                lines.push(format!("{pad}  {key}{opt}: {val},"));
            }
            lines.push(format!("{pad}}}"));
            lines.join("\n")
        }
        SchemaType::Union(types) => types.iter().cloned().collect::<Vec<_>>().join(" | "),
        SchemaType::Unknown => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_infer_simple_object() {
        let data = json!({"name": "Alice", "age": 30});
        let schema = infer_schema(&data, 10);
        if let SchemaType::Object { fields } = schema {
            assert!(fields.contains_key("name"));
            assert!(fields.contains_key("age"));
            assert!(matches!(fields["name"].schema, SchemaType::String { .. }));
            assert!(matches!(fields["age"].schema, SchemaType::Number { .. }));
        } else {
            panic!("expected object schema");
        }
    }

    #[test]
    fn test_infer_array_of_objects() {
        let data = json!([
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 35, "email": "bob@test.com"}
        ]);
        let schema = infer_schema(&data, 10);
        if let SchemaType::Array { items, .. } = schema {
            if let SchemaType::Object { fields } = *items {
                assert!(!fields["name"].optional);
                assert!(!fields["age"].optional);
                assert!(fields["email"].optional);
            } else {
                panic!("expected object items");
            }
        } else {
            panic!("expected array schema");
        }
    }

    #[test]
    fn test_infer_null() {
        assert_eq!(infer_schema(&json!(null), 10), SchemaType::Null);
    }

    #[test]
    fn test_infer_bool() {
        assert_eq!(infer_schema(&json!(true), 10), SchemaType::Bool);
    }

    #[test]
    fn test_infer_empty_array() {
        let schema = infer_schema(&json!([]), 10);
        if let SchemaType::Array {
            len_min, len_max, ..
        } = schema
        {
            assert_eq!(len_min, 0);
            assert_eq!(len_max, 0);
        } else {
            panic!("expected array schema");
        }
    }

    #[test]
    fn test_infer_mixed_types_in_array() {
        let data = json!([1, "two", null]);
        let schema = infer_schema(&data, 10);
        if let SchemaType::Array { items, .. } = schema {
            assert!(matches!(*items, SchemaType::Union(_)));
        } else {
            panic!("expected array schema");
        }
    }

    #[test]
    fn test_format_schema_basic() {
        let schema = SchemaType::Object {
            fields: BTreeMap::from([
                (
                    "name".into(),
                    FieldSchema {
                        schema: SchemaType::String {
                            sample: Some("Alice".into()),
                        },
                        optional: false,
                        count: 1,
                    },
                ),
                (
                    "age".into(),
                    FieldSchema {
                        schema: SchemaType::Number {
                            min: Some(30.0),
                            max: Some(30.0),
                        },
                        optional: false,
                        count: 1,
                    },
                ),
            ]),
        };
        let output = format_schema(&schema, 0);
        assert!(output.contains("name"));
        assert!(output.contains("age"));
        assert!(output.contains("string"));
        assert!(output.contains("number"));
    }
}
