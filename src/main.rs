use std::error::Error;
use std::process::{Command, exit, Stdio};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use clap::{Parser, Args, Subcommand, ValueEnum};
use std::env;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use tokio::runtime::Runtime;
use colored::Colorize;
use std::time::Instant;
use std::collections::HashMap;
use std::fmt;
use regex::Regex;

const CONFIG_FILE: &str = ".sage-config.json";
const MAX_DIFF_SIZE: usize = 15000;
const APP_VERSION: &str = "1.0.0";
const APP_NAME: &str = "sage";

#[derive(Debug, Clone)]
struct SageError(String);

impl fmt::Display for SageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for SageError {}

impl From<&str> for SageError {
    fn from(error: &str) -> Self {
        SageError(error.to_string())
    }
}

impl From<String> for SageError {
    fn from(error: String) -> Self {
        SageError(error)
    }
}

impl From<io::Error> for SageError {
    fn from(error: io::Error) -> Self {
        SageError(format!("IO error: {}", error))
    }
}

impl From<reqwest::Error> for SageError {
    fn from(error: reqwest::Error) -> Self {
        SageError(format!("Network error: {}", error))
    }
}

impl From<serde_json::Error> for SageError {
    fn from(error: serde_json::Error) -> Self {
        SageError(format!("JSON error: {}", error))
    }
}

type Result<T> = std::result::Result<T, SageError>;

#[derive(Parser)]
#[command(name = APP_NAME)]
#[command(author = "Author")]
#[command(version = APP_VERSION)]
#[command(about = "AI-powered Git Commit Message generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// File patterns to add (e.g. '*.rs', 'src/', etc.)
    #[arg(name = "FILES")]
    files: Vec<String>,

    /// Don't actually commit, just print the generated message
    #[arg(short, long)]
    dry_run: bool,

    /// Use all changes (staged + unstaged) for message generation
    #[arg(short, long)]
    all: bool,

    /// Commit message to use (skips AI generation)
    #[arg(short, long)]
    message: Option<String>,

    /// Add additional context to help AI generate better messages
    #[arg(short, long)]
    context: Option<String>,

    /// Show a diff of the changes before committing
    #[arg(short, long)]
    show_diff: bool,

    /// Amend the previous commit
    #[arg(long)]
    amend: bool,

    /// Be more verbose about what's happening
    #[arg(short, long)]
    verbose: bool,

    /// Push changes after commit
    #[arg(short, long)]
    push: bool,

    /// Force push when needed (with --push)
    #[arg(short = 'f', long)]
    force_push: bool,

    /// Skip confirmation prompt (auto-accept commit message)
    #[arg(short = 'y', long = "yes")]
    yes: bool,

    /// Style of commit message to generate
    #[arg(short = 't', long, value_enum)]
    style: Option<CommitStyle>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum CommitStyle {
    /// Standard conventional commits format
    Standard,
    /// More detailed multi-line commit message
    Detailed,
    /// Very short one-line commit message
    Short,
    /// Includes emojis in the commit message
    Emoji,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API settings
    Config(ConfigArgs),

    /// Switch between configured providers
    Use {
        /// Provider to switch to (e.g., "openai", "claude")
        provider: String,
    },

    /// Show git diff without committing
    Diff {
        /// Files to show diff for
        #[arg(name = "FILES")]
        files: Vec<String>,

        /// Show unstaged changes
        #[arg(short, long)]
        all: bool,
    },
}

#[derive(Args, Debug)]
struct ConfigArgs {
    /// Set API provider (openai, claude, etc.)
    #[arg(short, long)]
    provider: Option<String>,

    /// Set API key
    #[arg(short, long)]
    key: Option<String>,

    /// Update API key for a specific provider
    #[arg(long)]
    update_key: Option<String>,

    /// Show current configuration
    #[arg(short, long)]
    show: bool,

    /// Set preferred model for the provider
    #[arg(long)]
    model: Option<String>,

    /// Set maximum tokens for responses
    #[arg(long)]
    max_tokens: Option<usize>,
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
    default_style: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), ProviderConfig::default());

        Config {
            active_provider: "openai".to_string(),
            providers,
            max_tokens: Some(300),
            default_style: None,
        }
    }
}

impl Config {
    fn get_active_provider_config(&self) -> Result<(&String, &ProviderConfig)> {
        let provider = &self.active_provider;
        let config = self.providers.get(provider)
            .ok_or_else(|| format!("No configuration found for provider: {}", provider))?;

        if config.api_key.is_empty() {
            return Err(format!("API key not set for provider: {}. Run 'sage config -p {} -k <your-api-key>'", provider, provider).into());
        }

        Ok((provider, config))
    }

