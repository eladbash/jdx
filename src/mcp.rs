//! MCP (Model Context Protocol) server for jdx.
//!
//! Exposes jdx's format conversion and query capabilities as MCP tools
//! over JSON-RPC 2.0 on stdio. Start with `jdx --mcp`.

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::engine;
use crate::format::{detect_format, format_output, parse_input, DataFormat};

/// Run the MCP server, reading JSON-RPC requests from stdin and writing
/// responses to stdout.
pub fn run_mcp_server() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read from stdin")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json_rpc_error(Value::Null, -32700, &format!("Parse error: {e}"));
                write_response(&mut stdout, &err_resp)?;
                continue;
            }
        };

        // Notifications (no "id") don't require a response
        if request.get("id").is_none() {
            continue;
        }

        let id = request["id"].clone();
        let method = request["method"].as_str().unwrap_or("");

        let response = match method {
            "initialize" => handle_initialize(id),
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, &request["params"]),
            "ping" => json_rpc_ok(id, json!({})),
            _ => json_rpc_error(id, -32601, &format!("Method not found: {method}")),
        };

        write_response(&mut stdout, &response)?;
    }

    Ok(())
}

fn write_response(out: &mut impl Write, response: &Value) -> Result<()> {
    let s = serde_json::to_string(response)?;
    writeln!(out, "{s}")?;
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON-RPC helpers
// ---------------------------------------------------------------------------

fn json_rpc_ok(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn json_rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

// ---------------------------------------------------------------------------
// MCP handlers
// ---------------------------------------------------------------------------

fn handle_initialize(id: Value) -> Value {
    json_rpc_ok(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "jdx",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }),
    )
}

fn handle_tools_list(id: Value) -> Value {
    json_rpc_ok(
        id,
        json!({
            "tools": [
                {
                    "name": "convert",
                    "description": "Convert structured data between formats. Supported formats: json, yaml, toml, csv, ndjson. Input format is auto-detected if not specified.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "The input data to convert (as a string)"
                            },
                            "input_format": {
                                "type": "string",
                                "description": "Input format: json, yaml, toml, csv, ndjson. Auto-detected if omitted.",
                                "enum": ["json", "yaml", "toml", "csv", "ndjson"]
                            },
                            "output_format": {
                                "type": "string",
                                "description": "Output format: json, yaml, toml, csv, ndjson",
                                "enum": ["json", "yaml", "toml", "csv", "ndjson"]
                            }
                        },
                        "required": ["input", "output_format"]
                    }
                },
                {
                    "name": "query",
                    "description": "Query and transform structured data using jdx dot-notation. Supports path traversal (e.g. '.users[0].name'), filter predicates (e.g. '.items[price < 10]'), and transform commands (:keys, :values, :count, :flatten, :pick, :omit, :sort, :uniq, :group_by, :filter, :sum, :avg, :min, :max). Input format is auto-detected.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "The input data to query (as a string)"
                            },
                            "query": {
                                "type": "string",
                                "description": "jdx query expression (e.g. '.users[0].name', '.items[price < 10] :pick name,price :sort price')"
                            },
                            "input_format": {
                                "type": "string",
                                "description": "Input format: json, yaml, toml, csv, ndjson. Auto-detected if omitted.",
                                "enum": ["json", "yaml", "toml", "csv", "ndjson"]
                            },
                            "output_format": {
                                "type": "string",
                                "description": "Output format for the result: json, yaml, toml, csv, ndjson. Defaults to json.",
                                "enum": ["json", "yaml", "toml", "csv", "ndjson"]
                            }
                        },
                        "required": ["input", "query"]
                    }
                }
            ]
        }),
    )
}

fn handle_tools_call(id: Value, params: &Value) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let arguments = &params["arguments"];

    let result = match tool_name {
        "convert" => tool_convert(arguments),
        "query" => tool_query(arguments),
        _ => Err(anyhow::anyhow!("Unknown tool: {tool_name}")),
    };

    match result {
        Ok(text) => json_rpc_ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": text,
                    }
                ]
            }),
        ),
        Err(e) => json_rpc_ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Error: {e}"),
                    }
                ],
                "isError": true,
            }),
        ),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

