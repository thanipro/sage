/// Prompt templates for AI commit message generation
///
/// This module contains all prompt templates used for generating commit messages.
/// Templates are easy to edit and maintain in one central location.

use crate::cli::CommitStyle;

/// System prompt for OpenAI (sets the AI's role and behavior)
pub const OPENAI_SYSTEM_PROMPT: &str =
    "You are a helpful assistant that generates concise git commit messages \
     and branch names. You MUST output PLAIN TEXT ONLY with NO markdown \
     formatting whatsoever.";

/// Base prompt template for generating commit messages
pub const BASE_PROMPT_TEMPLATE: &str = r#"Generate a concise and descriptive git commit message for the following changes.

IMPORTANT RULES:
- Follow the conventional commits format: type(scope): description
- Use PLAIN TEXT ONLY - no markdown formatting
- Do NOT use asterisks (**), underscores (__), backticks (`), or any other formatting
- Do NOT wrap the message in code blocks
- Output only the commit message text, nothing else
- Keep it concise and focused on WHAT changed and WHY

{style_instructions}

Additional context: {context}

Files changed:
{files_changed}

Diff:
{diff}"#;

/// Branch name generation prompt template
pub const BRANCH_NAME_TEMPLATE: &str = r#"Generate a concise and descriptive git branch name for the following changes.

IMPORTANT RULES:
- Use kebab-case format (lowercase with hyphens): feature/add-user-auth
- Start with a type prefix: feature/, bugfix/, hotfix/, refactor/, docs/, test/, chore/
- Keep it SHORT and descriptive (max 50 characters total)
- Use PLAIN TEXT ONLY - no markdown, asterisks, or special characters
- Only use letters, numbers, hyphens, and forward slashes
- Output ONLY the branch name, nothing else

Common patterns:
- feature/add-authentication
- bugfix/fix-login-error
- refactor/simplify-api-calls
- docs/update-readme

Additional context: {context}

Files changed:
{files_changed}

Diff:
{diff}"#;

/// Get style-specific instructions for commit message generation
pub fn get_style_instructions(style: Option<CommitStyle>) -> &'static str {
    match style {
        Some(CommitStyle::Standard) | None => {
            "Use standard conventional commits format: 'type(scope): description'\n\
             Common types: feat, fix, docs, style, refactor, test, chore"
        },
        Some(CommitStyle::Detailed) => {
            "Create a detailed multi-line commit message following Git convention:\n\
             - First line: Short summary in conventional commits format (max 50 chars)\n\
             - Second line: MUST be blank\n\
             - Following lines: Detailed explanation of what changed and why\n\
             - Use bullet points with '- ' for listing changes\n\
             - Wrap lines at 72 characters\n\
             \n\
             Example format:\n\
             feat(auth): add JWT token validation\n\
             \n\
             - Implement token verification middleware\n\
             - Add expiration checking\n\
             - Handle refresh token logic"
        },
        Some(CommitStyle::Short) => {
            "Create an extremely concise one-line commit message.\n\
             Maximum 50 characters. Be direct and specific."
        },
    }
}

/// Build the complete prompt for commit message generation
pub fn build_commit_prompt(
    style: Option<CommitStyle>,
    context: &str,
    files_changed: &str,
    diff: &str,
) -> String {
    let style_instructions = get_style_instructions(style);
    let context_text = if context.is_empty() { "None" } else { context };

    BASE_PROMPT_TEMPLATE
        .replace("{style_instructions}", style_instructions)
        .replace("{context}", context_text)
        .replace("{files_changed}", files_changed)
        .replace("{diff}", diff)
}

/// Build the prompt for branch name generation
pub fn build_branch_prompt(
    context: &str,
    files_changed: &str,
    diff: &str,
) -> String {
    let context_text = if context.is_empty() { "None" } else { context };

    BRANCH_NAME_TEMPLATE
        .replace("{context}", context_text)
        .replace("{files_changed}", files_changed)
        .replace("{diff}", diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_commit_prompt() {
        let prompt = build_commit_prompt(
            None,
            "Bug fix",
            "src/main.rs",
            "+ fixed bug",
        );

        assert!(prompt.contains("PLAIN TEXT ONLY"));
        assert!(prompt.contains("Bug fix"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("+ fixed bug"));
    }

    #[test]
    fn test_style_instructions() {
        let standard = get_style_instructions(Some(CommitStyle::Standard));
        assert!(standard.contains("conventional commits"));

        let detailed = get_style_instructions(Some(CommitStyle::Detailed));
        assert!(detailed.contains("multi-line"));

        let short = get_style_instructions(Some(CommitStyle::Short));
        assert!(short.contains("50 characters"));

        // Test None defaults to standard
        let default = get_style_instructions(None);
        assert!(default.contains("conventional commits"));
    }
}
