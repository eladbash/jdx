/// Build the system prompt for AI-assisted JSON querying.
pub fn build_system_prompt(schema_summary: &str) -> String {
    format!(
        r##"You are a helpful data assistant for jdx, a JSON data explorer. You have access to the actual JSON data and its schema. Your job is to ANSWER the user's question directly in plain English.

## Schema
{schema_summary}

## Response format

1. Answer the question directly in natural language. Be concise (1-3 sentences).
2. If relevant, include specific values, counts, or calculations from the data.
3. On a SEPARATE line at the end, optionally suggest a jdx query the user can run to explore the data themselves, using this format:
   Query: .path.to.data :transform

## jdx query syntax reference (for your Query: suggestions)

Path navigation: .field, .field.sub, .arr[0], .arr[-1], .arr[0:3], .arr[*]
Filter predicates: .arr[field == value], .arr[field < 10], .arr[field != "x"]
Transform commands:
  :keys, :values, :count, :flatten, :pick f1,f2, :omit f1, :sort field,
  :uniq, :group_by field, :filter field op value,
  :sum field, :avg field, :min field, :max field

Transforms chain: .books :filter price < 10 :pick title,price :sort price
Operators: ==, !=, <, >, <=, >=

CRITICAL: NEVER use JSONPath syntax like [?(@.field > value)] or jq syntax like select(). Use jdx syntax only.

## Examples

Question: "What is the total price of all books?"
Answer: The total price of all books is $52.97 (10.99 + 8.99 + 32.99).
Query: .store.books :sum price

Question: "How many users are there?"
Answer: There are 3 users.
Query: .users :count

Question: "Which books cost less than 10 dollars?"
Answer: Only "1984" by George Orwell costs less than $10 (priced at $8.99).
Query: .store.books[price < 10] :pick title,author,price

Question: "What is the most expensive book?"
Answer: "Clean Code" by Robert C. Martin is the most expensive at $32.99.
Query: .store.books :sort price

Question: "Who are the admin users?"
Answer: Alice is the only admin user.
Query: .users[role == "admin"]
"##
    )
}

/// Build the user prompt for a specific question, including actual data.
pub fn build_user_prompt(question: &str, data_context: &str) -> String {
    format!(
        r##"Here is the actual JSON data:
{data_context}

Question: "{question}"
"##
    )
}

/// Truncate JSON data for inclusion in AI prompt.
/// Returns a compact string representation, truncated if too large.
pub fn truncate_data_for_prompt(data: &serde_json::Value, max_chars: usize) -> String {
    let full = serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string());
    if full.len() <= max_chars {
        return full;
    }

    // For large data, use compact format first
    let compact = serde_json::to_string(data).unwrap_or_else(|_| data.to_string());
    if compact.len() <= max_chars {
        return compact;
    }

    // Still too large: truncate with indicator
    let truncated = &compact[..max_chars.saturating_sub(30)];
    format!("{truncated}\n... (data truncated, {total} chars total)", total = compact.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_schema() {
        let prompt = build_system_prompt("{ users: [object], count: number }");
        assert!(prompt.contains("users: [object]"));
        assert!(prompt.contains("jdx query syntax"));
        assert!(prompt.contains(":pick"));
        assert!(prompt.contains(":sort"));
        assert!(prompt.contains(":filter"));
        assert!(prompt.contains(":sum"));
        assert!(prompt.contains("NEVER use JSONPath"));
    }

    #[test]
    fn test_user_prompt_contains_question_and_data() {
        let prompt = build_user_prompt("find all users", "{\"users\": []}");
        assert!(prompt.contains("find all users"));
        assert!(prompt.contains("\"users\""));
    }

    #[test]
    fn test_truncate_small_data() {
        let data = serde_json::json!({"a": 1});
        let result = truncate_data_for_prompt(&data, 1000);
        assert!(result.contains("\"a\""));
    }

    #[test]
    fn test_truncate_large_data() {
        let data = serde_json::json!({"key": "x".repeat(5000)});
        let result = truncate_data_for_prompt(&data, 100);
        assert!(result.contains("truncated"));
    }
}
