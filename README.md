# sage

AI-powered Git commit message and branch name generator. Analyze your code changes and generate meaningful commit messages following conventional commits format automatically.

## Overview

sage is a command-line tool that integrates with your Git workflow to automatically generate commit messages and branch names by analyzing your code changes using AI. It supports multiple AI providers (OpenAI and Claude) and provides an interactive workflow for reviewing and editing generated messages.

## Features

- **Automatic commit message generation** using AI analysis of your git diff
- **Branch name generation** based on your changes
- **Multiple commit styles** (standard, detailed, short)
- **Interactive mode** to review, edit, or abort before committing
- **Context-aware** generation with optional user-provided context
- **Multiple AI providers** (OpenAI GPT-4, Claude)
- **Shell completions** for bash, zsh, fish, and PowerShell
- **Loading animations** for better UX during AI operations
- **Secure input validation** to prevent command injection
- **Smart diff truncation** for large changesets
- **One-command workflow** to stage, generate, and commit

## Requirements

- Rust 1.65 or higher
- Git 2.0 or higher
- API key for OpenAI or Claude

## Installation

### Fresh Installation

```bash
git clone https://github.com/thanipro/sage.git
cd sage
./install.sh
```

The install script will:
1. Build the release binary
2. Install to `~/.cargo/bin`, `/usr/local/bin`, or `~/.local/bin`
3. Make the binary executable
4. Check if the install location is in your PATH

### Upgrading from Previous Version

If you already have sage installed:

```bash
cd sage
git pull
cargo build --release
cp target/release/sage ~/.cargo/bin/  # or wherever your current sage is installed
```

To find where your current sage is installed:

```bash
which sage
```

### Manual Installation

```bash
cargo install --path .
```

## Quick Start

### 1. Configure API Provider

```bash
sage config -p openai -k your_openai_api_key
```

Or for Claude:

```bash
sage config -p claude -k your_claude_api_key
```

### 2. Make Changes and Commit

```bash
# Make some changes to your files
echo "new feature" >> src/main.rs

# Stage and generate commit message in one command
sage src/main.rs

# Or stage all changes
sage -a

# Or use already staged changes
git add .
sage
```

### 3. Review and Confirm

The tool will show you the generated commit message and prompt:

```
Generated commit message:
feat(main): add new feature implementation

Commit with this message? [Y/n/e for edit]
```

- Press Enter or `y` to commit
- Press `n` to abort
- Press `e` to edit the message in your editor

## Usage

### Basic Commands

```bash
# Commit with staged changes
sage

# Stage specific files and commit
sage src/*.rs

# Stage all changes and commit
sage -a

# Add context to help AI generate better messages
sage -c "Refactoring authentication" src/auth.rs

# Preview without committing (dry run)
sage -d

# Show diff before generating message
sage -s

# Skip confirmation prompt
sage -y

# Commit with manual message (skip AI)
sage -m "fix: resolve login bug"

# Amend previous commit
sage --amend

# Commit and push
sage -p

# Force push (with --push)
sage -p -f

# Verbose output with timing and token info
sage -v
```

### Subcommands

#### config - Configure API Settings

```bash
# Set provider and API key
sage config -p openai -k your_api_key

# Set custom model
sage config -p openai -k your_api_key --model gpt-4-turbo

# Update API key for existing provider
sage config --update-key openai -k new_api_key

# Update model for active provider
sage config --model gpt-4-turbo

# Set max tokens for responses
sage config --max-tokens 500

# Show current configuration
sage config -s
```

#### use - Switch Between Providers

```bash
# Switch to Claude
sage use claude

# Switch to OpenAI
sage use openai
```

#### diff - Show Changes Without Committing

```bash
# Show staged changes
sage diff

# Show all changes (staged and unstaged)
sage diff -a

# Show diff for specific files
sage diff src/*.rs
```

#### branch - Create AI-Generated Branch Names

