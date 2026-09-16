# ⚡ ZenShot

> **Featherlight, ultra-fast, cross-platform screen capture tool with zero disk I/O.**  
> Native performance on **Ubuntu / Linux**, **Windows**, and **macOS**.

---

## 🚀 Why ZenShot?

Most desktop screenshot tools (like Flameshot) are bogged down by heavy runtimes, slow startup times, and writing temporary files to disk before copying.

**ZenShot** is engineered with a **Zen philosophy**:
- ⚡ **Instant Launch:** Starts in < 20 milliseconds.
- 🧠 **100% In-Memory (Zero-Disk I/O):** The screen is frozen directly into RAM.
- 📋 **Pure RAM Clipboard (`Ctrl+C`):** Copies directly from RAM to your system clipboard without creating any temporary files on disk, then exits immediately.
- 💾 **On-Demand Saving (`Ctrl+S`):** Never touches your storage unless you explicitly hit Save.
- 🎨 **Pro Annotation Suite:** High-DPI rectangle boxes, direction arrows, smooth freehand pen, and 5 curated designer colors.
- 📦 **Zero-Config Distribution:** Install directly via `.deb` package or single binary.

---

## ⌨️ Keyboard Shortcuts

These mirror the accelerators Lightshot ships with, recovered from its binary.

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + A` | **Select full screen** |
| `Ctrl + C` | **Instant Copy:** Crop in RAM, copy to clipboard & exit |
| `Ctrl + S` | **Save:** Save timestamped PNG to configured directory & exit |
| `Ctrl + P` | **Print:** Send the selection to the default printer & exit |
| `Ctrl + Z` | **Undo:** Remove last drawn annotation |
| `Ctrl + X` / `Esc` | **Close:** Exit immediately without saving or copying |
| Right-click | **Clear selection** |

Tools are selected from the vertical toolbar: Pen, Line, Arrow, Rectangle, Marker, Text, Color, Undo.

---

## 🛠️ Direct Installation on Ubuntu / Debian

### Option 1: Direct `.deb` Package (Recommended — No Build Required)
Download `zenshot_0.1.0_amd64.deb` and install with one command:
```bash
sudo dpkg -i zenshot_0.1.0_amd64.deb
```
This automatically registers:
- Binary to `/usr/bin/zenshot`
- Application icon and `.desktop` launcher in your application menu.

To generate the `.deb` package yourself at any time:
```bash
chmod +x package_deb.sh
./package_deb.sh
```

---

### Option 2: Automated Installation from Source
Ensure build dependencies are installed on Ubuntu:
```bash
sudo apt update
sudo apt install -y pkg-config libpipewire-0.3-dev libspa-0.2-dev libclang-dev libegl1-mesa-dev libgl1-mesa-dev libgbm-dev libwayland-dev libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libx11-dev
chmod +x install.sh
./install.sh
```

---

## ⚙️ Setting Up `PrintScreen` (PrtScn) Key in Ubuntu

1. Open Ubuntu **Settings** -> **Keyboard**.
2. Scroll down to **View and Customize Shortcuts**.
3. Select **Custom Shortcuts** and click **+** (Add Shortcut).
4. Enter:
   - **Name:** `ZenShot`
   - **Command:** `zenshot`
   - **Shortcut:** Press the `Print` (`PrtScn`) key.
5. Click **Add**.

Now, whenever you press `PrtScn`, ZenShot opens instantly.

---

## 🌐 Automated Multi-Platform CI/CD (.yml Workflow)

The project includes a production-grade GitHub Actions workflow at [`.github/workflows/release.yml`](.github/workflows/release.yml).

Whenever you push a tag or trigger a manual run in GitHub Actions, it automatically builds in the cloud:
- **Ubuntu / Debian:** `zenshot-ubuntu-amd64.deb` & `zenshot-linux-x86_64.tar.gz`
- **Windows:** `zenshot-windows-x86_64.zip` (containing `zenshot.exe`)
- **macOS:** `zenshot-macos-x86_64.tar.gz`

---

## 📁 Configuration

Settings are stored in:
- **Linux:** `~/.config/zenshot/config.toml`
- **Windows:** `%APPDATA%\zenshot\config.toml`

```toml
# Save directory
save_dir = "~/Pictures/Screenshots"

# Filename pattern (supports strftime format)
filename_format = "ZenShot_%Y-%m-%d_%H-%M-%S.png"

# Default RGB annotation stroke color [R, G, B]
stroke_color = [239, 68, 68]

# Default stroke thickness
stroke_thickness = 2.5
```
