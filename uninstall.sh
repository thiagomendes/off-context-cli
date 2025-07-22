#!/bin/bash

# off-context-cli Uninstaller
# Usage: curl -sSL https://raw.githubusercontent.com/thiagomendes/off-context-cli/main/uninstall.sh | bash

set -e

echo "🗑️  Uninstalling off-context-cli..."

# Check if off-context is installed
if ! command -v off-context >/dev/null 2>&1; then
    echo "ℹ️  off-context-cli is not installed or not in PATH"
else
    # Remove global hooks and memory using off-context itself
    echo "🧹 Removing global hooks and memory..."
    off-context uninstall 2>/dev/null || echo "ℹ️  Global cleanup completed"
fi

# Remove binary from all possible locations
echo "📦 Removing binary..."

# Common installation paths
BINARY_PATHS=(
    "/usr/local/bin/off-context"
    "/usr/bin/off-context"
    "$HOME/.local/bin/off-context"
    "$HOME/bin/off-context"
)

REMOVED=false

for path in "${BINARY_PATHS[@]}"; do
    if [[ -f "$path" ]]; then
        if [[ "$path" == "$HOME"* ]]; then
            # Home directory - no sudo needed
            rm -f "$path"
        else
            # System directory - needs sudo
            sudo rm -f "$path"
        fi
        echo "✅ Binary removed from $path"
        REMOVED=true
    fi
done

# Also check if off-context is in PATH and find its location
if command -v off-context >/dev/null 2>&1; then
    BINARY_LOCATION=$(which off-context)
    echo "⚠️  Binary still found at: $BINARY_LOCATION"
    
    if [[ "$BINARY_LOCATION" == "$HOME"* ]]; then
        rm -f "$BINARY_LOCATION"
    else
        sudo rm -f "$BINARY_LOCATION"
    fi
    echo "✅ Binary removed from $BINARY_LOCATION"
    REMOVED=true
fi

if [[ "$REMOVED" == false ]]; then
    echo "ℹ️  No off-context binary found in common locations"
fi

# Remove configuration directories
echo "🗂️  Removing configuration directories..."

# Remove global config
if [[ -d "$HOME/.off-context" ]]; then
    rm -rf "$HOME/.off-context"
    echo "✅ Removed ~/.off-context"
else
    echo "ℹ️  ~/.off-context not found"
fi

# Remove Claude Code hooks directory
CLAUDE_HOOKS_DIR="$HOME/.config/claude/hooks"
if [[ -d "$CLAUDE_HOOKS_DIR" ]]; then
    rm -rf "$CLAUDE_HOOKS_DIR"
    echo "✅ Removed Claude Code hooks directory"
else
    echo "ℹ️  Claude Code hooks directory not found"
fi

# Check for project-specific configurations
echo ""
echo "🔍 Checking for project-specific configurations..."
find "$HOME" -name ".claude" -type d 2>/dev/null | while read -r dir; do
    if [[ -f "$dir/settings.local.json" ]]; then
        echo "ℹ️  Found project config: $dir/settings.local.json"
        echo "   You may want to manually remove off-context hooks from this file"
    fi
done

echo ""

# Final verification
if command -v off-context >/dev/null 2>&1; then
    echo "⚠️  WARNING: off-context command still found in PATH!"
    echo "   Location: $(which off-context)"
    echo "   You may need to:"
    echo "   1. Restart your terminal"
    echo "   2. Run: hash -r"
    echo "   3. Manually remove: $(which off-context)"
    echo ""
else
    echo "✅ off-context-cli has been completely uninstalled!"
fi

echo ""
echo "ℹ️  Note: This uninstaller does not remove:"
echo "   - jq dependency (may be used by other tools)"
echo "   - Rust toolchain (if installed during manual installation)"
echo "   - Project-specific .claude/settings.local.json files"
echo ""
echo "💡 If off-context command still works, try:"
echo "   - Restart your terminal"
echo "   - Run: hash -r"
echo ""
echo "🎉 Uninstallation complete!"