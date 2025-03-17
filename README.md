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
sage --provider openai --key your_api_key
```

## Usage

```bash
# Basic usage (stages + analyzes + commits in one step)
sage src/*.rs

# Add message context to help the AI
sage --context "Refactoring authentication flow" src/auth/

# Use already staged changes
sage

# Preview without committing
sage --dry-run

# Include unstaged changes in analysis
sage --all

# Provide your own message (skips AI)
sage -m "fix: resolve login issue"

# Amend previous commit
sage --amend

# Show diff before committing
sage --show-diff
```

## Shell Integration

Add this to your `.bashrc` or `.zshrc`:

```bash
# Alias for completely replacing git commit
alias git-commit='sage'

# Function for committing all changes
gca() {
  sage --all "$@"
}
```

## How It Works

1. **Stage files** if specified (or uses already staged changes)
2. **Extract git diff** and changed file information
3. **Submit to AI** for analysis via OpenAI or Claude APIs
4. **Generate commit message** following conventional commits format
5. **Interactive confirmation** to commit, edit, or abort
6. **Commit changes** with the generated message

## Configuration

Configuration is stored in `~/.sage-config.json`.

```bash
# Set up API provider and key
sage --provider openai --key your_api_key
```

## Requirements

- Rust 1.65+
- Git
- OpenAI or Claude API key

## License

MIT