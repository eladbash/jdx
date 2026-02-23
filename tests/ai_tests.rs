use jdx::ai::service::parse_ai_response;

// --- parse_ai_response tests (pure function, no HTTP needed) ---

#[test]
fn test_parse_plain_answer() {
    let resp = parse_ai_response("There are 5 users in the dataset.");
    assert_eq!(resp.answer, "There are 5 users in the dataset.");
    assert!(resp.suggested_query.is_none());
}

#[test]
fn test_parse_answer_with_query() {
    let resp = parse_ai_response("There are 5 users.\nQuery: .users :count");
    assert_eq!(resp.answer, "There are 5 users.");
    assert_eq!(resp.suggested_query.as_deref(), Some(".users :count"));
}

#[test]
fn test_parse_query_with_backticks() {
    let resp = parse_ai_response("Found 2 admin users.\nQuery: `.users[role == \"admin\"]`");
    assert_eq!(resp.answer, "Found 2 admin users.");
    assert_eq!(
        resp.suggested_query.as_deref(),
        Some(".users[role == \"admin\"]")
    );
}

#[test]
fn test_parse_multiline_answer() {
    let resp = parse_ai_response("Line 1\nLine 2\nLine 3\nQuery: .data :keys");
    assert_eq!(resp.answer, "Line 1\nLine 2\nLine 3");
    assert_eq!(resp.suggested_query.as_deref(), Some(".data :keys"));
}

#[test]
fn test_parse_pure_query_response() {
    // AI sometimes returns just a query path
    let resp = parse_ai_response(".users[0].name");
    assert!(resp.suggested_query.is_some());
    assert_eq!(resp.suggested_query.as_deref(), Some(".users[0].name"));
}

#[test]
fn test_parse_query_with_transform() {
    let resp = parse_ai_response(".users :sort name");
    assert!(resp.suggested_query.is_some());
    assert_eq!(resp.suggested_query.as_deref(), Some(".users :sort name"));
}

#[test]
fn test_parse_empty_response() {
    let resp = parse_ai_response("");
    assert_eq!(resp.answer, "");
    assert!(resp.suggested_query.is_none());
}

#[test]
fn test_parse_strips_markdown_fences() {
    let resp =
        parse_ai_response("Here is the result:\n```\n.users :count\n```\nQuery: .users :count");
    // Markdown fences should be stripped from the answer
    assert!(!resp.answer.contains("```"));
    assert_eq!(resp.suggested_query.as_deref(), Some(".users :count"));
}

#[test]
fn test_parse_empty_query_line() {
    let resp = parse_ai_response("Some answer.\nQuery:");
    assert_eq!(resp.answer, "Some answer.");
    // Empty query should not be set
    assert!(resp.suggested_query.is_none());
}

#[test]
fn test_parse_whitespace_response() {
    let resp = parse_ai_response("  \n  \n  ");
    assert_eq!(resp.answer, "");
    assert!(resp.suggested_query.is_none());
}

// --- wiremock-based HTTP tests ---

#[cfg(test)]
mod openai_mock_tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use jdx::ai::openai::OpenAiProvider;
    use jdx::ai::service::{AiProvider, AiQuery};

    fn sample_query() -> AiQuery {
        AiQuery {
            question: "How many users?".into(),
            schema_summary: "{ users: [object] }".into(),
            data_context: r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#.into(),
        }
    }

    #[tokio::test]
    async fn test_openai_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "There are 2 users.\nQuery: .users :count"
                    }
                }]
            })))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("test-key".into(), "gpt-4".into(), Some(server.uri()));

        let resp = provider.query(&sample_query()).await.unwrap();
        assert_eq!(resp.answer, "There are 2 users.");
        assert_eq!(resp.suggested_query.as_deref(), Some(".users :count"));
    }

    #[tokio::test]
    async fn test_openai_error_status() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("bad-key".into(), "gpt-4".into(), Some(server.uri()));

        let result = provider.query(&sample_query()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_openai_empty_choices() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": []
            })))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("test-key".into(), "gpt-4".into(), Some(server.uri()));

        let resp = provider.query(&sample_query()).await.unwrap();
        assert_eq!(resp.answer, "");
    }
}

#[cfg(test)]
mod ollama_mock_tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use jdx::ai::ollama::OllamaProvider;
    use jdx::ai::service::{AiProvider, AiQuery};

    fn sample_query() -> AiQuery {
        AiQuery {
            question: "What is the most expensive book?".into(),
            schema_summary: "{ books: [{ title: string, price: number }] }".into(),
            data_context:
                r#"{"books": [{"title": "A", "price": 10}, {"title": "B", "price": 20}]}"#.into(),
        }
    }

    #[tokio::test]
    async fn test_ollama_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "response": "Book B is the most expensive at $20.\nQuery: .books :sort price desc"
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new("llama2".into(), Some(server.uri()));

        let resp = provider.query(&sample_query()).await.unwrap();
        assert!(resp.answer.contains("Book B"));
        assert_eq!(
            resp.suggested_query.as_deref(),
            Some(".books :sort price desc")
        );
    }

    #[tokio::test]
    async fn test_ollama_connection_error() {
        // Point to a port that nothing is listening on
        let provider = OllamaProvider::new("llama2".into(), Some("http://127.0.0.1:1".into()));

        let result = provider.query(&sample_query()).await;
        assert!(result.is_err());
    }
}
