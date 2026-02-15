use thiserror::Error;

/// Comparison operator for filter predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareOp {
    Eq, // ==
    Ne, // !=
    Lt, // <
    Gt, // >
    Le, // <=
    Ge, // >=
}

/// A value literal in a filter predicate.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

// Manual Eq impl because f64 doesn't implement Eq, but we need it for PathSegment.
impl Eq for FilterValue {}

/// A filter predicate: `field op value` (e.g., `price < 10`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    pub field: String,
    pub op: CompareOp,
    pub value: FilterValue,
}

/// A single segment of a JSON path query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// Object key access: `.foo`
    Key(String),
    /// Array index access: `[0]`, `[-1]`
    Index(i64),
    /// Array slice: `[0:5]`, `[:3]`, `[2:]`
    Slice(Option<i64>, Option<i64>),
    /// Wildcard: `[*]` or `.*`
    Wildcard,
    /// Filter predicate on array: `[price < 10]`
    Filter(Predicate),
}

/// Error from parsing a query string.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueryError {
    #[error("query must start with '.'")]
    MustStartWithDot,
    #[error("unexpected character '{ch}' at position {pos}")]
    UnexpectedChar { ch: char, pos: usize },
    #[error("unclosed bracket at position {pos}")]
    UnclosedBracket { pos: usize },
    #[error("unclosed quote at position {pos}")]
    UnclosedQuote { pos: usize },
    #[error("invalid index '{value}' at position {pos}")]
    InvalidIndex { value: String, pos: usize },
    #[error("invalid filter predicate '{expr}' at position {pos}")]
    InvalidPredicate { expr: String, pos: usize },
    #[error("empty query")]
    Empty,
}

/// Parse a predicate expression like `price < 10` or `name == "Alice"`.
pub fn parse_predicate(expr: &str) -> Result<Predicate, String> {
    let expr = expr.trim();

    // Try two-char operators first, then single-char
    let ops = [
        ("==", CompareOp::Eq),
        ("!=", CompareOp::Ne),
        ("<=", CompareOp::Le),
        (">=", CompareOp::Ge),
        ("<", CompareOp::Lt),
        (">", CompareOp::Gt),
    ];

    for (op_str, op) in &ops {
        if let Some(idx) = expr.find(op_str) {
            let field = expr[..idx].trim().to_string();
            let value_str = expr[idx + op_str.len()..].trim();

            if field.is_empty() || value_str.is_empty() {
                return Err(format!("incomplete predicate: {expr}"));
            }

            let value = parse_filter_value(value_str)?;
            return Ok(Predicate {
                field,
                op: op.clone(),
                value,
            });
        }
    }

    Err(format!("no valid operator found in: {expr}"))
}

/// Parse a filter value literal: string, number, bool, or null.
fn parse_filter_value(s: &str) -> Result<FilterValue, String> {
    let s = s.trim();

    // Quoted string
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return Ok(FilterValue::String(s[1..s.len() - 1].to_string()));
    }

    // Boolean
    if s == "true" {
        return Ok(FilterValue::Bool(true));
    }
    if s == "false" {
        return Ok(FilterValue::Bool(false));
    }

    // Null
    if s == "null" {
        return Ok(FilterValue::Null);
    }

    // Number
    if let Ok(n) = s.parse::<f64>() {
        return Ok(FilterValue::Number(n));
    }

    // Unquoted string (treat as string literal)
    Ok(FilterValue::String(s.to_string()))
}

