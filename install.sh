#!/usr/bin/env bash
# ZenShot (Ultra-Fast Native Screen Capture) Installer
set -e

echo "=================================================="
echo "      Installing ZenShot (Featherlight)           "
echo "=================================================="

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed."
    echo "Please install Rust using: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "[1/4] Building release binary with optimizations..."
cargo build --release

# Ensure local bin directory exists
mkdir -p "$HOME/.local/bin"

echo "[2/4] Installing binary to $HOME/.local/bin/zenshot..."
cp target/release/zenshot "$HOME/.local/bin/zenshot"
chmod +x "$HOME/.local/bin/zenshot"

# Ensure local applications directory exists
mkdir -p "$HOME/.local/share/applications"
mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"

echo "[3/4] Creating desktop entry and icon..."
cp zenshot.desktop "$HOME/.local/share/applications/zenshot.desktop"
chmod +x "$HOME/.local/share/applications/zenshot.desktop"

if [ -f "assets/icon.svg" ]; then
    cp assets/icon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/zenshot.svg"
fi

echo "[4/4] Installation Complete!"
echo "--------------------------------------------------"
echo "Binary installed at: $HOME/.local/bin/zenshot"
echo ""
echo "Make sure $HOME/.local/bin is in your PATH. If not, add:"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "to your ~/.bashrc or ~/.profile."
echo ""
echo "=================================================="
echo "    How to Bind to 'PrintScreen' Key in Ubuntu:   "
echo "=================================================="
echo "1. Open Ubuntu 'Settings' -> 'Keyboard'."
echo "2. Scroll down and click 'View and Customize Shortcuts'."
echo "3. Click 'Custom Shortcuts' -> '+' (Add Shortcut)."
echo "4. Set:"
echo "     Name:     ZenShot"
echo "     Command:  $HOME/.local/bin/zenshot"
echo "     Shortcut: Press 'Print' (PrtScn)"
echo "5. Click 'Add'."
echo "=================================================="
