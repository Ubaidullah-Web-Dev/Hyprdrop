<div align="center">

  <img src="assets/hyprdrop.svg" width="140" height="140" alt="Hyprdrop Icon" />

  # Hyprdrop

  **Native Wayland radial file studio, quick editor, and drop-shelf utility for Arch Linux & Hyprland**

  <p align="center">
    <a href="https://github.com/Ubaidullah-Web-Dev/Hyprdrop/releases/latest"><img src="https://img.shields.io/github/v/release/Ubaidullah-Web-Dev/Hyprdrop?color=0A84FF&label=Release&logo=github&style=flat-square" alt="GitHub Release"></a>
    <a href="https://aur.archlinux.org"><img src="https://img.shields.io/badge/AUR-Coming%20Soon-orange?logo=arch-linux&logoColor=white&style=flat-square" alt="AUR Status"></a>
    <a href="https://hyprland.org"><img src="https://img.shields.io/badge/Hyprland-Wayland%20Layer%20Shell-00A4DC?logo=wayland&logoColor=white&style=flat-square" alt="Hyprland"></a>
    <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2024%20Edition-DEA584?logo=rust&logoColor=white&style=flat-square" alt="Rust"></a>
    <a href="https://gtk.org"><img src="https://img.shields.io/badge/GTK4-Libadwaita-3584E4?logo=gnome&logoColor=white&style=flat-square" alt="GTK4"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0-blue.svg?style=flat-square" alt="License: GPL-3.0"></a>
  </p>

  <p align="center">
    <b>Cursor-Anchored 144Hz Radial Wheel</b> • <b>Adaptive Conversions</b> • <b>Crop & Studio Modals</b> • <b>Chaining Drop Shelf</b>
  </p>

</div>

---

## ⚡ Overview

**Hyprdrop** is a hyper-optimized, native Wayland desktop utility tailored specifically for power users on **Hyprland** and **Arch Linux**. It eliminates window juggling and tedious terminal commands by spawning a cursor-centered, 144Hz radial pie menu whenever a file is dropped or dragged while holding `Shift`.

* 🚀 **Zero WebTech / Electron Bloat**: Built purely in **Rust (2024 Edition)** with `gtk4`, `libadwaita`, `gtk4-layer-shell`, `cairo-rs`, and `rayon`.
* ⚡ **Resource Footprint**: Daemon idle RAM strictly **< 20 MB**; overlay appearance latency **< 10 ms**.
* 🎨 **Refined Studio Aesthetic**: High-contrast tactile graphite palette (`#161618` at 92% alpha with Hyprland dual-cavity blur) complemented by Electric Blue (`#0A84FF`) and International Warm Orange (`#FF5722`) accents.

---

## 📸 Interface Preview

```
                  [ PNG ]
          [ WEBP ]       [ JPG ]
      [ AVIF ]               [ TIFF ]
              ( 410 KB )
      [ BMP ]                 [ ICO ]
          [ PDF ]         [ EXTRACT ]
```

* **Interactive Center Hub**: Displays exact file size or hovered target with fluid transitions.
* **Instant Mode Flip**: Hit `Tab`, `Space`, `Right-Click`, scroll the mouse wheel, or click the center hub to alternate between **Mode 1 (Format Conversions)** and **Mode 2 (Quick Studio Actions)**.
* **Smart Contextual Filtering**: Incompatible operations (e.g. attempting to crop or redact a video or archive) are automatically grayed out and disabled.

---

## ✨ Features

### 🔄 Mode 1 — Adaptive Format Conversion
Releasing a file onto any radial slice initiates instant background conversion, saving the new file directly alongside your source with intelligent collision protection (`image (2).png`):

| Category | Supported Formats |
| :--- | :--- |
| **Raster & Vector Images** | `PNG`, `JPG`, `WEBP`, `AVIF`, `TIFF`, `BMP`, `ICO`, `PDF` |
| **Documents & PDFs** | `DOCX`, `PDF`, `JPG`, `PNG`, `TXT`, `MD`, `HTML` |
| **Audio & Video** | `MP4`, `WEBM`, `GIF` (Lanczos palettegen two-pass), `MP3`, `WAV`, `FLAC`, `OPUS` |
| **Archives & Bundles** | `EXTRACT HERE`, `ZIP`, `TAR.ZST`, `TAR.GZ` |