```bash
# Generate branch name from staged changes
sage branch

# Stage files and generate branch name
sage branch src/*.rs

# Stage all and generate branch name
sage branch -a

# Add context for better branch names
sage branch -c "Adding user authentication"

# Skip confirmation prompt
sage branch -y

# Verbose output
sage branch -v
```

The tool will analyze your changes and generate a branch name following the format:
- `feature/add-user-auth`
- `bugfix/fix-login-error`
- `refactor/simplify-api`
- `docs/update-readme`

#### completion - Generate Shell Completions

```bash
# Generate bash completions
sage completion bash > /usr/local/etc/bash_completion.d/sage

# Generate zsh completions
sage completion zsh > ~/.zfunc/_sage

# Generate fish completions
sage completion fish > ~/.config/fish/completions/sage.fish

# Generate PowerShell completions
sage completion powershell > sage.ps1
```

After generating completions, restart your shell or source the completion file.

### Commit Message Styles

Use the `-t` or `--style` flag to control message format:

```bash
# Standard conventional commits (default)
sage -t standard
# Output: feat(auth): add user authentication

# Detailed multi-line messages
sage -t detailed
# Output:
# feat: add user authentication
#
# - Implement JWT token validation
# - Add login endpoint
# - Create user session management

# Short one-line messages (max 50 chars)
sage -t short
# Output: add user auth
```

## Configuration

Configuration is stored in `~/.sage-config.json`:

```json
{
  "active_provider": "openai",
  "providers": {
    "openai": {
      "api_key": "sk-...",
      "model": "gpt-4-turbo"
    },
    "claude": {
      "api_key": "sk-ant-...",
      "model": "claude-3-sonnet-20240229"
    }
  },
  "max_tokens": 300,
  "default_style": null
}
```

### Supported Models

**OpenAI:**
- gpt-4-turbo (default)
- gpt-4
- gpt-3.5-turbo

**Claude:**
- claude-3-sonnet-20240229 (default)
- claude-3-opus-20240229
- claude-3-haiku-20240307

## Command-Line Options

### Global Flags

| Flag | Long Form | Description |
|------|-----------|-------------|
| `-a` | `--all` | Stage all changes before committing |
| `-m` | `--message <MSG>` | Use manual commit message (skip AI) |
| `-c` | `--context <TEXT>` | Add context to help AI generate better messages |
| `-d` | `--dry-run` | Preview generated message without committing |
| `-s` | `--show-diff` | Show diff before generating message |
| `-y` | `--yes` | Skip confirmation prompt |
| `-v` | `--verbose` | Show detailed progress, timing, and token usage |
| `-p` | `--push` | Push changes after committing |
| `-f` | `--force-push` | Force push (requires --push) |
| `-t` | `--style <STYLE>` | Commit message style (standard/detailed/short) |
| | `--amend` | Amend the previous commit |

### Subcommand-Specific Options

**config:**
- `-p, --provider <NAME>` - Set API provider
- `-k, --key <KEY>` - Set API key
- `--update-key <PROVIDER>` - Update key for specific provider
- `--model <MODEL>` - Set model name
- `--max-tokens <NUM>` - Set maximum tokens
- `-s, --show` - Show current configuration

**branch:**
- Same as global flags: `-a`, `-c`, `-y`, `-v`
- Plus file arguments for staging

**diff:**
- `-a, --all` - Show unstaged changes
- Plus file arguments to diff

## Examples

### Common Workflows

```bash
# Quick commit with context
sage -a -c "Fix authentication bug"

# Stage all, commit, and push
sage -a -p

# Generate detailed commit message
sage -a -t detailed

# Preview message before committing
sage -a -d

# Create feature branch and commit
sage branch -a -c "Add user profile page"
# ... make changes ...
sage -a -c "Implement user profile"

# Amend last commit with new changes
sage --amend src/*.rs

# Commit specific files with context
sage -c "Refactor validation logic" src/validation.rs src/utils.rs
```

### Integration with Git Workflows

