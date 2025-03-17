use std::error::Error;
use std::process::{Command, exit, Stdio};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use clap::{Parser, Args, Subcommand};
use std::env;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use tokio::runtime::Runtime;
use colored::Colorize;
use std::time::Instant;
use std::collections::HashMap;

const CONFIG_FILE: &str = ".sage-config.json";
const MAX_DIFF_SIZE: usize = 15000;

#[derive(Parser)]
#[command(name = "sage")]
#[command(author = "Author")]
#[command(version = "1.0.0")]
#[command(about = "AI-powered Git Commit Message generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(name = "FILES")]
    files: Vec<String>,

    #[arg(short, long)]
    dry_run: bool,

     #[arg(short, long)]
    all: bool,

    #[arg(short, long)]
    message: Option<String>,

    #[arg(short, long)]
    context: Option<String>,

    #[arg(short, long)]
    show_diff: bool,

    #[arg(long)]
    amend: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    push: bool,
}

#[derive(Subcommand)]
enum Commands {
    Config(ConfigArgs),
    Use {
        provider: String,
    },
}

#[derive(Args, Debug)]
struct ConfigArgs {
    #[arg(short, long)]
    provider: Option<String>,

    #[arg(short, long)]
    key: Option<String>,

    #[arg(long)]
    update_key: Option<String>,

    #[arg(short, long)]
    show: bool,

    #[arg(long)]
    model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct ProviderConfig {
    api_key: String,
    model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    active_provider: String,
    providers: HashMap<String, ProviderConfig>,
    max_tokens: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), ProviderConfig::default());

        Config {
            active_provider: "openai".to_string(),
            providers,
            max_tokens: Some(300),
        }
    }
}

impl Config {
    fn get_active_provider_config(&self) -> Result<(&String, &ProviderConfig), Box<dyn Error>> {
        let provider = &self.active_provider;
        let config = self.providers.get(provider)
            .ok_or_else(|| format!("No configuration found for provider: {}", provider))?;

        if config.api_key.is_empty() {
            return Err(format!("API key not set for provider: {}. Run 'sage config -p {} -k <your-api-key>'", provider, provider).into());
        }

        Ok((provider, config))
    }

    fn set_provider(&mut self, provider: &str, api_key: Option<String>, model: Option<String>) -> Result<(), Box<dyn Error>> {
        let config = self.providers.entry(provider.to_string())
            .or_insert_with(ProviderConfig::default);

        if let Some(key) = api_key {
            config.api_key = key;
        }

        if let Some(m) = model {
            config.model = Some(m);
        }

        self.active_provider = provider.to_string();
        Ok(())
    }

    fn update_key(&mut self, provider: &str, api_key: &str) -> Result<(), Box<dyn Error>> {
        let config = self.providers.entry(provider.to_string())
            .or_insert_with(ProviderConfig::default);

        config.api_key = api_key.to_string();
        Ok(())
    }
}

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
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

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
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaudeResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !is_git_repo() {
        eprintln!("{}", "Error: Not in a git repository".red());
        exit(1);
    }

    match &cli.command {
        Some(Commands::Config(args)) => {
            handle_config_command(args)?;
        },
        Some(Commands::Use { provider }) => {
            use_provider(provider)?;
        },
        None => {
            let rt = Runtime::new()?;
            rt.block_on(async {
                run_commit_flow(&cli).await
            })?;
        }
    }

    Ok(())
}

fn handle_config_command(args: &ConfigArgs) -> Result<(), Box<dyn Error>> {
    let config_path = get_config_path()?;
    let mut config = load_config(&config_path)?;

    if args.show {
        show_config(&config);
        return Ok(());
    }

    let mut updated = false;

    if let Some(provider) = &args.provider {
        if let Some(key) = &args.key {
            config.set_provider(provider, Some(key.clone()), args.model.clone())?;
            println!("{}", format!("Provider set to: {} with new API key", provider).green());
            updated = true;
        } else {
            config.set_provider(provider, None, args.model.clone())?;
            println!("{}", format!("Provider set to: {}", provider).green());
            updated = true;
        }
    } else if let Some(provider) = &args.update_key {
        if let Some(key) = &args.key {
            config.update_key(provider, key)?;
            println!("{}", format!("API key updated for provider: {}", provider).green());
            updated = true;
        } else {
            return Err("API key required with --update-key".into());
        }
    } else if let Some(key) = &args.key {
        let provider_name = config.active_provider.clone();
        config.update_key(&provider_name, key)?;
        println!("{}", format!("API key updated for active provider: {}", provider_name).green());
        updated = true;
    } else if let Some(model) = &args.model {
        let provider_name = config.active_provider.clone();
        config.set_provider(&provider_name, None, Some(model.clone()))?;
        println!("{}", format!("Model updated for provider: {}", provider_name).green());
        updated = true;
    }

    if updated {
        save_config(&config, &config_path)?;
        println!("{}", "Configuration saved.".green());
    } else if !args.show {
        show_config(&config);
    }

    Ok(())
}

