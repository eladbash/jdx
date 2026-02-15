pub mod csv_format;
pub mod json_fmt;
pub mod toml_format;
pub mod yaml_format;

use anyhow::{bail, Result};
use serde_json::Value;

/// Supported input/output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Json,
    Yaml,
    Toml,
    Csv,
    Ndjson,
}

impl DataFormat {
    /// Parse a format name from a CLI argument.
    pub fn from_str_name(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            "csv" => Ok(Self::Csv),
            "ndjson" | "jsonl" => Ok(Self::Ndjson),
            _ => bail!("unknown format: {s}"),
        }
    }
}

/// Auto-detect the format of input data by inspecting content.
pub fn detect_format(content: &str) -> DataFormat {
    let trimmed = content.trim();

    // NDJSON: multiple lines each starting with { (check before JSON)
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() > 1 && lines.iter().take(5).all(|l| l.trim().starts_with('{')) {
        return DataFormat::Ndjson;
    }

    // JSON starts with { or [, but only if it looks like a single JSON value
    // (TOML sections also start with [ but contain `=` signs)
    if trimmed.starts_with('{') {
        return DataFormat::Json;
    }
    if trimmed.starts_with('[') && !trimmed.contains(" = ") {
        return DataFormat::Json;
    }

    // TOML: contains `key = value` or `[section]` with `=`
    if trimmed.contains(" = ") {
        return DataFormat::Toml;
    }

    // YAML: contains `---` or `key:` patterns
    if trimmed.starts_with("---") || trimmed.contains(": ") {
        return DataFormat::Yaml;
    }

    // CSV: contains commas and multiple lines
    if trimmed.contains(',') && lines.len() > 1 {
        return DataFormat::Csv;
    }

    // Default to JSON
    DataFormat::Json
}

/// Parse input content to a JSON Value based on format.
pub fn parse_input(content: &str, format: DataFormat) -> Result<Value> {
    match format {
        DataFormat::Json => json_fmt::parse(content),
        DataFormat::Yaml => yaml_format::parse(content),
        DataFormat::Toml => toml_format::parse(content),
        DataFormat::Csv => csv_format::parse(content),
        DataFormat::Ndjson => {
            let values: Result<Vec<Value>> = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str(l).map_err(Into::into))
                .collect();
            Ok(Value::Array(values?))
        }
    }
}

/// Serialize a JSON Value to a specific output format.
pub fn format_output(value: &Value, format: DataFormat) -> Result<String> {
    match format {
        DataFormat::Json => Ok(serde_json::to_string_pretty(value)?),
        DataFormat::Yaml => Ok(serde_yaml::to_string(value)?),
        DataFormat::Toml => toml_format::serialize(value),
        DataFormat::Csv => csv_format::serialize(value),
        DataFormat::Ndjson => {
            if let Value::Array(arr) = value {
                let lines: Result<Vec<String>> = arr
                    .iter()
                    .map(|v| serde_json::to_string(v).map_err(Into::into))
                    .collect();
                Ok(lines?.join("\n"))
            } else {
                Ok(serde_json::to_string(value)?)
            }
        }
    }
}
