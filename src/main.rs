mod error;
mod cli;
mod config;
mod git;
mod ai;
mod prompts;

use std::process::exit;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::time::Instant;
use colored::Colorize;
use clap::{Parser, CommandFactory};
use clap_complete::{generate, Shell};
use indicatif::{ProgressBar, ProgressStyle};

use error::{Result, SageError};
use cli::{Cli, Commands, ConfigArgs};
use config::{get_config_path, load_config, save_config};
use git::{
    is_git_repo, get_diff, get_files_changed, stage_files, stage_all_files,
    has_staged_changes, commit_changes, push_changes, show_changes, smart_truncate_diff,
    get_current_branch, create_and_checkout_branch, branch_exists
};
use ai::call_ai;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if !is_git_repo() {
        eprintln!("{}", "Error: Not in a git repository".red());
        exit(1);
    }

    if let Err(e) = run_app(cli).await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        exit(1);
    }
}

async fn run_app(cli: Cli) -> Result<()> {
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
        Some(Commands::Branch { files, all, context, yes, verbose }) => {
            run_branch_flow(files, *all, context.as_deref(), *yes, *verbose).await?;
        },
        Some(Commands::Completion { shell }) => {
            generate_completions(*shell);
        },
        None => {
            run_commit_flow(&cli).await?;
        }
    }

    Ok(())
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut io::stdout());
}

fn handle_config_command(args: &ConfigArgs) -> Result<()> {
    let config_path = get_config_path()?;
    let mut config = load_config(&config_path)?;

    if args.show {
        config.show();
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
            return Err(SageError::InvalidInput("API key required with --update-key".to_string()));
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
        config.show();
    }

    Ok(())
}