---

### 🎨 Mode 2 — Quick-Action Studio Suite
* ✂️ **`CROP`**: Interactive crop modal with high-visibility boundary box, 8 draggable handles, dark mask, rule-of-thirds gridlines, and aspect ratio presets (`Free`, `1:1`, `16:9`, `4:3`, `3:2`, `9:16`).
* 🌄 **`ADD BG`**: Studio backdrop generator featuring a compact live preview, **33 curated aesthetic gradients and solids**, with real-time sliders for `Padding` (0–300 px), `Corners` (0–150 px), and `Shadow` (0.00–1.00 Gaussian blur).
* 🗜️ **`COMPRESS`**: Lossless and perceptual compression for images, PDFs, and video clips with toast notifications showing exact space saved.
* 🛡️ **`REDACT` & `ANNOTATE`**: Censorship and markup studio to draw blackout bars, Gaussian blur patches, pixelation blocks, callout dots, and highlight arrows.
* 🔍 **`METADATA`**: Full EXIF, GPS, and PDF metadata inspector with 1-click `Strip All Metadata` sanitization.
* 🔄 **`EDIT`**: Rapid rotation (90°, 180°, 270°), flip horizontal/vertical, brightness and contrast tuning, and transparent border trimming.
* 📋 **`OCR / COPY`**: Optical text recognition via `tesseract` or direct file/image copying to the Wayland clipboard via `wl-copy`.

---

### 🪝 Chaining Drop Shelf
Immediately following any conversion or studio edit, Hyprdrop displays a compact floating **Drop Shelf** thumbnail card with native drag-and-drop support:
* Drag the generated result directly into Discord, Slack, Firefox, or file managers.
* **Chain operations** by dragging the thumbnail directly back into the radial wheel (e.g., `Scan.pdf` ➔ `PNG` ➔ `Crop` ➔ `Add BG`).

---

## 📦 Installation

Hyprdrop offers multiple installation options tailored for both quick setup and system customization:

### Method 1: Instant 1-Liner (Precompiled Binary — Recommended)

Installs the official precompiled release binary to `~/.local/bin` in seconds with zero compilation dependencies:

```bash
curl -fsSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Hyprdrop/main/install.sh | bash -s -- --bin
```

---

### Method 2: 1-Liner Source Install (Auto-Compiling)

Clones the latest code and compiles with Cargo LTO optimizations:

```bash
curl -fsSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Hyprdrop/main/install.sh | bash
```

---

### Method 3: Arch User Repository (AUR)

> [!NOTE]
> **AUR Status: Coming Soon**
> Arch Linux sysadmins have temporarily paused new account registrations on `aur.archlinux.org` due to an automated spam wave. The PKGBUILD recipes are fully prepared in this repository (`packaging/aur-bin` and `packaging/aur-git`) and will be published the moment registrations resume.

Once live on the AUR:

```bash
# Precompiled binary package (Fastest)
yay -S hyprdrop-bin
# or
paru -S hyprdrop-bin

# Development git package (Builds from main branch)
yay -S hyprdrop-git
# or
paru -S hyprdrop-git
```

---

### Method 4: Native Arch Package (`makepkg -si`)

Build and install an official Arch Linux package directly on your machine:

```bash
# 1. Clone repository
git clone https://github.com/Ubaidullah-Web-Dev/Hyprdrop.git
cd Hyprdrop

# 2. Build precompiled package locally:
cd packaging/aur-bin
makepkg -si

# Or build from git source:
cd ../aur-git
makepkg -si
```

---

### Method 5: Manual Release Archive

