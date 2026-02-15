use anyhow::Result;
use async_trait::async_trait;

/// A query request sent to an AI provider.
#[derive(Debug, Clone)]
pub struct AiQuery {
    /// The natural language question from the user
    pub question: String,
    /// The JSON schema summary (inferred from the data)
    pub schema_summary: String,
    /// Actual JSON data (truncated if large) for the AI to compute answers
    pub data_context: String,
}

/// A response from an AI provider.
#[derive(Debug, Clone)]
pub struct AiResponse {
    /// The natural language text answer
    pub answer: String,
    /// Optional jdx query suggestion (user can apply it to explore the data)
    pub suggested_query: Option<String>,
}

/// Trait for AI providers (OpenAI, Ollama, etc.)
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Send a natural language query and get back a JSON path expression.
    async fn query(&self, request: &AiQuery) -> Result<AiResponse>;

    /// Get the name of this provider (for display/logging).
    fn name(&self) -> &str;
}

/// The AI service dispatches queries to the configured provider.
pub struct AiService {
    provider: Option<Box<dyn AiProvider>>,
}

impl AiService {
    /// Create a new AI service with no provider (disabled).
    pub fn new() -> Self {
        Self { provider: None }
    }

    /// Create a new AI service with a provider.
    pub fn with_provider(provider: Box<dyn AiProvider>) -> Self {
        Self {
            provider: Some(provider),
        }
    }

    /// Check if AI is available.
    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// Send a query to the AI provider.
    pub async fn query(&self, request: &AiQuery) -> Result<AiResponse> {
        match &self.provider {
            Some(provider) => provider.query(request).await,
            None => {
                anyhow::bail!("No AI provider configured. Set up AI in ~/.config/jdx/config.toml")
            }
        }
    }
}

impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse raw AI response text into an answer and optional suggested query.
///
/// Expected format:
///   First line(s): natural language answer text
///   Optional line: `Query: .some.path :transform`
///
/// If the AI only returns a query (starts with `.`), treat it as both answer and query.
pub fn parse_ai_response(text: &str) -> AiResponse {
    let text = text.trim();

    let mut answer_lines = Vec::new();
    let mut suggested_query: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        // Check for "Query: ..." line
        if let Some(q) = trimmed.strip_prefix("Query:") {
            let q = q
                .trim()
                .trim_start_matches('`')
                .trim_end_matches('`')
                .trim()
                .to_string();
            if !q.is_empty() {
                suggested_query = Some(q);
            }
            continue;
        }

        // Skip markdown backtick fences
        if trimmed.starts_with("```") {
            continue;
        }

        answer_lines.push(line.to_string());
    }

    let answer = answer_lines.join("\n").trim().to_string();

    // If the entire response is just a jdx query (starts with .), treat it as a query
    if answer.starts_with('.') && !answer.contains(' ')
        || (answer.starts_with('.') && answer.contains(" :"))
    {
        let query = answer.split('\n').next().unwrap_or("").trim().to_string();
        return AiResponse {
            answer: format!("Suggested query: {query}"),
            suggested_query: Some(query),
        };
    }

    AiResponse {
        answer,
        suggested_query,
    }
}
