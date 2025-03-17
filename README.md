# sage - Git Commit with AI

`sage` is a lightning-fast tool that uses AI to generate meaningful commit messages by analyzing your Git changes. Streamline your workflow with one command that stages, analyzes, and commits.

## Features

- ⚡ **One command workflow**: Stage files and commit in a single step
- 🧠 **AI-powered analysis**: Generates conventional commit messages automatically
- 🔄 **Interactive mode**: Preview, edit, or confirm generated messages
- 🔍 **Smart diff analysis**: Efficiently processes large changes
- 📝 **Custom context**: Add additional context to improve message quality
- 🚀 **Multiple AI providers**: Support for OpenAI and Claude

## Installation

```bash
# Clone and build
git clone https://github.com/yourusername/sage.git
cd sage
cargo install --path .

# Configure with your API key
sage config -p openai -k your_api_key
```

## Usage

```bash
# Basic usage (stages + analyzes + commits in one step)
sage src/*.rs

# Add message context to help the AI
sage -c "Refactoring authentication flow" src/auth/

# Use already staged changes
sage

# Preview without committing
sage -d

# Include unstaged changes in analysis
sage -a

# Provide your own message (skips AI)
sage -m "fix: resolve login issue"

# Amend previous commit
sage --amend

# Show diff before committing
sage -s

# Commit and push in one step
sage -p

# Combine flags for powerful workflows
sage -a -p -c "Fix authentication bugs"  # Stage all, add context, and push
```

## Shortcuts

All commands use short flags for faster workflows:

| Short Flag | Long Flag    | Description                     |
|------------|-------------|---------------------------------|
| `-a`       | `--all`     | Stage all changes              |
| `-p`       | `--push`    | Push changes after commit      |
| `-c`       | `--context` | Add context for AI             |
| `-m`       | `--message` | Provide manual commit message  |
| `-d`       | `--dry-run` | Preview without committing     |
| `-s`       | `--show-diff` | Show changes before commit   |
| `-v`       | `--verbose` | Show detailed progress         |

## Configuration

```bash
# Set up API provider and key
sage config -p openai -k your_api_key

# Show current configuration
sage config -s

# Switch to a different provider
sage use claude

# Update key for a specific provider
sage config --update-key openai -k new_api_key

# Set preferred model for a provider
sage config -p openai --model gpt-4-turbo
```

## Shell Integration

Add this to your `.bashrc` or `.zshrc`:

```bash
# Quick sage shortcuts
alias sg='sage'                    # Basic sage command
alias sga='sage -a'                # Stage all changes
alias sgp='sage -p'                # Commit and push
alias sgap='sage -a -p'            # Stage all, commit and push
alias sgm='sage -m'                # Commit with manual message
alias sgd='sage -s'                # Show diff before committing

# Common workflows
alias sgac='sage -a -c'            # Stage all with context
alias sgapc='sage -a -p -c'        # Stage all, push with context
```

## How It Works

1. **Stage files** if specified (or uses already staged changes)
2. **Extract git diff** and changed file information
3. **Submit to AI** for analysis via OpenAI or Claude APIs
4. **Generate commit message** following conventional commits format
5. **Interactive confirmation** to commit, edit, or abort
6. **Commit changes** with the generated message

## Requirements

- Rust 1.65+
- Git
- OpenAI or Claude API key

