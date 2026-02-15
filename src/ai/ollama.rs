use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::prompts;
use super::service::{parse_ai_response, AiProvider, AiQuery, AiResponse};

/// Ollama local LLM provider.
pub struct OllamaProvider {
    client: Client,
    model: String,
    endpoint: String,
}

impl OllamaProvider {
    pub fn new(model: String, endpoint: Option<String>) -> Self {
        Self {
            client: Client::new(),
            model,
            endpoint: endpoint.unwrap_or_else(|| "http://localhost:11434".into()),
        }
    }
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn query(&self, request: &AiQuery) -> Result<AiResponse> {
        let system_prompt = prompts::build_system_prompt(&request.schema_summary);
        let user_prompt = prompts::build_user_prompt(&request.question, &request.data_context);

        let body = GenerateRequest {
            model: self.model.clone(),
            prompt: user_prompt,
            system: system_prompt,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<GenerateResponse>()
            .await?;

        let text = response.response.trim().to_string();
        Ok(parse_ai_response(&text))
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}