/// Resolve a DataFormat from an optional string parameter, falling back to
/// auto-detection from the content.
fn resolve_input_format(args: &Value, content: &str) -> Result<DataFormat> {
    match args.get("input_format").and_then(|v| v.as_str()) {
        Some(fmt) => DataFormat::from_str_name(fmt),
        None => Ok(detect_format(content)),
    }
}

fn resolve_output_format(args: &Value, default: DataFormat) -> Result<DataFormat> {
    match args.get("output_format").and_then(|v| v.as_str()) {
        Some(fmt) => DataFormat::from_str_name(fmt),
        None => Ok(default),
    }
}

fn tool_convert(args: &Value) -> Result<String> {
    let input = args["input"]
        .as_str()
        .context("missing required parameter: input")?;
    let in_fmt = resolve_input_format(args, input)?;
    let out_fmt = resolve_output_format(args, DataFormat::Json)?;

    let data = parse_input(input, in_fmt).context("failed to parse input data")?;
    format_output(&data, out_fmt).context("failed to format output")
}

fn tool_query(args: &Value) -> Result<String> {
    let input = args["input"]
        .as_str()
        .context("missing required parameter: input")?;
    let query_str = args["query"]
        .as_str()
        .context("missing required parameter: query")?;

    let in_fmt = resolve_input_format(args, input)?;
    let out_fmt = resolve_output_format(args, DataFormat::Json)?;

    let data = parse_input(input, in_fmt).context("failed to parse input data")?;

    // Split query from transform commands (separated by ` :`)
    let (path_part, transform_part) = split_query_and_transforms(query_str);

    // Parse and traverse the path
    let segments = engine::query::parse(path_part)
        .map_err(|e| anyhow::anyhow!("invalid query: {e}"))?;
    let traversal = engine::json::traverse(&data, &segments);

    let value = traversal
        .value
        .context(format!("no match for query: {path_part}"))?;

    // Apply transform commands if present
    let result = if let Some(transforms) = transform_part {
        engine::transform::apply_transform(&value, transforms)
            .context("transform failed")?
    } else {
        value
    };

    format_output(&result, out_fmt).context("failed to format output")
}

/// Split a full query string into the path portion and the optional transform
/// chain. For example:
///   `.users :pick name,age :sort age`
/// becomes `(".users", Some(":pick name,age :sort age"))`.
fn split_query_and_transforms(query: &str) -> (&str, Option<&str>) {
    // Look for the first ` :` that signals the start of transforms
    if let Some(idx) = query.find(" :") {
        let path = query[..idx].trim();
        let transforms = query[idx + 1..].trim(); // keep the leading ':'
        (path, Some(transforms))
    } else {
        (query, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_query_no_transforms() {
        let (path, transforms) = split_query_and_transforms(".users[0].name");
        assert_eq!(path, ".users[0].name");
        assert!(transforms.is_none());
    }

    #[test]
    fn test_split_query_with_transforms() {
        let (path, transforms) =
            split_query_and_transforms(".users :pick name,age :sort age");
        assert_eq!(path, ".users");
        assert_eq!(transforms.unwrap(), ":pick name,age :sort age");
    }

    #[test]
    fn test_tool_convert_json_to_yaml() {
        let args = serde_json::json!({
            "input": "{\"name\": \"Alice\", \"age\": 30}",
            "output_format": "yaml"
        });
        let result = tool_convert(&args).unwrap();
        assert!(result.contains("name:"));
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_tool_convert_yaml_to_json() {
        let args = serde_json::json!({
            "input": "name: Alice\nage: 30",
            "output_format": "json"
        });
        let result = tool_convert(&args).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
    }

    #[test]
    fn test_tool_query_simple() {
        let args = serde_json::json!({
            "input": "{\"users\": [{\"name\": \"Alice\"}, {\"name\": \"Bob\"}]}",
            "query": ".users[0].name"
        });
        let result = tool_query(&args).unwrap();
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_tool_query_with_transform() {
        let args = serde_json::json!({
            "input": "[{\"name\": \"Alice\", \"age\": 30}, {\"name\": \"Bob\", \"age\": 25}]",
            "query": ". :sort age :pick name"
        });
        let result = tool_query(&args).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr[0]["name"], "Bob");
        assert_eq!(arr[1]["name"], "Alice");
    }

    #[test]
    fn test_tool_convert_missing_input() {
        let args = serde_json::json!({
            "output_format": "yaml"
        });
        let result = tool_convert(&args);
        assert!(result.is_err());
    }
}
