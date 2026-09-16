# ⚡ ZenShot

> **Featherlight, ultra-fast, cross-platform screen capture tool with zero disk I/O.**  
> Native performance on **Windows**, **Ubuntu / Linux**, and **macOS**.

Windows ships as a tray app with a setup installer (auto-start + hotkeys), the same way Lightshot does. Linux ships as a `.deb` — no background process, because Wayland does not let apps steal PrintScreen.

---

## Windows (setup.exe)

1. Run `zenshot-setup-<version>.exe`.
2. Leave **Start ZenShot with Windows** checked.
3. Finish. A tray icon appears.
4. Press **Print Screen** (change it later in **Options → Hotkeys**).

Tray menu: Take a screenshot, Options, About, Help, Exit.

Portable option: `zenshot-windows-x86_64.zip` — run `zenshot.exe` once so it sits in the tray. `--capture` still works as a one-shot overlay.

```
zenshot                 stay in the tray
zenshot --capture       open the overlay
zenshot --options       settings
```

---

## Linux (.deb, no tray)

Wayland compositors own the PrintScreen key. Industry-standard tools (Flameshot, GNOME Screenshot) therefore install a launcher and let you bind the key in system settings — they do not register a global hook.

```bash
sudo dpkg -i zenshot_<version>_amd64.deb
sudo apt-get install -f
```

Then bind **Ctrl+Shift+S**:

1. Settings → Keyboard → Custom Shortcuts → **+**
2. Name `ZenShot`, command `zenshot`, shortcut `Ctrl+Shift+S`.

```
zenshot                 open the overlay
zenshot --options       settings (save folder, PNG/JPEG, notifications)
```

From source:

```bash
sudo apt install -y pkg-config libpipewire-0.3-dev libspa-0.2-dev libclang-dev \
  libegl1-mesa-dev libgl1-mesa-dev libgbm-dev libwayland-dev libxkbcommon-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libx11-dev
chmod +x install.sh && ./install.sh
```

Build a `.deb` yourself with `./package_deb.sh`.

---

## Overlay shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + A` | Select full screen |
| `Ctrl + C` | Copy to clipboard & exit |
| `Ctrl + S` | Save & exit |
| `Ctrl + P` | Print & exit |
| `Ctrl + Z` | Undo last annotation |
| `Ctrl + X` / `Esc` | Close without saving |
| Right-click | Clear selection |

Tools: Pen, Line, Arrow, Rectangle, Marker, Text, Color, Undo.

---

## Configuration

`zenshot --options`, or edit:

- **Linux:** `~/.config/zenshot/config.toml`
- **Windows:** `%APPDATA%\zenshot\config.toml`

```toml
save_dir = "~/Pictures/Screenshots"
filename_format = "ZenShot_%Y-%m-%d_%H-%M-%S.png"
output_format = "png"          # or "jpeg"
jpeg_quality = 90
autostart = true               # Windows only
show_notifications = true
capture_cursor = false
keep_selection = false
```

---

## CI release artifacts

Tag `v*` (or run the workflow manually):

- Ubuntu: `zenshot-ubuntu-amd64.deb`, `zenshot-linux-x86_64.tar.gz`
- Windows: `zenshot-setup-<version>.exe`, `zenshot-windows-x86_64.zip`
- macOS: `zenshot-macos-x86_64.tar.gz`
