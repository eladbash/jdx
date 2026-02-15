use jdx::format::{detect_format, format_output, parse_input, DataFormat};
use serde_json::json;

#[test]
fn test_detect_json() {
    assert_eq!(detect_format(r#"{"key": "value"}"#), DataFormat::Json);
    assert_eq!(detect_format("[1, 2, 3]"), DataFormat::Json);
}

#[test]
fn test_detect_yaml() {
    assert_eq!(
        detect_format("---\nname: Alice\nage: 30\n"),
        DataFormat::Yaml
    );
    assert_eq!(
        detect_format("key: value\nnested:\n  a: 1\n"),
        DataFormat::Yaml
    );
}

#[test]
fn test_detect_toml() {
    assert_eq!(
        detect_format("[server]\nhost = \"localhost\"\n"),
        DataFormat::Toml
    );
}

#[test]
fn test_detect_ndjson() {
    let input = "{\"a\": 1}\n{\"b\": 2}\n{\"c\": 3}\n";
    assert_eq!(detect_format(input), DataFormat::Ndjson);
}

#[test]
fn test_parse_json_fixture() {
    let content = std::fs::read_to_string("fixtures/simple.json").unwrap();
    let result = parse_input(&content, DataFormat::Json).unwrap();
    assert_eq!(result["name"], "Alice");
    assert_eq!(result["age"], 30);
}

#[test]
fn test_parse_yaml_fixture() {
    let content = std::fs::read_to_string("fixtures/sample.yaml").unwrap();
    let result = parse_input(&content, DataFormat::Yaml).unwrap();
    assert_eq!(result["name"], "Alice");
    assert_eq!(result["hobbies"].as_array().unwrap().len(), 3);
}

#[test]
fn test_parse_toml_fixture() {
    let content = std::fs::read_to_string("fixtures/sample.toml").unwrap();
    let result = parse_input(&content, DataFormat::Toml).unwrap();
    assert_eq!(result["server"]["host"], "localhost");
    assert_eq!(result["server"]["port"], 8080);
}

#[test]
fn test_parse_csv_fixture() {
    let content = std::fs::read_to_string("fixtures/sample.csv").unwrap();
    let result = parse_input(&content, DataFormat::Csv).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[1]["name"], "Bob");
}

#[test]
fn test_parse_ndjson() {
    let input = "{\"a\": 1}\n{\"b\": 2}\n";
    let result = parse_input(input, DataFormat::Ndjson).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["a"], 1);
    assert_eq!(arr[1]["b"], 2);
}

#[test]
fn test_parse_malformed_json() {
    let content = std::fs::read_to_string("fixtures/malformed.json").unwrap();
    let result = parse_input(&content, DataFormat::Json);
    assert!(result.is_err());
}

#[test]
fn test_format_output_json() {
    let data = json!({"name": "Alice"});
    let output = format_output(&data, DataFormat::Json).unwrap();
    assert!(output.contains("\"name\""));
    assert!(output.contains("\"Alice\""));
}

#[test]
fn test_format_output_yaml() {
    let data = json!({"name": "Alice", "age": 30});
    let output = format_output(&data, DataFormat::Yaml).unwrap();
    assert!(output.contains("name:"));
    assert!(output.contains("Alice"));
}

#[test]
fn test_format_output_ndjson() {
    let data = json!([{"a": 1}, {"b": 2}]);
    let output = format_output(&data, DataFormat::Ndjson).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_json_roundtrip() {
    let data = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
    let output = format_output(&data, DataFormat::Json).unwrap();
    let parsed = parse_input(&output, DataFormat::Json).unwrap();
    assert_eq!(data, parsed);
}

#[test]
fn test_format_from_str_name() {
    assert_eq!(DataFormat::from_str_name("json").unwrap(), DataFormat::Json);
    assert_eq!(DataFormat::from_str_name("yaml").unwrap(), DataFormat::Yaml);
    assert_eq!(DataFormat::from_str_name("yml").unwrap(), DataFormat::Yaml);
    assert_eq!(DataFormat::from_str_name("toml").unwrap(), DataFormat::Toml);
    assert_eq!(DataFormat::from_str_name("csv").unwrap(), DataFormat::Csv);
    assert_eq!(
        DataFormat::from_str_name("ndjson").unwrap(),
        DataFormat::Ndjson
    );
    assert_eq!(
        DataFormat::from_str_name("jsonl").unwrap(),
        DataFormat::Ndjson
    );
    assert!(DataFormat::from_str_name("unknown").is_err());
}