    fn set_provider(&mut self, provider: &str, api_key: Option<String>, model: Option<String>) -> Result<()> {
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

    fn update_key(&mut self, provider: &str, api_key: &str) -> Result<()> {
        let config = self.providers.entry(provider.to_string())
            .or_insert_with(ProviderConfig::default);

        config.api_key = api_key.to_string();
        Ok(())
    }

    #[allow(dead_code)]
    fn set_default_style(&mut self, style: Option<&CommitStyle>) -> Result<()> {
        self.default_style = style.map(|s| format!("{:?}", s).to_lowercase());
        Ok(())
    }

    fn set_max_tokens(&mut self, tokens: usize) -> Result<()> {
        self.max_tokens = Some(tokens);
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

/// Command execution context
struct Context {
    runtime: Runtime,
    verbose: bool,
}

impl Context {
    fn new(verbose: bool) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| format!("Failed to initialize async runtime: {}", e))?;
        Ok(Context { runtime, verbose })
    }

    fn log(&self, message: &str) {
        if self.verbose {
            println!("{}", message.blue());
        }
    }

    #[allow(dead_code)]
    fn log_always(&self, message: &str) {
        println!("{}", message);
    }

    fn run_async<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime.block_on(future)
    }
}

fn main() -> std::result::Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Check if we're in a git repository first
    if !is_git_repo() {
        eprintln!("{}", "Error: Not in a git repository".red());
        exit(1);
    }

    let context = Context::new(cli.verbose)?;

    match &cli.command {
        Some(Commands::Config(args)) => {
            handle_config_command(args)?;
        },
        Some(Commands::Use { provider }) => {
            use_provider(provider)?;
        },
        Some(Commands::Diff { files, all }) => {
            show_diff_command(files, *all)?;
        },
        None => {
            context.run_async(async {
                run_commit_flow(&context, &cli).await
            })?;
        }
    }

    Ok(())
}

fn handle_config_command(args: &ConfigArgs) -> Result<()> {
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
    } else if let Some(tokens) = args.max_tokens {
        config.set_max_tokens(tokens)?;
        println!("{}", format!("Max tokens set to: {}", tokens).green());
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

fn use_provider(provider: &str) -> Result<()> {
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

fn show_diff_command(files: &[String], all: bool) -> Result<()> {
    if !files.is_empty() {
        stage_files(files)?;
    }

    let diff = get_diff(all)?;
    let files_changed = get_files_changed(all)?;

    if diff.trim().is_empty() {
        println!("{}", "No changes to display.".yellow());
        return Ok(());
    }

    show_changes(&diff, &files_changed)
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

    if let Some(style) = &config.default_style {
        println!("\n  Default commit style: {}", style);
    }

    println!("  Max tokens: {}", config.max_tokens.unwrap_or(300));
}

fn get_config_path() -> Result<String> {
    let home_dir = env::var("HOME").map_err(|_| SageError("Could not find home directory".into()))?;
    Ok(Path::new(&home_dir).join(CONFIG_FILE).to_string_lossy().to_string())
}

fn load_config(config_path: &str) -> Result<Config> {
    let path = Path::new(config_path);
    if !path.exists() {
        return Ok(Config::default());
    }

    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&config_str)?;

    Ok(config)
}

fn save_config(config: &Config, config_path: &str) -> Result<()> {
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, config_json)?;
    Ok(())
}

