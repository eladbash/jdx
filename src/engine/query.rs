use thiserror::Error;

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
    #[error("empty query")]
    Empty,
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
        _ => {
            return Err(QueryError::UnexpectedChar {
                ch: chars[i],
                pos: i,
            });
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
}
