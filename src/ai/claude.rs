use serde::{Deserialize, Serialize};
use reqwest::Client;

use crate::config::ProviderConfig;
use crate::error::{Result, SageError};
use super::{AiResponse, TokenUsage};

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    temperature: f32,
    max_tokens: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeMessage {
    role: String,
    content: Vec<ClaudeContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeResponse {
    content: Vec<ClaudeResponseContent>,
    usage: ClaudeUsage,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeUsage {
    input_tokens: usize,
    output_tokens: usize,
}

pub async fn call_claude_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<AiResponse> {
    let client = Client::new();

    let model = provider_config.model.clone().unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

    let request = ClaudeRequest {
        model,
        messages: vec![
            ClaudeMessage {
                role: "user".to_string(),
                content: vec![
                    ClaudeContent {
                        content_type: "text".to_string(),
                        text: prompt.to_string(),
                    },
                ],
            },
        ],
        temperature: 0.7,
        max_tokens,
    };

    let response = client.post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", &provider_config.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await
        .map_err(|e| SageError::ApiNetworkError {
            provider: "Claude".to_string(),
            details: e.to_string(),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 || status == 403 {
            return Err(SageError::ApiAuthError {
                provider: "Claude".to_string(),
            });
        }

        return Err(SageError::ApiResponseError {
            provider: "Claude".to_string(),
            details: error_text,
        });
    }

    let response_data: ClaudeResponse = response.json().await
        .map_err(|e| SageError::ApiResponseError {
            provider: "Claude".to_string(),
            details: format!("Failed to parse response: {}", e),
        })?;

    if let Some(content) = response_data.content.first() {
        if content.content_type == "text" {
            return Ok(AiResponse {
                message: content.text.trim().to_string(),
                usage: TokenUsage {
                    input_tokens: response_data.usage.input_tokens,
                    output_tokens: response_data.usage.output_tokens,
                    total_tokens: response_data.usage.input_tokens + response_data.usage.output_tokens,
                },
            });
        }
    }

    Err(SageError::ApiNoResponse {
        provider: "Claude".to_string(),
    })
}
