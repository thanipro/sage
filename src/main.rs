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

    if args.wizard {
        run_config_wizard(&mut config)?;
        save_config(&config, &config_path)?;
        return Ok(());
    }

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
    } else if let Some(pref_key) = &args.set_pref {
        if let Some(value) = args.value {
            let normalized_key = pref_key.replace("-", "_");
            config.set_preference(&normalized_key, value)?;
            println!("{}", format!("Preference '{}' set to: {}", pref_key, value).green());
            updated = true;
        } else {
            return Err(SageError::InvalidInput("--value required with --set-pref".to_string()));
        }
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
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;

    let should_stage_all = cli.all || config.preferences.auto_stage_all.unwrap_or(false);
    let should_show_diff = cli.show_diff || config.preferences.show_diff.unwrap_or(false);
    let should_skip_confirm = cli.yes || config.preferences.skip_confirmation.unwrap_or(false);
    let is_verbose = cli.verbose || config.preferences.verbose.unwrap_or(false);
    let should_push = cli.push || config.preferences.auto_push.unwrap_or(false);

    if should_stage_all || !cli.files.is_empty() {
        if should_stage_all {
            if is_verbose {
                println!("{}", "Staging all changes...".blue());
            }
            stage_all_files()?;
        } else if !cli.files.is_empty() {
            if is_verbose {
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

            if should_push {
                push_changes(cli.force_push)?;
            }
        }
        return Ok(());
    }

    if is_verbose {
        println!("{}", "Analyzing git repository changes...".blue());
    }

    let start = Instant::now();
    let diff = get_diff(false)?;
    let files_changed = get_files_changed(false)?;

    if diff.trim().is_empty() {
        return Err(SageError::GitNoChanges);
    }

    if should_show_diff {
        show_changes(&diff, &files_changed)?;
    }

    let truncated_diff = smart_truncate_diff(&diff);

    let context_str = cli.context.as_deref().unwrap_or("");
    let commit_style = cli.style.or_else(|| {
        config.default_style.as_ref().and_then(|s| match s.as_str() {
            "standard" => Some(cli::CommitStyle::Standard),
            "detailed" => Some(cli::CommitStyle::Detailed),
            "short" => Some(cli::CommitStyle::Short),
            _ => None,
        })
    });

    let prompt = prompts::build_commit_prompt(
        commit_style,
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

    let response = call_ai(&config, &prompt).await?;

    spinner.finish_and_clear();

    if is_verbose {
        let elapsed = start.elapsed();
        println!("{}", format!("Generation took {:.2}s", elapsed.as_secs_f32()).blue());
    }

    println!("\n{}", "Generated commit message:".green().bold());
    println!("{}", response.message);

    if is_verbose {
        println!("\n{}", format!("Tokens: {} in / {} out / {} total",
            response.usage.input_tokens,
            response.usage.output_tokens,
            response.usage.total_tokens
        ).cyan());
    }


    if cli.dry_run {
        println!("\n{}", "Dry run - changes were not committed.".yellow());
    } else {
        let (should_commit, final_message) = if should_skip_confirm {
            (true, response.message.clone())
        } else {
            confirm_commit(&response.message)?
        };

        if should_commit {
            commit_changes(&final_message, cli.amend)?;
            println!("{}", "Changes committed successfully!".green());

            if should_push {
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

fn run_config_wizard(config: &mut config::Config) -> Result<()> {
    println!("{}", "Configuration Wizard".blue().bold());
    println!("");

    println!("Current Settings:");
    config.show();
    println!("");

    println!("{}", "Select what you'd like to configure:".blue());
    println!("  1) Default commit style");
    println!("  2) Auto-push after commit");
    println!("  3) Auto-stage all changes");
    println!("  4) Show diff by default");
    println!("  5) Skip confirmation prompts");
    println!("  6) Verbose mode");
    println!("  7) All preferences");
    println!("  0) Exit wizard");
    println!("");

    loop {
        print!("Enter choice [0-7]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "0" => break,
            "1" => configure_commit_style(config)?,
            "2" => configure_bool_pref(config, "auto_push", "Auto-push after commit")?,
            "3" => configure_bool_pref(config, "auto_stage_all", "Auto-stage all changes")?,
            "4" => configure_bool_pref(config, "show_diff", "Show diff by default")?,
            "5" => configure_bool_pref(config, "skip_confirmation", "Skip confirmation prompts")?,
            "6" => configure_bool_pref(config, "verbose", "Verbose mode")?,
            "7" => {
                configure_commit_style(config)?;
                configure_bool_pref(config, "auto_push", "Auto-push after commit")?;
                configure_bool_pref(config, "auto_stage_all", "Auto-stage all changes")?;
                configure_bool_pref(config, "show_diff", "Show diff by default")?;
                configure_bool_pref(config, "skip_confirmation", "Skip confirmation prompts")?;
                configure_bool_pref(config, "verbose", "Verbose mode")?;
                break;
            }
            _ => println!("{}", "Invalid choice".red()),
        }

        println!("");
        print!("Configure another option? [0-7, or 0 to exit]: ");
        io::stdout().flush()?;
    }

    println!("");
    println!("{}", "Configuration updated!".green().bold());
    Ok(())
}

fn configure_commit_style(config: &mut config::Config) -> Result<()> {
    println!("");
    println!("Select default commit style:");
    println!("  1) Conventional - single line conventional commits format");
    println!("  2) Detailed - multi-line with summary + bullet points (Git convention)");
    println!("  3) Short - concise one-liner");
    println!("  4) No default (ask each time)");
    print!("Choice [1-4]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => {
            config.default_style = Some("standard".to_string());
            println!("{}", "✓ Default style set to: conventional".green());
        }
        "2" => {
            config.default_style = Some("detailed".to_string());
            println!("{}", "✓ Default style set to: detailed".green());
        }
        "3" => {
            config.default_style = Some("short".to_string());
            println!("{}", "✓ Default style set to: short".green());
        }
        "4" => {
            config.default_style = None;
            println!("{}", "✓ No default style (will prompt each time)".green());
        }
        _ => println!("{}", "Invalid choice, keeping current setting".yellow()),
    }

    Ok(())
}

fn configure_bool_pref(config: &mut config::Config, key: &str, description: &str) -> Result<()> {
    println!("");
    println!("{}: [y/n]", description);
    print!("Enable? ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    let value = match input.as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => {
            println!("{}", "Invalid input, keeping current setting".yellow());
            return Ok(());
        }
    };

    config.set_preference(key, value)?;
    println!("{}", format!("✓ {} {}", description, if value { "enabled" } else { "disabled" }).green());

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
