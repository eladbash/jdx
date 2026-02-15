use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::prompts;
use super::service::{AiProvider, AiQuery, AiResponse};

/// OpenAI-compatible API provider (works with OpenAI, Azure, local APIs).
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String, endpoint: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            endpoint: endpoint.unwrap_or_else(|| "https://api.openai.com/v1".into()),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn query(&self, request: &AiQuery) -> Result<AiResponse> {
        let system_prompt = prompts::build_system_prompt(&request.schema_summary);
        let user_prompt = prompts::build_user_prompt(&request.question);

        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: system_prompt,
                },
                Message {
                    role: "user".into(),
                    content: user_prompt,
                },
            ],
            temperature: 0.1,
            max_tokens: 200,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<ChatResponse>()
            .await?;

        let text = response
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();

        // Parse response: first line is the path, rest is explanation
        let mut lines = text.lines();
        let path_expression = lines.next().unwrap_or("").to_string();
        let explanation: String = lines
            .filter(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# "))
            .collect::<Vec<_>>()
            .join(" ");

        Ok(AiResponse {
            path_expression,
            explanation: if explanation.is_empty() {
                None
            } else {
                Some(explanation)
            },
        })
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}