```bash
# Feature development workflow
git checkout main
git pull
sage branch -a -c "User profile feature"
# ... make changes ...
sage -a -c "Initial implementation"
# ... more changes ...
sage -a -c "Add tests"
sage -p  # Push when ready

# Bugfix workflow
sage branch -a -c "Fix login error"
# ... fix bug ...
sage -a -c "Resolve authentication issue"
sage -p

# Review before committing
sage -a -s -d  # Show diff and preview message
sage -a        # Commit if satisfied
```

## Shell Aliases

Add these to your `.bashrc` or `.zshrc` for faster workflows:

```bash
# Quick sage commands
alias sg='sage'
alias sga='sage -a'
alias sgp='sage -p'
alias sgap='sage -a -p'
alias sgd='sage -d'
alias sgv='sage -v'

# With context
alias sgc='sage -c'
alias sgac='sage -a -c'

# Branch creation
alias sgb='sage branch'
alias sgba='sage branch -a'
```

## How It Works

1. **File Staging**: Files specified as arguments are staged using `git add`. If no files specified, uses already staged changes.

2. **Diff Extraction**: Runs `git diff --cached` to get staged changes and `git status --porcelain` to get file list.

3. **Smart Truncation**: Large diffs are intelligently truncated to stay within AI token limits while preserving important context.

4. **AI Analysis**: Sends diff and file changes to configured AI provider (OpenAI or Claude) with a carefully crafted prompt that enforces:
   - Conventional commits format
   - Plain text output (no markdown)
   - Focus on what changed and why
   - Appropriate scope and type

5. **Sanitization**: AI response is sanitized to remove any markdown formatting that might slip through.

6. **Interactive Review**: User can:
   - Accept the message (press Enter or 'y')
   - Edit in their configured editor (press 'e')
   - Abort the commit (press 'n')

7. **Commit**: Executes `git commit` with the final message.

8. **Optional Push**: If `-p` flag is used, pushes changes to remote.

## Security

sage includes several security features:

- **Input Validation**: File paths are validated to prevent command injection and path traversal attacks
- **No Shell Interpolation**: Uses Rust's `Command` API with separate arguments (no shell=True)
- **API Key Storage**: Keys stored in home directory config file with restricted permissions
- **Sanitization**: AI responses are sanitized to prevent injection of special characters

## Troubleshooting

### API Key Not Set

```
Error: API key not set for provider: openai

Tip: Run 'sage config -p openai -k <your-api-key>'
```

Solution: Configure your API key as shown.

### No Staged Changes

```
Error: No staged changes found

Tip: Stage files with 'sage <files>' or use 'sage --all' to stage all changes
```

Solution: Stage files using `sage file.rs` or `sage -a`.

### Network Errors

```
Error: Network error connecting to OpenAI: connection timeout
```

Solution: Check your internet connection and API endpoint availability.

### Authentication Failed

```
Error: Authentication failed for OpenAI

Tip: Verify your API key with 'sage config -s' and update if needed
```

Solution: Verify your API key is correct using `sage config -s`.

### Editor Not Found

```
Error: Failed to open editor

Tip: Set your EDITOR environment variable or use a different editor
```

Solution: Set your `EDITOR` environment variable:

```bash
export EDITOR=vim  # or nano, emacs, code, etc.
```

## Development

### Building from Source

```bash
git clone https://github.com/thanipro/sage.git
cd sage
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Project Structure

```
sage/
├── src/
│   ├── main.rs           # Entry point and CLI orchestration
│   ├── cli.rs            # Command-line argument definitions
│   ├── config.rs         # Configuration management
│   ├── error.rs          # Error types and handling
│   ├── git.rs            # Git operations
│   ├── prompts.rs        # AI prompt templates
│   └── ai/
│       ├── mod.rs        # AI provider interface
│       ├── openai.rs     # OpenAI implementation
│       └── claude.rs     # Claude implementation
├── Cargo.toml            # Rust dependencies
├── install.sh            # Installation script
└── README.md             # This file
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Acknowledgments

Built with Rust and powered by OpenAI GPT-4 and Anthropic Claude.
