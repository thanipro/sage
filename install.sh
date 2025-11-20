#!/bin/bash

set -e

REPO_URL="https://github.com/thanipro/sage.git"
INSTALL_DIR="$HOME/.local/sage"
BIN_DIR="$HOME/.cargo/bin"

echo "================================================"
echo "  sage - AI-powered Git Commit Message Tool"
echo "================================================"
echo ""

# Check if Rust and Cargo are installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust and Cargo are required but not installed."
    echo "Please install from https://rustup.rs/"
    echo ""
    echo "Quick install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo "Error: Git is required but not installed."
    exit 1
fi

# Clone or update repository
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing installation..."
    cd "$INSTALL_DIR"
    git pull
else
    echo "Cloning sage repository..."
    git clone "$REPO_URL" "$INSTALL_DIR"
    cd "$INSTALL_DIR"
fi

# Build the project
echo ""
echo "Building sage..."
cargo build --release

# Determine install location
if [ -d "$BIN_DIR" ]; then
    TARGET_DIR="$BIN_DIR"
elif [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    TARGET_DIR="/usr/local/bin"
else
    TARGET_DIR="$HOME/.local/bin"
    mkdir -p "$TARGET_DIR"
fi

# Copy binary to install location
echo "Installing to $TARGET_DIR/sage..."
cp "target/release/sage" "$TARGET_DIR/"
chmod +x "$TARGET_DIR/sage"

# Check if install location is in PATH
if [[ ":$PATH:" != *":$TARGET_DIR:"* ]]; then
    echo ""
    echo "Warning: $TARGET_DIR is not in your PATH!"
    echo "Add this line to your shell configuration:"
    echo "  export PATH=\"\$PATH:$TARGET_DIR\""
fi

# Detect shell and setup completions
echo ""
echo "Setting up shell completions..."

SHELL_NAME=$(basename "$SHELL")
COMPLETION_INSTALLED=false

case "$SHELL_NAME" in
    zsh)
        COMP_DIR="$HOME/.zfunc"
        mkdir -p "$COMP_DIR"
        "$TARGET_DIR/sage" completion zsh > "$COMP_DIR/_sage"

        # Add to .zshrc if not already present
        ZSHRC="$HOME/.zshrc"
        if ! grep -q "fpath=.*\.zfunc" "$ZSHRC" 2>/dev/null; then
            echo "" >> "$ZSHRC"
            echo "# sage completions" >> "$ZSHRC"
            echo "fpath=(~/.zfunc \$fpath)" >> "$ZSHRC"
            echo "autoload -Uz compinit && compinit" >> "$ZSHRC"
            echo "Added completion setup to $ZSHRC"
        fi
        COMPLETION_INSTALLED=true
        echo "✓ ZSH completions installed to $COMP_DIR/_sage"
        ;;

    bash)
        COMP_DIR="$HOME/.local/share/bash-completion/completions"
        mkdir -p "$COMP_DIR"
        "$TARGET_DIR/sage" completion bash > "$COMP_DIR/sage"
        COMPLETION_INSTALLED=true
        echo "✓ Bash completions installed to $COMP_DIR/sage"
        ;;

    fish)
        COMP_DIR="$HOME/.config/fish/completions"
        mkdir -p "$COMP_DIR"
        "$TARGET_DIR/sage" completion fish > "$COMP_DIR/sage.fish"
        COMPLETION_INSTALLED=true
        echo "✓ Fish completions installed to $COMP_DIR/sage.fish"
        ;;

    *)
        echo "Shell '$SHELL_NAME' detected. You can manually generate completions with:"
        echo "  sage completion <shell> > <output-file>"
        ;;
esac

echo ""
echo "================================================"
echo "  Installation Complete!"
echo "================================================"
echo ""

# Prompt for configuration
echo "Would you like to configure your API key now? [Y/n]"
read -r CONFIGURE

if [[ "$CONFIGURE" =~ ^[Yy]$ ]] || [[ -z "$CONFIGURE" ]]; then
    echo ""
    echo "Select your AI provider:"
    echo "  1) OpenAI (GPT-4)"
    echo "  2) Claude (Anthropic)"
    echo -n "Enter choice [1-2]: "
    read -r PROVIDER_CHOICE

    case "$PROVIDER_CHOICE" in
        1)
            PROVIDER="openai"
            echo ""
            echo "Enter your OpenAI API key:"
            echo "(Get one at https://platform.openai.com/api-keys)"
            ;;
        2)
            PROVIDER="claude"
            echo ""
            echo "Enter your Claude API key:"
            echo "(Get one at https://console.anthropic.com/)"
            ;;
        *)
            echo "Invalid choice. Skipping configuration."
            PROVIDER=""
            ;;
    esac

    if [ -n "$PROVIDER" ]; then
        echo -n "API Key: "
        read -r API_KEY

        if [ -n "$API_KEY" ]; then
            "$TARGET_DIR/sage" config -p "$PROVIDER" -k "$API_KEY"
            echo ""
            echo "✓ Configuration saved!"
        fi
    fi
else
    echo ""
    echo "Skipping configuration. You can configure later with:"
    echo "  sage config -p openai -k your_api_key"
fi

echo ""
echo "Quick Start:"
echo "  1. Navigate to a git repository"
echo "  2. Make some changes"
echo "  3. Run: sage -a"
echo ""
echo "For help: sage --help"
echo "View config: sage config -s"
echo ""

if [ "$COMPLETION_INSTALLED" = true ]; then
    echo "Restart your shell or run 'source ~/.$SHELL_NAME"rc"' to enable completions"
    echo ""
fi

echo "Happy committing! 🚀"
