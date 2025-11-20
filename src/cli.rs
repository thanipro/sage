use clap::{Parser, Args, Subcommand, ValueEnum};
use clap_complete::Shell;

const APP_VERSION: &str = "1.0.0";
const APP_NAME: &str = "sage";

#[derive(Parser)]
#[command(name = APP_NAME)]
#[command(author = "Author")]
#[command(version = APP_VERSION)]
#[command(about = "AI-powered Git Commit Message generator", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// File patterns to add (e.g. '*.rs', 'src/', etc.)
    #[arg(name = "FILES")]
    pub files: Vec<String>,

    /// Don't actually commit, just print the generated message
    #[arg(short, long)]
    pub dry_run: bool,

    /// Use all changes (staged + unstaged) for message generation
    #[arg(short, long)]
    pub all: bool,

    /// Commit message to use (skips AI generation)
    #[arg(short, long)]
    pub message: Option<String>,

    /// Add additional context to help AI generate better messages
    #[arg(short, long)]
    pub context: Option<String>,

    /// Show a diff of the changes before committing
    #[arg(short, long)]
    pub show_diff: bool,

    /// Amend the previous commit
    #[arg(long)]
    pub amend: bool,

    /// Be more verbose about what's happening
    #[arg(short, long)]
    pub verbose: bool,

    /// Push changes after commit
    #[arg(short, long)]
    pub push: bool,

    /// Force push when needed (with --push)
    #[arg(short = 'f', long)]
    pub force_push: bool,

    /// Skip confirmation prompt (auto-accept commit message)
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Style of commit message to generate
    #[arg(short = 't', long, value_enum)]
    pub style: Option<CommitStyle>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CommitStyle {
    /// Standard conventional commits format
    Standard,
    /// More detailed multi-line commit message
    Detailed,
    /// Very short one-line commit message
    Short,
}


#[derive(Subcommand)]
pub enum Commands {
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

    /// Create and checkout a new branch with AI-generated name
    Branch {
        /// Files to stage before analyzing
        #[arg(name = "FILES")]
        files: Vec<String>,

        /// Stage all changes before analyzing
        #[arg(short, long)]
        all: bool,

        /// Additional context to help AI generate better branch name
        #[arg(short, long)]
        context: Option<String>,

        /// Skip confirmation prompt (auto-accept branch name)
        #[arg(short = 'y', long = "yes")]
        yes: bool,

        /// Be more verbose about what's happening
        #[arg(short, long)]
        verbose: bool,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Set API provider (openai, claude, etc.)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Set API key
    #[arg(short, long)]
    pub key: Option<String>,

    /// Update API key for a specific provider
    #[arg(long)]
    pub update_key: Option<String>,

    /// Show current configuration
    #[arg(short, long)]
    pub show: bool,

    /// Set preferred model for the provider
    #[arg(long)]
    pub model: Option<String>,

    /// Set maximum tokens for responses
    #[arg(long)]
    pub max_tokens: Option<usize>,

    /// Launch interactive configuration wizard
    #[arg(short, long)]
    pub wizard: bool,

    /// Set preference: auto-push, auto-stage-all, show-diff, skip-confirmation, verbose
    #[arg(long)]
    pub set_pref: Option<String>,

    /// Value for preference (true/false)
    #[arg(long)]
    pub value: Option<bool>,
}