fn use_provider(provider: &str) -> Result<()> {
    let config_path = get_config_path()?;
    let mut config = load_config(&config_path)?;

    if !config.providers.contains_key(provider) {
        return Err(SageError::ConfigProviderNotConfigured {
            provider: provider.to_string()
        });
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

async fn run_commit_flow(cli: &Cli) -> Result<()> {
    if cli.all || !cli.files.is_empty() {
        if cli.all {
            if cli.verbose {
                println!("{}", "Staging all changes...".blue());
            }
            stage_all_files()?;
        } else if !cli.files.is_empty() {
            if cli.verbose {
                println!("{}", format!("Staging specified files: {:?}...", cli.files).blue());
            }
            stage_files(&cli.files)?;
        }
    }

    if !has_staged_changes()? {
        return Err(SageError::GitNoStagedChanges);
    }

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

    if cli.verbose {
        println!("{}", "Analyzing git repository changes...".blue());
    }

    let start = Instant::now();
    let diff = get_diff(false)?;
    let files_changed = get_files_changed(false)?;

    if diff.trim().is_empty() {
        return Err(SageError::GitNoChanges);
    }

    if cli.show_diff {
        show_changes(&diff, &files_changed)?;
    }

    let truncated_diff = smart_truncate_diff(&diff);

    let context_str = cli.context.as_deref().unwrap_or("");
    let prompt = prompts::build_commit_prompt(
        cli.style,
        context_str,
        &files_changed,
        &truncated_diff,
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );
    spinner.set_message("Generating commit message using AI...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let response = call_ai(&config, &prompt).await?;

    spinner.finish_and_clear();

    if cli.verbose {
        let elapsed = start.elapsed();
        println!("{}", format!("Generation took {:.2}s", elapsed.as_secs_f32()).blue());
    }

    println!("\n{}", "Generated commit message:".green().bold());
    println!("{}", response.message);

    // Show token usage
    if cli.verbose {
        println!("\n{}", format!("Tokens: {} in / {} out / {} total",
            response.usage.input_tokens,
            response.usage.output_tokens,
            response.usage.total_tokens
        ).cyan());
    }


    if cli.dry_run {
        println!("\n{}", "Dry run - changes were not committed.".yellow());
    } else {
        let (should_commit, final_message) = if cli.yes {
            (true, response.message.clone())
        } else {
            confirm_commit(&response.message)?
        };

        if should_commit {
            commit_changes(&final_message, cli.amend)?;
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

async fn run_branch_flow(
    files: &[String],
    all: bool,
    context: Option<&str>,
    yes: bool,
    verbose: bool,
) -> Result<()> {
    let current_branch = get_current_branch()?;
    if verbose {
        println!("{}", format!("Current branch: {}", current_branch).blue());
    }

    if all || !files.is_empty() {
        if all {
            if verbose {
                println!("{}", "Staging all changes...".blue());
            }
            stage_all_files()?;
        } else if !files.is_empty() {
            if verbose {
                println!("{}", format!("Staging specified files: {:?}...", files).blue());
            }
            stage_files(files)?;
        }
    }

    if verbose {
        println!("{}", "Analyzing changes...".blue());
    }

    let diff = get_diff(true)?;
    let files_changed = get_files_changed(true)?;

    if diff.trim().is_empty() && files_changed.trim().is_empty() {
        return Err(SageError::GitNoChanges);
    }

    let truncated_diff = smart_truncate_diff(&diff);

    let context_str = context.unwrap_or("");
    let prompt = prompts::build_branch_prompt(
        context_str,
        &files_changed,
        &truncated_diff,
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );
    spinner.set_message("Generating branch name using AI...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let response = call_ai(&config, &prompt).await?;

    spinner.finish_and_clear();

    let branch_name = sanitize_branch_name(&response.message);

    println!("\n{}", "Generated branch name:".green().bold());
    println!("{}", branch_name);

    if verbose {
        println!("\n{}", format!("Tokens: {} in / {} out / {} total",
            response.usage.input_tokens,
            response.usage.output_tokens,
            response.usage.total_tokens
        ).cyan());
    }

    let (should_create, final_branch_name) = if yes {
        (true, branch_name.clone())
    } else {
        confirm_branch_name(&branch_name)?
    };

    if should_create {
        if branch_exists(&final_branch_name)? {
            return Err(SageError::InvalidInput(
                format!("Branch '{}' already exists", final_branch_name)
            ));
        }

        create_and_checkout_branch(&final_branch_name)?;
    } else {
        println!("{}", "Branch creation aborted.".yellow());
    }

    Ok(())
}

fn sanitize_branch_name(name: &str) -> String {
    let mut result = name.trim().to_string();

    result = result.replace(" ", "-");
    result = result.replace("_", "-");
    result = result.replace("**", "");
    result = result.replace("*", "");
    result = result.replace("`", "");

    result = result.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '/')
        .collect();

    result = result.to_lowercase();

    while result.contains("--") {
        result = result.replace("--", "-");
    }

    result = result.trim_matches('-').to_string();

    result
}

fn confirm_branch_name(branch_name: &str) -> Result<(bool, String)> {
    print!("\nCreate this branch? [Y/n/e for edit] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "e" {
        print!("Enter branch name: ");
        io::stdout().flush()?;

        let mut edited_name = String::new();
        io::stdin().read_line(&mut edited_name)?;
        let edited_name = sanitize_branch_name(&edited_name);

        if edited_name.is_empty() {
            return Ok((false, branch_name.to_string()));
        }

        Ok((true, edited_name))
    } else {
        let should_create = input.is_empty() || input == "y";
        Ok((should_create, branch_name.to_string()))
    }
}

fn confirm_commit(message: &str) -> Result<(bool, String)> {
    print!("\nCommit with this message? [Y/n/e for edit] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "e" {
        let temp_file = "/tmp/sage_commit_msg";
        fs::write(temp_file, message)?;

        let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = std::process::Command::new(&editor)
            .arg(temp_file)
            .status()?;

        if !status.success() {
            return Err(SageError::EditorFailed);
        }

        let edited_message = fs::read_to_string(temp_file)?;
        let edited_message = edited_message.trim().to_string();

        if edited_message.is_empty() {
            return Ok((false, message.to_string()));
        }

        Ok((true, edited_message))
    } else {
        let should_commit = input.is_empty() || input == "y";
        Ok((should_commit, message.to_string()))
    }
}