fn use_provider(provider: &str) -> Result<(), Box<dyn Error>> {
    let config_path = get_config_path()?;
    let mut config = load_config(&config_path)?;

    if !config.providers.contains_key(provider) {
        return Err(format!("Provider '{}' not configured. Run 'sage config -p {} -k <your-api-key>' first", provider, provider).into());
    }

    config.active_provider = provider.to_string();
    save_config(&config, &config_path)?;

    println!("{}", format!("Switched to provider: {}", provider).green());
    Ok(())
}

fn show_config(config: &Config) {
    println!("Current configuration:");
    println!("  Active provider: {}", config.active_provider);

    for (provider, provider_config) in &config.providers {
        println!("\n  Provider: {}{}",
                 provider,
                 if provider == &config.active_provider { " (active)" } else { "" }
        );
        println!("    API Key: {}",
                 if provider_config.api_key.is_empty() {
                     "Not set".red().to_string()
                 } else {
                     "Set (hidden)".green().to_string()
                 }
        );
        if let Some(model) = &provider_config.model {
            println!("    Model: {}", model);
        } else {
            println!("    Model: Default");
        }
    }

    println!("\n  Max tokens: {}", config.max_tokens.unwrap_or(300));
}

fn get_config_path() -> Result<String, Box<dyn Error>> {
    let home_dir = env::var("HOME").map_err(|_| "Could not find home directory")?;
    Ok(Path::new(&home_dir).join(CONFIG_FILE).to_string_lossy().to_string())
}

fn load_config(config_path: &str) -> Result<Config, Box<dyn Error>> {
    let path = Path::new(config_path);
    if !path.exists() {
        return Ok(Config::default());
    }

    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&config_str)?;

    Ok(config)
}

fn save_config(config: &Config, config_path: &str) -> Result<(), Box<dyn Error>> {
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, config_json)?;
    Ok(())
}

