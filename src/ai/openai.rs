use serde::{Deserialize, Serialize};
use reqwest::Client;

use crate::config::ProviderConfig;
use crate::error::{Result, SageError};
use crate::prompts;
use super::{sanitize_commit_message, AiResponse, TokenUsage};

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

pub async fn call_openai_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<AiResponse> {
    let client = Client::new();

    let model = provider_config.model.clone().unwrap_or_else(|| "gpt-4-turbo".to_string());

    let request = OpenAIRequest {
        model,
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: prompts::OPENAI_SYSTEM_PROMPT.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens,
    };

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", &provider_config.api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| SageError::ApiNetworkError {
            provider: "OpenAI".to_string(),
            details: e.to_string(),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 || status == 403 {
            return Err(SageError::ApiAuthError {
                provider: "OpenAI".to_string(),
            });
        }

        return Err(SageError::ApiResponseError {
            provider: "OpenAI".to_string(),
            details: error_text,
        });
    }

    let response_data: OpenAIResponse = response.json().await
        .map_err(|e| SageError::ApiResponseError {
            provider: "OpenAI".to_string(),
            details: format!("Failed to parse response: {}", e),
        })?;

    if let Some(choice) = response_data.choices.first() {
        let message = choice.message.content.trim().to_string();
        let sanitized_message = sanitize_commit_message(&message);

        Ok(AiResponse {
            message: sanitized_message,
            usage: TokenUsage {
                input_tokens: response_data.usage.prompt_tokens,
                output_tokens: response_data.usage.completion_tokens,
                total_tokens: response_data.usage.total_tokens,
            },
        })
    } else {
        Err(SageError::ApiNoResponse {
            provider: "OpenAI".to_string(),
        })
    }
}
