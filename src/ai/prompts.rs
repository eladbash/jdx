/// Build the system prompt for AI-assisted JSON querying.
pub fn build_system_prompt(schema_summary: &str) -> String {
    format!(
        r##"You are a JSON path query assistant. Given a JSON schema and a natural language question,
you must return a valid dot-notation path expression that answers the question.

The JSON data has this schema:
{schema_summary}

Rules:
- Return ONLY the path expression, nothing else.
- Use dot-notation: .field.subfield
- Use array indices: .items[0]
- Use slices: .items[0:5]
- If the question involves filtering, return the closest valid path.
- If the question cannot be answered with a simple path, explain briefly after the path on a new line starting with "# ".

Examples:
Question: "What is the first user name?"
Answer: .users[0].name

Question: "How many items are there?"
Answer: .items
# Use :count to get the total
"##
    )
}

/// Build the user prompt for a specific question.
pub fn build_user_prompt(question: &str) -> String {
    format!("Question: \"{question}\"\nAnswer:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_schema() {
        let prompt = build_system_prompt("{ users: [object], count: number }");
        assert!(prompt.contains("users: [object]"));
        assert!(prompt.contains("dot-notation"));
    }

    #[test]
    fn test_user_prompt_contains_question() {
        let prompt = build_user_prompt("find all users");
        assert!(prompt.contains("find all users"));
    }
}
