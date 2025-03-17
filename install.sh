#!/bin/bash

set -e

echo "Installing sage - Git Commit with AI..."

# Check if Rust and Cargo are installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust and Cargo are required but not installed."
    echo "Please install from https://rustup.rs/"
    exit 1
fi

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo "Error: Git is required but not installed."
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Build the project
echo "Building sage..."
cargo build --release

# Determine install location
INSTALL_DIR="$HOME/.local/bin"
if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="$HOME/.cargo/bin"
fi

# Create directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Copy binary to install location
echo "Installing to $INSTALL_DIR/sage..."
cp "target/release/sage" "$INSTALL_DIR/"

# Make executable
chmod +x "$INSTALL_DIR/sage"

# Check if install location is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "Warning: $INSTALL_DIR is not in your PATH!"
    echo "You may need to add it to your shell configuration:"
    echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
    echo "  source ~/.bashrc"
fi

echo "Installation complete! Run 'sage --help' to get started."
echo ""
echo "First, configure your API key:"
echo "  sage --provider openai --key your_api_key"
echo ""
echo "Then start using it to commit changes:"
echo "  sage path/to/your/file.rs"