use anyhow::Result;
use async_trait::async_trait;

/// A query request sent to an AI provider.
#[derive(Debug, Clone)]
pub struct AiQuery {
    /// The natural language question from the user
    pub question: String,
    /// The JSON schema summary (inferred from the data)
    pub schema_summary: String,
}

/// A response from an AI provider.
#[derive(Debug, Clone)]
pub struct AiResponse {
    /// The generated JSON path expression
    pub path_expression: String,
    /// Human-readable explanation (optional)
    pub explanation: Option<String>,
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