fn get_diff(all: bool) -> Result<String> {
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

fn get_files_changed(all: bool) -> Result<String> {
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

fn stage_files(files: &[String]) -> Result<()> {
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

fn stage_all_files() -> Result<()> {
    let status = Command::new("git")
        .args(["add", "--all"])
        .status()?;

    if !status.success() {
        return Err("Failed to stage all files".into());
    }

    Ok(())
}

fn has_staged_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()?;

    Ok(!output.success())
}

fn commit_changes(message: &str, amend: bool) -> Result<()> {
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

fn push_changes(force: bool) -> Result<()> {
    println!("{}", "Pushing changes...".blue());

    let mut args = vec!["push"];
    if force {
        args.push("--force");
    }

    let status = Command::new("git")
        .args(args)
        .status()?;

    if !status.success() {
        return Err("Failed to push changes".into());
    }

    println!("{}", "Changes pushed successfully!".green());
    Ok(())
}

async fn run_commit_flow(context: &Context, cli: &Cli) -> Result<()> {
    // Handle staged/unstaged files
    let staged_initially = has_staged_changes()?;

    if cli.all || !cli.files.is_empty() {
        if cli.all {
            context.log("Staging all changes...");
            stage_all_files()?;
        } else if !cli.files.is_empty() {
            context.log(&format!("Staging specified files: {:?}...", cli.files));
            stage_files(&cli.files)?;
        }
    }

    // Check if there are staged changes now
    if !has_staged_changes()? {
        if staged_initially || !cli.files.is_empty() || cli.all {
            return Err("No staged changes found. Nothing to commit.".into());
        } else {
            // Only show this more detailed message when they haven't tried to stage anything
            return Err("No staged changes found. Use --all to include unstaged changes or specify files to stage.".into());
        }
    }

    // If manual message is provided, use it directly
    if let Some(message) = &cli.message {
        if cli.dry_run {
            println!("{}", "Would commit with message:".blue());
            println!("{}", message);
        } else {
            commit_changes(message, cli.amend)?;
            println!("{}", "Changes committed successfully!".green());

            if cli.push {
                push_changes(cli.force_push)?;
            }
        }
        return Ok(());
    }

    // Get git changes for AI analysis
    context.log("Analyzing git repository changes...");

    let start = Instant::now();
    let diff = get_diff(false)?;
    let files_changed = get_files_changed(false)?;

    if diff.trim().is_empty() {
        return Err("No changes detected. Nothing to commit.".into());
    }

    if cli.show_diff {
        show_changes(&diff, &files_changed)?;
    }

    let truncated_diff = smart_truncate_diff(&diff);

    // Generate prompt for AI
    let context_str = cli.context.as_deref().unwrap_or("");

    // Get commit style instructions based on CLI option or config
    let style_instructions = get_style_instructions(cli.style)?;

    let prompt = format!(
        "Generate a concise and descriptive git commit message for the following changes.\n\
        Follow the conventional commits format (type: description).\n\
        {}\n\
        Additional context: {}\n\
        Files changed:\n{}\n\nDiff:\n{}",
        style_instructions, context_str, files_changed, truncated_diff
    );

    context.log("Generating commit message using AI...");

    // Call the AI service
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let message = call_ai(&config, &prompt).await?;

    // Show timing information if verbose
    let elapsed = start.elapsed();
    context.log(&format!("Generation took {:.2}s", elapsed.as_secs_f32()));

    println!("\n{}", "Generated commit message:".green().bold());
    println!("{}", sanitize_commit_message(&message));


    if cli.dry_run {
        println!("\n{}", "Dry run - changes were not committed.".yellow());
    } else {
        // Skip confirmation if --yes flag is used
        let should_commit = if cli.yes {
            true
        } else {
            confirm_commit()?
        };

        if should_commit {
            commit_changes(&message, cli.amend)?;
            println!("{}", "Changes committed successfully!".green());

            if cli.push {
                push_changes(cli.force_push)?;
            }
        } else {
            println!("{}", "Commit aborted.".yellow());
        }
    }

    Ok(())
}

fn get_style_instructions(style: Option<CommitStyle>) -> Result<String> {
    match style {
        Some(CommitStyle::Standard) => {
            Ok("Use standard conventional commits format: 'type(scope): description'".to_string())
        },
        Some(CommitStyle::Detailed) => {
            Ok("Create a detailed multi-line commit message with a summary line followed by empty line and bullet points explaining the changes".to_string())
        },
        Some(CommitStyle::Short) => {
            Ok("Create an extremely concise one-line commit message".to_string())
        },
        Some(CommitStyle::Emoji) => {
            Ok("Include appropriate emojis in the commit message".to_string())
        },
        None => {
            Ok("Use standard conventional commits format: 'type(scope): description'".to_string())
        }
    }
}

fn show_changes(diff: &str, files_changed: &str) -> Result<()> {
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

fn confirm_commit() -> Result<bool> {
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
            if i < chunk_lines.len() {
                important_parts.push_str(chunk_lines[i]);
                important_parts.push('\n');
            }
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

async fn call_ai(config: &Config, prompt: &str) -> Result<String> {
    let (provider_name, provider_config) = config.get_active_provider_config()?;

    match provider_name.as_str() {
        "openai" => call_openai_api(provider_config, prompt, config.max_tokens).await,
        "claude" => call_claude_api(provider_config, prompt, config.max_tokens).await,
        _ => Err(format!("Unsupported provider: {}", provider_name).into())
    }
}

async fn call_openai_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<String> {
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
        let message = choice.message.content.trim().to_string();
        Ok(sanitize_commit_message(&message))
    } else {
        Err("No response from OpenAI API".into())
    }
}

fn sanitize_commit_message(message: &str) -> String {
    let re = Regex::new(r"```\w*\s*([\s\S]*?)\s*```").unwrap();
    if let Some(captures) = re.captures(message) {
        if let Some(inner_text) = captures.get(1) {
            return inner_text.as_str().trim().to_string();
        }
    }

    message.trim().to_string()
}

async fn call_claude_api(provider_config: &ProviderConfig, prompt: &str, max_tokens: Option<usize>) -> Result<String> {
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