Download the standalone tarball directly from the [GitHub Releases Page](https://github.com/Ubaidullah-Web-Dev/Hyprdrop/releases/latest):

```bash
# Download latest release
tar -xvf hyprdrop-v0.1.0-linux-x86_64.tar.gz
cd hyprdrop-v0.1.0-linux-x86_64

# Run automatic setup
./hyprdrop setup
```

---

## 🔄 Updating Hyprdrop

Keeping Hyprdrop up-to-date with new features, format converters, and fixes is seamless:

### Option A: Built-in Command (Easiest)
Simply run the update command from your terminal:
```bash
hyprdrop update
```

### Option B: Quick Re-run 1-Liner
```bash
curl -fsSL https://raw.githubusercontent.com/Ubaidullah-Web-Dev/Hyprdrop/main/install.sh | bash -s -- --bin
```

### Option C: From Git Repository
```bash
cd Hyprdrop
git pull origin main
./install.sh --bin
```

### Option D: Via AUR Helper (Once Published)
```bash
yay -Syu hyprdrop-bin
# or
paru -Syu hyprdrop-git
```

---

## 🛠️ Hyprland Configuration

Hyprdrop supports both modern **`hyprland.lua`** (native Lua syntax) and classic **`hyprland.conf`**.

### 1-Click Automatic Setup
You can automatically configure Hyprland window rules, layer rules, and mouse keybinds with:
```bash
hyprdrop setup
```
*(This automatically backs up your existing configuration with a timestamped backup before adding rules).*

### Manual Configuration

#### Modern Lua Syntax (`~/.config/hypr/hyprland.lua`)
```lua
-- --- Hyprdrop Radial Utility ---
hl.layer_rule({
    name = "hyprdrop",
    match = { namespace = "hyprdrop" },
    blur = true,
    ignore_alpha = 0.0,
    animation = "popin 80%",
})

-- Shift + Left Click drag trigger
hl.bind(
    "SHIFT + mouse:272",
    hl.dsp.exec_cmd("hyprdrop trigger"),
    { description = "Hyprdrop Radial Trigger" }
)
```

#### Classic Syntax (`~/.config/hypr/hyprland.conf`)
```ini
# --- Hyprdrop Radial Utility ---
layerrule = blur, hyprdrop
layerrule = ignorezero, hyprdrop
layerrule = animation popin 80%, hyprdrop

# Shift + Left Click drag trigger
bind = SHIFT, mouse:272, exec, hyprdrop trigger
```

---

## 💻 CLI Reference

| Command | Description |
| :--- | :--- |
| `hyprdrop --daemon` | Launches the background daemon with StatusNotifierItem (SNI) tray icon and IPC server |
| `hyprdrop trigger [file]` | Spawns the radial overlay centered directly under the current cursor position |
| `hyprdrop toggle` | Toggles radial overlay visibility |
| `hyprdrop settings` | Opens the desktop settings, activity history, and dependency health checker |
| `hyprdrop setup` *(or `configure`)* | Automatically backs up configuration and injects Hyprland layer rules & binds |
| `hyprdrop update` | Downloads and installs the latest Hyprdrop release automatically |
| `hyprdrop <file>` | Directly preloads `<file>` into the radial overlay |

---

## ⚙️ Systemd User Service

To automatically launch the Hyprdrop background daemon on graphical login:

```bash
systemctl --user enable --now hyprdrop.service
```

---

## 🧩 Optional Dependencies

Hyprdrop operates standalone, but benefits from standard Linux command-line utilities for extended media capabilities:

| Utility | Feature Enabled | Arch Linux Package |
| :--- | :--- | :--- |
| `ffmpeg` | Video/Audio conversion & GIF generation | `sudo pacman -S ffmpeg` |
| `pdftoppm` | High-fidelity PDF page rendering | `sudo pacman -S poppler` |
| `wl-copy` | Wayland clipboard integration | `sudo pacman -S wl-clipboard` |
| `tesseract` | Optical character recognition (OCR) | `sudo pacman -S tesseract tesseract-data-eng` |
| `hyprctl` | Wayland cursor tracking and layer shell | *(Installed with Hyprland)* |

---

## 📄 License

Hyprdrop is licensed under the [GNU General Public License v3.0 or later (GPL-3.0-or-later)](LICENSE).

Built with precision for the Arch Linux and Hyprland Wayland ecosystem.