/// Parse a dot-notation query string into path segments.
///
/// # Examples
/// ```
/// use jdx::engine::query::{parse, PathSegment};
///
/// let segments = parse(".store.books[0].author").unwrap();
/// assert_eq!(segments, vec![
///     PathSegment::Key("store".into()),
///     PathSegment::Key("books".into()),
///     PathSegment::Index(0),
///     PathSegment::Key("author".into()),
/// ]);
/// ```
pub fn parse(input: &str) -> Result<Vec<PathSegment>, QueryError> {
    if input.is_empty() {
        return Err(QueryError::Empty);
    }

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    if chars[0] != '.' {
        return Err(QueryError::MustStartWithDot);
    }

    // Just the root `.`
    if len == 1 {
        return Ok(vec![]);
    }

    let mut segments = Vec::new();
    let mut i = 1; // skip leading `.`

    // Parse the first key right after the leading dot (if it's not a bracket or another dot)
    if i < len && chars[i] != '.' && chars[i] != '[' {
        if chars[i] == '*' {
            segments.push(PathSegment::Wildcard);
            i += 1;
        } else {
            let key_start = i;
            while i < len && chars[i] != '.' && chars[i] != '[' {
                i += 1;
            }
            let key: String = chars[key_start..i].iter().collect();
            segments.push(PathSegment::Key(key));
        }
    }

    while i < len {
        match chars[i] {
            // Bracket notation: `[...]`
            '[' => {
                i = parse_bracket(&chars, i, len, &mut segments)?;
            }
            // Dot: start of a new key segment
            '.' => {
                i += 1; // skip `.`

                if i >= len {
                    // Trailing dot (e.g., `.foo.`) — partial input, not error
                    break;
                }

                // Handle `.*` wildcard
                if chars[i] == '*' {
                    segments.push(PathSegment::Wildcard);
                    i += 1;
                    continue;
                }

                // Handle `.[` bracket after dot
                if chars[i] == '[' {
                    continue; // let the bracket handler deal with it
                }

                // Handle `..` (double dot) — treat as partial, skip
                if chars[i] == '.' {
                    continue;
                }

                // Regular key
                let key_start = i;
                while i < len && chars[i] != '.' && chars[i] != '[' {
                    i += 1;
                }
                let key: String = chars[key_start..i].iter().collect();
                segments.push(PathSegment::Key(key));
            }
            c => {
                return Err(QueryError::UnexpectedChar { ch: c, pos: i });
            }
        }
    }

    Ok(segments)
}

/// Parse a bracket expression `[...]` starting at position `i`.
/// Returns the position after the closing `]`.
fn parse_bracket(
    chars: &[char],
    start: usize,
    len: usize,
    segments: &mut Vec<PathSegment>,
) -> Result<usize, QueryError> {
    let bracket_start = start;
    let mut i = start + 1; // skip `[`

    if i >= len {
        return Err(QueryError::UnclosedBracket { pos: bracket_start });
    }

    match chars[i] {
        // Wildcard: `[*]`
        '*' => {
            i += 1;
            if i >= len || chars[i] != ']' {
                return Err(QueryError::UnclosedBracket { pos: bracket_start });
            }
            segments.push(PathSegment::Wildcard);
            i += 1; // skip `]`
        }
        // Quoted key: `["key"]`
        '"' => {
            i += 1; // skip opening `"`
            let key_start = i;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i >= len {
                return Err(QueryError::UnclosedQuote { pos: key_start - 1 });
            }
            let key: String = chars[key_start..i].iter().collect();
            i += 1; // skip closing `"`
            if i >= len || chars[i] != ']' {
                return Err(QueryError::UnclosedBracket { pos: bracket_start });
            }
            segments.push(PathSegment::Key(key));
            i += 1; // skip `]`
        }
        // Number (index or slice)
        c if c.is_ascii_digit() || c == '-' || c == ':' => {
            let content_start = i;
            while i < len && chars[i] != ']' {
                i += 1;
            }
            if i >= len {
                return Err(QueryError::UnclosedBracket { pos: bracket_start });
            }
            let content: String = chars[content_start..i].iter().collect();
            i += 1; // skip `]`

            // Check for slice notation
            if content.contains(':') {
                let parts: Vec<&str> = content.splitn(2, ':').collect();
                let slice_start = if parts[0].is_empty() {
                    None
                } else {
                    Some(
                        parts[0]
                            .parse::<i64>()
                            .map_err(|_| QueryError::InvalidIndex {
                                value: parts[0].to_string(),
                                pos: content_start,
                            })?,
                    )
                };
                let slice_end = if parts[1].is_empty() {
                    None
                } else {
                    Some(
                        parts[1]
                            .parse::<i64>()
                            .map_err(|_| QueryError::InvalidIndex {
                                value: parts[1].to_string(),
                                pos: content_start,
                            })?,
                    )
                };
                segments.push(PathSegment::Slice(slice_start, slice_end));
            } else {
                let idx = content
                    .parse::<i64>()
                    .map_err(|_| QueryError::InvalidIndex {
                        value: content.clone(),
                        pos: content_start,
                    })?;
                segments.push(PathSegment::Index(idx));
            }
        }
        // Anything else: try to parse as a filter predicate (e.g., `[price < 10]`)
        _ => {
            let content_start = i;
            while i < len && chars[i] != ']' {
                i += 1;
            }
            if i >= len {
                return Err(QueryError::UnclosedBracket { pos: bracket_start });
            }
            let content: String = chars[content_start..i].iter().collect();
            i += 1; // skip `]`

            match parse_predicate(&content) {
                Ok(pred) => {
                    segments.push(PathSegment::Filter(pred));
                }
                Err(_) => {
                    return Err(QueryError::InvalidPredicate {
                        expr: content,
                        pos: content_start,
                    });
                }
            }
        }
    }

    Ok(i)
}

