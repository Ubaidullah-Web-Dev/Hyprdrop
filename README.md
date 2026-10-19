# Hyprdrop (`hyprdrop`)

> **Industrial-grade radial drag-and-drop file converter, quick-action studio editor, and floating drop-shelf utility for Arch Linux and Hyprland (Wayland).**

[![Arch Linux](https://img.shields.io/badge/Arch_Linux-AUR_Package-1793D1?logo=arch-linux&logoColor=white)](https://aur.archlinux.org/packages/hyprdrop)
[![Hyprland](https://img.shields.io/badge/Hyprland-Wayland_Layer_Shell-00A4DC?logo=wayland&logoColor=white)](https://hyprland.org)
[![Rust 2024](https://img.shields.io/badge/Rust-2024_Edition-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![GTK4 + Libadwaita](https://img.shields.io/badge/GTK4-Libadwaita-3584E4?logo=gnome&logoColor=white)](https://gtk.org)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

---

## Overview

**Hyprdrop** is an ultra-fast, native Wayland utility designed specifically for power users on **Hyprland** and **Arch Linux**. It eliminates context switching and file manager clutter by spawning a cursor-centered, 144Hz radial pie menu whenever a file is dropped or dragged while holding `Shift`.

- **Zero WebTech/Electron Bloat**: Built purely in **Rust (2024 Edition)** with `gtk4`, `libadwaita`, `gtk4-layer-shell`, `cairo-rs`, and `rayon`.
- **Resource Footprint**: Daemon idle RAM strictly **< 20 MB**; overlay appearance latency **< 10 ms**.
- **Industrial Studio Palette**: Eliminates generic neon AI gradients in favor of tactile studio graphite (`#161618` at 92% alpha with Hyprland dual-cavity background blur) and International Warm Orange (`#FF5722`) highlights.

---

## Key Features

### 1. Cursor-Anchored Radial Pie Menu (380x380 px Cairo Widget)
- Queries `hyprctl cursorpos -j` to spawn a `Layer::Overlay` window centered directly beneath your cursor.
- **Center Hub**:
  - Displays file size (`410 KB`, `1.4 MB`) by default.
  - Dynamically updates to show the hovered format or action name when tracking over outer slices.
- **Instant Mode Switching**: Press `Tab`, `Space`, `Right-Click`, scroll your mouse wheel, or click the center hub to smoothly flip between **Mode 1** and **Mode 2**.

```
              [ PNG ]
       [ WEBP ]     [ JPG ]
   [ AVIF ]             [ TIFF ]
           ( 410 KB )
   [ BMP ]               [ ICO ]
       [ PDF ]       [ EXTRACT ]
```

### 2. Mode 1 — Dynamic Format Conversion (Adapts to MIME Type)
Releasing a file onto any slice triggers instant background conversion, saving the result alongside the original file with collision-safe naming (`foo (2).png`):
- **Images**: `PNG`, `JPG`, `WEBP`, `AVIF`, `TIFF`, `BMP`, `ICO`, `PDF`
- **PDF & Documents**: `DOCX`, `JPG`, `PNG`, `TXT`, `MD`, `HTML`, `PDF`
- **Audio & Video**: `MP4`, `WEBM`, `GIF` (Lanczos palettegen two-pass), `MP3`, `WAV`, `FLAC`, `OPUS`
- **Archives**: `EXTRACT HERE`, `ZIP`, `TAR.ZST`, `TAR.GZ`

### 3. Mode 2 — Quick-Action Studio Modals
- **`CROP`**: Interactive crop modal with an orange boundary box, 8 draggable handles, exterior darkening mask, rule-of-thirds gridlines, aspect ratio presets (`Free`, `1:1`, `16:9`, `4:3`, `3:2`, `9:16`), and live pixel dimensions.
- **`ADD BG`**: Studio backdrop beautifier featuring 16 curated gradients and solids, real-time sliders for `Padding` (0–300 px), `Corners` (0–150 px rounded corners), and `Shadow` (0.00–1.00 Gaussian drop shadow).
- **`COMPRESS`**: Lossless and perceptual compression for images, PDFs, and videos with toast notifications reporting exact space saved (`Saved 64%`).
- **`REDACT` & `ANNOTATE`**: Interactive studio to draw blackout bars, Gaussian blur, pixelation boxes, arrows, callout dots, and highlight rectangles.
- **`METADATA`**: EXIF, GPS, and PDF metadata inspector with a 1-click `Strip All Metadata` sanitizer.
- **`EDIT`**: Rapid orientation transform (90°, 180°, 270°, Flip H/V), brightness/contrast adjustment, and automatic transparent border trimming.
- **`OCR / COPY`**: Extracts text via `tesseract` or copies files/images directly to the Wayland clipboard via `wl-copy`.

### 4. Floating Chaining Drop Shelf
Immediately following any conversion or studio edit, Hyprdrop spawns a compact floating **Drop Shelf** thumbnail card with `GtkDragSource`.
- Users can immediately drag the newly generated file into external applications (Discord, Slack, Firefox, Thunderbird) or **drag it back into the radial wheel to chain actions** (e.g. `Page 001.pdf` -> `Page 001.png` -> `Crop` -> `Add BG`).

### 5. Desktop Settings & Activity App (`hyprdrop settings`)
- Comprehensive activity history with timestamps, original and converted sizes, and quick open buttons.
- Dependency health checker (`ffmpeg`, `pdftoppm`, `libreoffice`, `pandoc`, `wl-clipboard`, `tesseract`, `hyprctl`).
- 1-click automated Hyprland configurator (backs up `hyprland.conf` and injects recommended `layerrule` and `bind`).

---

## Hyprland Configuration

Hyprdrop supports both modern **`hyprland.lua`** (Hyprland's native Lua syntax) and classic **`hyprland.conf`**.

### 1. Modern Lua Syntax (`~/.config/hypr/hyprland.lua`)

```lua
-- --- Hyprdrop Radial Utility ---
hl.layer_rule({
    name = "hyprdrop",
    match = { namespace = "hyprdrop" },
    blur = true,
    ignore_alpha = 0.0,
    animation = "popin 80%",
})

-- Shift + Left Click drag activation
hl.bind(
    "SHIFT + mouse:272",
    hl.dsp.exec_cmd("hyprdrop trigger"),
    { description = "Hyprdrop Radial Trigger" }
)
```

### 2. Classic Syntax (`~/.config/hypr/hyprland.conf`)

```ini
# --- Hyprdrop Radial Utility ---
layerrule = blur, hyprdrop
layerrule = ignorezero, hyprdrop
layerrule = animation popin 80%, hyprdrop

# Shift + Left Click drag activation
bind = SHIFT, mouse:272, exec, hyprdrop trigger
```

Alternatively, open `hyprdrop settings` and click **"Configure Hyprland Automatically"** to auto-detect your active configuration format, create an automated timestamped backup, and inject the rules.


---

## Installation

### From the AUR (Arch User Repository)

```bash
yay -S hyprdrop
# or
paru -S hyprdrop
```

### Manual Build via `makepkg`

```bash
git clone https://github.com/hyprland-community/hyprdrop.git
cd hyprdrop
makepkg -si
```

### Local Build from Source

```bash
git clone https://github.com/hyprland-community/hyprdrop.git
cd hyprdrop
cargo build --release
./install.sh
```

---

## CLI Usage

| Command | Description |
| :--- | :--- |
| `hyprdrop --daemon` | Starts the background daemon with StatusNotifierItem (SNI) tray icon, context menu, and IPC server |
| `hyprdrop trigger [file]` | Triggers the radial overlay centered under the cursor |

| `hyprdrop toggle` | Toggles overlay visibility |
| `hyprdrop settings` | Opens the desktop settings, activity history, and health checker |
| `hyprdrop <file>` | Spawns radial overlay preloaded with `<file>` |

---

## Systemd User Service

To enable the daemon on login:

```bash
systemctl --user enable --now hyprdrop.service
```

---

## Architecture & Codebase

```
hyprdrop/
├── assets/                 # Icons, desktop file, systemd service
├── src/
│   ├── main.rs             # CLI dispatcher & entrypoint
│   ├── app.rs              # GTK4 application lifecycle & IPC daemon loop
│   ├── cli.rs              # Clap CLI argument parsing
│   ├── config.rs           # Preferences & history persistence
│   ├── ipc.rs              # Unix domain socket protocol
│   ├── hyprland/           # Hyprland IPC, layer-shell, config installer
│   ├── engine/             # Image, PDF, Video, Archive, Compression engine
│   └── ui/                 # Cairo radial menu, studio modals, drop shelf
├── tests/                  # Polar math, collision naming, engine integration tests
├── PKGBUILD                # Arch Linux AUR package definition
├── .SRCINFO                # Generated package metadata
└── install.sh              # One-step build and install script
```

---

## License

GNU General Public License v3.0 or later ([GPL-3.0-or-later](LICENSE)).
Built for the Linux Wayland & Hyprland community.
