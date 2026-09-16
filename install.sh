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
if [ -f "assets/zenshot.png" ]; then
    mkdir -p "$HOME/.local/share/icons/hicolor/256x256/apps"
    cp assets/zenshot.png "$HOME/.local/share/icons/hicolor/256x256/apps/zenshot.png"
fi

echo "[4/4] Installation Complete!"
echo "--------------------------------------------------"
echo "Binary installed at: $HOME/.local/bin/zenshot"
echo "Settings window:     zenshot --options"
echo ""
echo "Make sure $HOME/.local/bin is in your PATH. If not, add:"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "to your ~/.bashrc or ~/.profile."
echo ""
echo "=================================================="
echo "    Bind PrintScreen in your desktop settings     "
echo "=================================================="
echo "GNOME / Ubuntu:"
echo "  Settings -> Keyboard -> Custom Shortcuts -> +"
echo "  Name:     ZenShot"
echo "  Command:  $HOME/.local/bin/zenshot"
echo "  Shortcut: Print (PrtScn)"
echo ""
echo "KDE Plasma:"
echo "  System Settings -> Shortcuts -> Custom Shortcuts"
echo "=================================================="
