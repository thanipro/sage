pub mod openai;
pub mod claude;

use regex::Regex;

use crate::config::Config;
use crate::error::{Result, SageError};

/// Token usage information from AI API calls
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}


/// Response from AI API including the message and token usage
pub struct AiResponse {
    pub message: String,
    pub usage: TokenUsage,
}

pub async fn call_ai(config: &Config, prompt: &str) -> Result<AiResponse> {
    let (provider_name, provider_config) = config.get_active_provider_config()?;

    match provider_name.as_str() {
        "openai" => openai::call_openai_api(provider_config, prompt, config.max_tokens).await,
        "claude" => claude::call_claude_api(provider_config, prompt, config.max_tokens).await,
        _ => Err(SageError::ApiUnsupportedProvider {
            provider: provider_name.clone()
        })
    }
}

pub fn sanitize_commit_message(message: &str) -> String {
    let mut result = message.trim().to_string();

    let code_block_re = Regex::new(r"```\w*\s*([\s\S]*?)\s*```").unwrap();
    if let Some(captures) = code_block_re.captures(&result) {
        if let Some(inner_text) = captures.get(1) {
            result = inner_text.as_str().trim().to_string();
        }
    }

    // Remove bold markdown first
    let bold_re = Regex::new(r"\*\*([^*]+)\*\*|__([^_]+)__").unwrap();
    result = bold_re.replace_all(&result, "$1$2").to_string();

    // Remove italic markdown after bold is removed
    let italic_asterisk_re = Regex::new(r"\*([^*]+)\*").unwrap();
    result = italic_asterisk_re.replace_all(&result, "$1").to_string();

    let italic_underscore_re = Regex::new(r"_([^_]+)_").unwrap();
    result = italic_underscore_re.replace_all(&result, "$1").to_string();

    let inline_code_re = Regex::new(r"`([^`]+)`").unwrap();
    result = inline_code_re.replace_all(&result, "$1").to_string();

    result = result.replace("**", "").replace("__", "").replace("*", "").replace("_", "");

    let whitespace_re = Regex::new(r"\s+").unwrap();
    result = whitespace_re.replace_all(&result, " ").to_string();

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_bold_markdown() {
        let input = "**feat(middlewares): refactor provider onboarding logic**";
        let expected = "feat(middlewares): refactor provider onboarding logic";
        assert_eq!(sanitize_commit_message(input), expected);
    }

    #[test]
    fn test_sanitize_code_blocks() {
        let input = "```\nfeat: add new feature\n```";
        let expected = "feat: add new feature";
        assert_eq!(sanitize_commit_message(input), expected);
    }

    #[test]
    fn test_sanitize_inline_code() {
        let input = "fix: resolve `bug` in authentication";
        let expected = "fix: resolve bug in authentication";
        assert_eq!(sanitize_commit_message(input), expected);
    }

    #[test]
    fn test_sanitize_mixed_formatting() {
        let input = "**feat**: add `new` _feature_ with __improvements__";
        let expected = "feat: add new feature with improvements";
        assert_eq!(sanitize_commit_message(input), expected);
    }

    #[test]
    fn test_sanitize_plain_text() {
        let input = "feat: add new feature";
        let expected = "feat: add new feature";
        assert_eq!(sanitize_commit_message(input), expected);
    }
}
