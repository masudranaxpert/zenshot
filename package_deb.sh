#!/usr/bin/env bash
# Script to package ZenShot as a standard Debian / Ubuntu (.deb) package
set -e

VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d '"' -f2)
ARCH="amd64"
PKG_DIR="target/debian_pkg"

echo "Building .deb package for ZenShot (Ubuntu / Debian)..."

# Ensure binary is built
if [ ! -f "target/release/zenshot" ]; then
    echo "Release binary not found. Building with cargo..."
    cargo build --release
fi

# Clean and prepare directory structure
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/pixmaps"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/scalable/apps"

# Control file
cat << EOF > "$PKG_DIR/DEBIAN/control"
Package: zenshot
Version: $VERSION
Section: graphics
Priority: optional
Architecture: $ARCH
Maintainer: ZenShot Team <dev@zenshot.io>
Description: Ultra-fast, featherlight Lightshot alternative for Linux
 Operates completely in RAM with zero disk pre-saving overhead.
 Instant clipboard copy, rectangle, arrow, and pen annotations.
EOF

# Copy files
cp target/release/zenshot "$PKG_DIR/usr/bin/zenshot"
chmod 755 "$PKG_DIR/usr/bin/zenshot"
cp zenshot.desktop "$PKG_DIR/usr/share/applications/zenshot.desktop"
chmod 644 "$PKG_DIR/usr/share/applications/zenshot.desktop"

if [ -f "assets/icons/feather.png" ]; then
    cp assets/icons/feather.png "$PKG_DIR/usr/share/pixmaps/zenshot.png"
    cp assets/icons/feather.png "$PKG_DIR/usr/share/icons/hicolor/256x256/apps/zenshot.png"
    chmod 644 "$PKG_DIR/usr/share/pixmaps/zenshot.png"
    chmod 644 "$PKG_DIR/usr/share/icons/hicolor/256x256/apps/zenshot.png"
fi

if [ -f "assets/icon.svg" ]; then
    cp assets/icon.svg "$PKG_DIR/usr/share/icons/hicolor/scalable/apps/zenshot.svg"
    chmod 644 "$PKG_DIR/usr/share/icons/hicolor/scalable/apps/zenshot.svg"
fi

# Build package
dpkg-deb --build "$PKG_DIR" "zenshot_${VERSION}_${ARCH}.deb"

echo "=================================================="
echo "SUCCESS! Created: zenshot_${VERSION}_${ARCH}.deb"
echo "Install with: sudo dpkg -i zenshot_${VERSION}_${ARCH}.deb"
echo "=================================================="