fn get_diff(all: bool) -> Result<String, Box<dyn Error>> {
    let args = if all {
        vec!["diff"]
    } else {
        vec!["diff", "--cached"]
    };

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err("Failed to get git diff".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_files_changed(all: bool) -> Result<String, Box<dyn Error>> {
    let args = if all {
        vec!["status", "--porcelain"]
    } else {
        vec!["diff", "--cached", "--name-status"]
    };

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err("Failed to get changed files".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn stage_files(files: &[String]) -> Result<(), Box<dyn Error>> {
    if files.is_empty() {
        return Ok(());
    }

    let mut cmd = Command::new("git");
    cmd.arg("add");

    for file in files {
        cmd.arg(file);
    }

    let status = cmd.status()?;

    if !status.success() {
        return Err("Failed to stage files".into());
    }

    Ok(())
}

fn stage_all_files() -> Result<(), Box<dyn Error>> {
    let status = Command::new("git")
        .args(["add", "--all"])
        .status()?;

    if !status.success() {
        return Err("Failed to stage all files".into());
    }

    Ok(())
}

fn has_staged_changes() -> Result<bool, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()?;

    Ok(!output.success())
}

fn commit_changes(message: &str, amend: bool) -> Result<(), Box<dyn Error>> {
    let mut args = vec!["commit", "-m", message];

    if amend {
        args.push("--amend");
    }

    let status = Command::new("git")
        .args(args)
        .status()?;

    if !status.success() {
        return Err("Failed to commit changes".into());
    }

    Ok(())
}

fn push_changes() -> Result<(), Box<dyn Error>> {
    println!("{}", "Pushing changes...".blue());

    let status = Command::new("git")
        .args(["push"])
        .status()?;

    if !status.success() {
        return Err("Failed to push changes".into());
    }

    println!("{}", "Changes pushed successfully!".green());
    Ok(())
}

async fn run_commit_flow(cli: &Cli) -> Result<(), Box<dyn Error>> {
    if cli.all || !cli.files.is_empty() {
        if cli.all {
            if cli.verbose {
                println!("{}", "Staging all changes...".blue());
            }
            stage_all_files()?;
        }

        else if !cli.files.is_empty() {
            if cli.verbose {
                println!("{}", format!("Staging specified files: {:?}...", cli.files).blue());
            }
            stage_files(&cli.files)?;
        }
    }

    if !has_staged_changes()? {
        println!("{}", "No staged changes found. Nothing to commit.".yellow());
        return Ok(());
    }

    if let Some(message) = &cli.message {
        if cli.dry_run {
            println!("{}", "Would commit with message:".blue());
            println!("{}", message);
        } else {
            commit_changes(message, cli.amend)?;
            println!("{}", "Changes committed successfully!".green());

            if cli.push {
                push_changes()?;
            }
        }

        return Ok(());
    }

    if cli.verbose {
        println!("{}", "Analyzing git repository changes...".blue());
    }

    let start = Instant::now();
    let diff = get_diff(false)?;
    let files_changed = get_files_changed(false)?;

    if diff.trim().is_empty() {
        println!("{}", "No changes detected. Nothing to commit.".yellow());
        return Ok(());
    }

    if cli.show_diff {
        show_changes(&diff, &files_changed)?;
    }

    let truncated_diff = smart_truncate_diff(&diff);

    let context_str = cli.context.as_deref().unwrap_or("");
    let prompt = format!(
        "Generate a concise and descriptive git commit message for the following changes.\n\
        Follow the conventional commits format (type: description).\n\
        Additional context: {}\n\
        Files changed:\n{}\n\nDiff:\n{}",
        context_str, files_changed, truncated_diff
    );

    if cli.verbose {
        println!("{}", "Generating commit message using AI...".blue());
    } else {
        println!("{}", "Analyzing changes...".blue());
    }

    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let message = call_ai(&config, &prompt).await?;

    let elapsed = start.elapsed();
    if cli.verbose {
        println!("{}", format!("Generation took {:.2}s", elapsed.as_secs_f32()).blue());
    }

    println!("\n{}", "Generated commit message:".green().bold());
    println!("{}", message);

    if cli.dry_run {
        println!("\n{}", "Dry run - changes were not committed.".yellow());
    } else {
        let should_commit = confirm_commit()?;

        if should_commit {
            commit_changes(&message, cli.amend)?;
            println!("{}", "Changes committed successfully!".green());

            if cli.push {
                push_changes()?;
            }
        } else {
            println!("{}", "Commit aborted.".yellow());
        }
    }

    Ok(())
}

fn show_changes(diff: &str, files_changed: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", "Changes to be committed:".blue().bold());
    println!("{}", files_changed);
    println!("\n{}", "Diff:".blue().bold());

    let display_len = std::cmp::min(diff.len(), 2000);
    if display_len > 0 {
        println!("{}", &diff[..display_len]);
        if diff.len() > 2000 {
            println!("... [truncated for display]");
        }
    } else {
        println!("(empty diff)");
    }

    Ok(())
}

fn confirm_commit() -> Result<bool, Box<dyn Error>> {
    print!("\nCommit with this message? [Y/n/e for edit] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "e" {

        let temp_file = "/tmp/sage_commit_msg";
        fs::write(temp_file, "")?;

        let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = Command::new(&editor)
            .arg(temp_file)
            .status()?;

        if !status.success() {
            return Err("Failed to edit commit message".into());
        }

        Ok(true)
    } else {
        Ok(input.is_empty() || input == "y")
    }
}

fn smart_truncate_diff(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_SIZE {
        return diff.to_string();
    }

    let mut important_parts = String::new();

    for chunk in diff.split("diff --git").skip(1) {
        let chunk_lines: Vec<&str> = chunk.lines().collect();
        let header_lines = std::cmp::min(5, chunk_lines.len());

        important_parts.push_str("diff --git");
        for i in 0..header_lines {
            important_parts.push_str(chunk_lines[i]);
            important_parts.push('\n');
        }

        if chunk_lines.len() > 20 {
            important_parts.push_str("...[truncated]...\n");
            let mid = chunk_lines.len() / 2;
            for i in mid..(mid + 3).min(chunk_lines.len()) {
                important_parts.push_str(chunk_lines[i]);
                important_parts.push('\n');
            }
        }

        if important_parts.len() > MAX_DIFF_SIZE / 2 {
            break;
        }
    }

    if important_parts.len() < MAX_DIFF_SIZE {
        important_parts
    } else {
        format!("{}... [truncated - diff too large]", &diff[0..MAX_DIFF_SIZE / 2])
    }
}


async fn call_ai(config: &Config, prompt: &str) -> Result<String, Box<dyn Error>> {
    let (provider_name, provider_config) = config.get_active_provider_config()?;

    match provider_name.as_str() {
        "openai" => call_openai_api(provider_config, prompt, config.max_tokens).await,
        "claude" => call_claude_api(provider_config, prompt, config.max_tokens).await,
        _ => Err(format!("Unsupported provider: {}", provider_name).into())
    }
}

async fn call_openai_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let model = provider_config.model.clone().unwrap_or_else(|| "gpt-4-turbo".to_string());

    let request = OpenAIRequest {
        model,
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant that generates concise git commit messages following conventional commits format.".to_string(),
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
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("OpenAI API error: {}", error_text).into());
    }

    let response_data: OpenAIResponse = response.json().await?;
    if let Some(choice) = response_data.choices.first() {
        Ok(choice.message.content.trim().to_string())
    } else {
        Err("No response from OpenAI API".into())
    }
}

async fn call_claude_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<String, Box<dyn Error>> {
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
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Claude API error: {}", error_text).into());
    }

    let response_data: ClaudeResponse = response.json().await?;

    // Extract text from the first content block
    if let Some(content) = response_data.content.first() {
        if content.content_type == "text" {
            return Ok(content.text.trim().to_string());
        }
    }

    Err("No valid response from Claude API".into())
}

fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_truncate_diff() {
        let small_diff = "diff --git a/file.txt b/file.txt\nindex 123..456 789\n--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n-old line\n+new line\n context";
        assert_eq!(smart_truncate_diff(small_diff), small_diff);

        let large_diff = "a".repeat(MAX_DIFF_SIZE + 1000);
        assert!(smart_truncate_diff(&large_diff).len() <= MAX_DIFF_SIZE);
    }
}