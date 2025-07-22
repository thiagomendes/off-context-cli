#!/bin/bash

# off-context-cli Auto-installer for macOS
# Usage: curl -sSL https://raw.githubusercontent.com/thiagomendes/off-context-cli/main/install-mac.sh | bash
# Usage (local): ./install-mac.sh --local /path/to/binary

set -e

LOCAL_PATH=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --local)
            LOCAL_PATH="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--local /path/to/binary]"
            exit 1
            ;;
    esac
done

echo "🧠 Installing off-context-cli for macOS..."

if [[ -n "$LOCAL_PATH" ]]; then
    echo "📍 Using local binary: $LOCAL_PATH"
fi

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ This installer is for macOS systems"
    echo "For Linux, use: curl -sSL https://raw.githubusercontent.com/thiagomendes/off-context-cli/main/install.sh | bash"
    exit 1
fi

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo "❌ Don't run this script as root/sudo"
   exit 1
fi

# Check and install jq dependency
if command -v jq >/dev/null 2>&1; then
    echo "✅ jq already installed"
else
    echo "📦 Installing jq dependency..."
    if command -v brew >/dev/null 2>&1; then
        brew install jq
    else
        echo "❌ Homebrew not found. Please install jq manually: brew install jq"
        exit 1
    fi
fi

if [[ -n "$LOCAL_PATH" ]]; then
    # Local installation mode
    if [[ ! -f "$LOCAL_PATH" ]]; then
        echo "❌ Local binary not found: $LOCAL_PATH"
        exit 1
    fi
    
    echo "📦 Using local binary..."
    VERSION="local-build"
    
    # Create temporary directory and copy local binary
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"
    cp "$LOCAL_PATH" ./off-context
    chmod +x ./off-context
else
    # GitHub release mode (default)
    echo "🔍 Finding latest release..."
    LATEST_RELEASE=$(curl -s https://api.github.com/repos/thiagomendes/off-context-cli/releases/latest)

    # Check if we got a valid response
    if [[ -z "$LATEST_RELEASE" ]] || echo "$LATEST_RELEASE" | jq -e '.message == "Not Found"' >/dev/null 2>&1; then
        echo "❌ No GitHub releases found yet."
        echo ""
        echo "Please use the manual installation method:"
        echo "1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "2. Clone repo: git clone https://github.com/thiagomendes/off-context-cli.git"
        echo "3. Build: cd off-context-cli && cargo build --release"
        echo "4. Install: sudo cp target/release/off-context /usr/local/bin/"
        echo ""
        echo "Check releases at: https://github.com/thiagomendes/off-context-cli/releases"
        exit 1
    fi

    VERSION=$(echo "$LATEST_RELEASE" | jq -r '.tag_name')
    DOWNLOAD_URL=$(echo "$LATEST_RELEASE" | jq -r '.assets[] | select(.name | contains("macos")) | .browser_download_url')

    if [[ "$VERSION" == "null" || "$DOWNLOAD_URL" == "null" || -z "$VERSION" || -z "$DOWNLOAD_URL" ]]; then
        echo "❌ No GitHub releases found yet."
        echo ""
        echo "Please use the manual installation method:"
        echo "1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "2. Clone repo: git clone https://github.com/thiagomendes/off-context-cli.git"
        echo "3. Build: cd off-context-cli && cargo build --release"
        echo "4. Install: sudo cp target/release/off-context /usr/local/bin/"
        echo ""
        echo "Check releases at: https://github.com/thiagomendes/off-context-cli/releases"
        exit 1
    fi

    echo "📦 Downloading off-context-cli $VERSION..."

    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"

    # Download and extract binary
    curl -L -o off-context-macos.tar.gz "$DOWNLOAD_URL"
    tar -xzf off-context-macos.tar.gz
fi

# Install binary
echo "📦 Installing binary..."
sudo cp off-context /usr/local/bin/
sudo chmod +x /usr/local/bin/off-context

# Verify installation
if command -v off-context >/dev/null 2>&1; then
    echo "✅ off-context-cli $VERSION installed successfully!"
    echo ""
    echo "🚀 Configuring global hooks..."
    
    # Run setup automatically
    if off-context setup; then
        echo ""
        echo "🎉 Setup complete! Ready to use."
        echo ""
        echo "📝 Next steps:"
        echo "1. Navigate to your project: cd /path/to/your/project"
        echo "2. Initialize project memory: off-context init"
        echo "3. Start using Claude Code normally!"
        echo ""
        echo "📚 For help: off-context --help"
    else
        echo "⚠️ Setup failed. You can run it manually later:"
        echo "   off-context setup"
    fi
else
    echo "❌ Installation failed. Binary not found in PATH."
    exit 1
fi

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo "🎉 Installation complete!"