/// Return the "last keyword" being typed (partial key for suggestion matching).
/// For `.foo.bar.ba`, returns `"ba"`. For `.foo.bar.`, returns `""`.
pub fn get_last_keyword(input: &str) -> String {
    if input.is_empty() || input == "." {
        return String::new();
    }

    // Find the last `.` or `[`
    let bytes = input.as_bytes();
    let mut last_sep = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' || b == b'[' {
            last_sep = i;
        }
    }

    let after = &input[last_sep + 1..];
    // Strip trailing `]` if present
    after.trim_end_matches(']').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let result = parse(".foo.bar").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("foo".into()),
                PathSegment::Key("bar".into()),
            ]
        );
    }

    #[test]
    fn test_parse_with_index() {
        let result = parse(".users[0].name").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("users".into()),
                PathSegment::Index(0),
                PathSegment::Key("name".into()),
            ]
        );
    }

    #[test]
    fn test_parse_negative_index() {
        let result = parse(".items[-1]").unwrap();
        assert_eq!(
            result,
            vec![PathSegment::Key("items".into()), PathSegment::Index(-1),]
        );
    }

    #[test]
    fn test_parse_slice() {
        let result = parse(".items[0:5]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Slice(Some(0), Some(5)),
            ]
        );
    }

    #[test]
    fn test_parse_slice_open_start() {
        let result = parse(".items[:3]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Slice(None, Some(3)),
            ]
        );
    }

    #[test]
    fn test_parse_slice_open_end() {
        let result = parse(".items[2:]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Slice(Some(2), None),
            ]
        );
    }

    #[test]
    fn test_parse_wildcard_bracket() {
        let result = parse(".items[*]").unwrap();
        assert_eq!(
            result,
            vec![PathSegment::Key("items".into()), PathSegment::Wildcard,]
        );
    }

    #[test]
    fn test_parse_wildcard_dot() {
        let result = parse(".items.*").unwrap();
        assert_eq!(
            result,
            vec![PathSegment::Key("items".into()), PathSegment::Wildcard,]
        );
    }

    #[test]
    fn test_parse_quoted_key() {
        let result = parse(".[\"key.with.dot\"]").unwrap();
        assert_eq!(result, vec![PathSegment::Key("key.with.dot".into())]);
    }

    #[test]
    fn test_parse_root_only() {
        let result = parse(".").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_empty_error() {
        assert_eq!(parse(""), Err(QueryError::Empty));
    }

    #[test]
    fn test_parse_no_leading_dot_error() {
        assert_eq!(parse("foo"), Err(QueryError::MustStartWithDot));
    }

    #[test]
    fn test_parse_unclosed_bracket() {
        assert!(matches!(
            parse(".foo[0"),
            Err(QueryError::UnclosedBracket { .. })
        ));
    }

    #[test]
    fn test_parse_unclosed_quote() {
        assert!(matches!(
            parse(".[\"unclosed"),
            Err(QueryError::UnclosedQuote { .. })
        ));
    }

    #[test]
    fn test_parse_complex_path() {
        let result = parse(".store.books[0].authors[*].name").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("store".into()),
                PathSegment::Key("books".into()),
                PathSegment::Index(0),
                PathSegment::Key("authors".into()),
                PathSegment::Wildcard,
                PathSegment::Key("name".into()),
            ]
        );
    }

    #[test]
    fn test_get_last_keyword_partial() {
        assert_eq!(get_last_keyword(".foo.ba"), "ba");
    }

    #[test]
    fn test_get_last_keyword_trailing_dot() {
        assert_eq!(get_last_keyword(".foo."), "");
    }

    #[test]
    fn test_get_last_keyword_root() {
        assert_eq!(get_last_keyword("."), "");
    }

    #[test]
    fn test_get_last_keyword_empty() {
        assert_eq!(get_last_keyword(""), "");
    }

    // --- Filter predicate tests ---

    #[test]
    fn test_parse_filter_number_lt() {
        let result = parse(".books[price < 10]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("books".into()),
                PathSegment::Filter(Predicate {
                    field: "price".into(),
                    op: CompareOp::Lt,
                    value: FilterValue::Number(10.0),
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_string_eq() {
        let result = parse(".users[role == \"admin\"]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("users".into()),
                PathSegment::Filter(Predicate {
                    field: "role".into(),
                    op: CompareOp::Eq,
                    value: FilterValue::String("admin".into()),
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_ge() {
        let result = parse(".items[score >= 90]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Filter(Predicate {
                    field: "score".into(),
                    op: CompareOp::Ge,
                    value: FilterValue::Number(90.0),
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_ne() {
        let result = parse(".items[status != \"deleted\"]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Filter(Predicate {
                    field: "status".into(),
                    op: CompareOp::Ne,
                    value: FilterValue::String("deleted".into()),
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_bool() {
        let result = parse(".users[active == true]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("users".into()),
                PathSegment::Filter(Predicate {
                    field: "active".into(),
                    op: CompareOp::Eq,
                    value: FilterValue::Bool(true),
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_null() {
        let result = parse(".items[deleted == null]").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Filter(Predicate {
                    field: "deleted".into(),
                    op: CompareOp::Eq,
                    value: FilterValue::Null,
                }),
            ]
        );
    }

    #[test]
    fn test_parse_filter_with_continuation() {
        let result = parse(".store.books[price < 10].title").unwrap();
        assert_eq!(
            result,
            vec![
                PathSegment::Key("store".into()),
                PathSegment::Key("books".into()),
                PathSegment::Filter(Predicate {
                    field: "price".into(),
                    op: CompareOp::Lt,
                    value: FilterValue::Number(10.0),
                }),
                PathSegment::Key("title".into()),
            ]
        );
    }

    #[test]
    fn test_parse_predicate_all_ops() {
        assert_eq!(parse_predicate("a == 1").unwrap().op, CompareOp::Eq);
        assert_eq!(parse_predicate("a != 1").unwrap().op, CompareOp::Ne);
        assert_eq!(parse_predicate("a < 1").unwrap().op, CompareOp::Lt);
        assert_eq!(parse_predicate("a > 1").unwrap().op, CompareOp::Gt);
        assert_eq!(parse_predicate("a <= 1").unwrap().op, CompareOp::Le);
        assert_eq!(parse_predicate("a >= 1").unwrap().op, CompareOp::Ge);
    }

    #[test]
    fn test_parse_predicate_float() {
        let pred = parse_predicate("price < 9.99").unwrap();
        assert_eq!(pred.field, "price");
        assert_eq!(pred.value, FilterValue::Number(9.99));
    }
}
