# ⚡ ZenShot

> Ultra-fast, featherlight screen capture utility with zero disk I/O.  
> Inspired by Lightshot, rebuilt for modern **Windows**, **Linux**, and **macOS**.

[![Release](https://img.shields.io/github/v/release/masudranaxpert/zenshot?style=flat-square)](https://github.com/masudranaxpert/zenshot/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

---

## ✨ Features

- **⚡ Zero Disk I/O:** Captures and copies directly to clipboard entirely in RAM.
- **🎯 Precision Selection:** Freely drag, resize, and fine-tune capture boundaries with live dimensions.
- **🎨 Rich Annotations:** Pen, Marker, Line, Arrow, Rectangle, Inline Text, and Color Palette.
- **🚀 Native Performance:** Written in Rust — instant startup, zero background bloat, and smooth DWM integration.
- **🔔 Instant Feedback:** Desktop notifications for clipboard copy and fullscreen saves.
- **⚙️ Configurable:** Customize hotkeys, save directory, PNG/JPEG quality, and autostart.

---

## ⌨️ Default Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + Shift + S` | **Take Screenshot** (Global Hotkey) |
| `Ctrl + Alt + S` | **Instant Fullscreen Save** |
| `Ctrl + C` | Copy selection to clipboard & exit |
| `Ctrl + S` | Save selection to disk & exit |
| `Ctrl + A` | Select full screen |
| `Ctrl + P` | Print selection & exit |
| `Ctrl + Z` | Undo last annotation |
| `Esc` / `Ctrl + X` | Close overlay without saving |
| `Right-Click` | Clear selection |

---

## 📦 Installation

### Windows
Download the latest `zenshot-setup-<version>.exe` or portable `.zip` from **[Releases](https://github.com/masudranaxpert/zenshot/releases)**.
- Runs quietly in system tray with global hotkeys.
- Configure via tray icon menu or `zenshot --options`.

### Linux (Ubuntu / Debian)
Download `zenshot_<version>_amd64.deb` from **[Releases](https://github.com/masudranaxpert/zenshot/releases)**:
```bash
sudo dpkg -i zenshot_<version>_amd64.deb
sudo apt-get install -f
```
> **Tip:** On Wayland/X11, bind `Ctrl+Shift+S` to `zenshot` in your desktop system shortcut settings.

---

## 🛠️ CLI Usage

```bash
zenshot               # Start daemon / overlay
zenshot --capture     # Force capture overlay
zenshot --options     # Open settings dialog
zenshot --version     # Display version
```

---

## 📄 License

Distributed under the [MIT License](LICENSE).

