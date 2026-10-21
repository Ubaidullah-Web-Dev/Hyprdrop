#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo "           Hyprdrop — Arch Linux / Hyprland Installer     "
echo "=========================================================="

if [ -z "${PREFIX:-}" ]; then
    if [ -w "/usr/local/bin" ]; then
        PREFIX="/usr/local"
    else
        PREFIX="${HOME}/.local"
    fi
fi
BIN_DIR="${PREFIX}/bin"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${ICON_DIR}" "${SYSTEMD_DIR}"

# 1. Build release binary if not present
if [ ! -f "target/release/hyprdrop" ] || [ ! -f "target/release/hyprdrop-detect-selection" ]; then
    echo "==> Building optimized release binary with LTO..."
    cargo build --release
fi

# Ensure helper binary is compiled
if [ ! -f "target/release/hyprdrop-detect-selection" ]; then
    echo "==> Compiling native hyprdrop-detect-selection helper..."
    gcc -O3 c/hyprdrop-detect-selection.c $(pkg-config --cflags --libs atspi-2 gobject-2.0 glib-2.0) -o target/release/hyprdrop-detect-selection
fi

# 2. Install binary
echo "==> Installing hyprdrop binaries to ${BIN_DIR}..."
if [ -w "${BIN_DIR}" ] || [ -w "$(dirname "${BIN_DIR}")" ]; then
    install -Dm755 "target/release/hyprdrop" "${BIN_DIR}/hyprdrop"
    install -Dm755 "target/release/hyprdrop-detect-selection" "${BIN_DIR}/hyprdrop-detect-selection"
else
    echo "Requesting sudo privileges for ${BIN_DIR} installation..."
    sudo install -Dm755 "target/release/hyprdrop" "${BIN_DIR}/hyprdrop"
    sudo install -Dm755 "target/release/hyprdrop-detect-selection" "${BIN_DIR}/hyprdrop-detect-selection"
fi

# 3. Install desktop entry and icons
echo "==> Installing desktop entry and icons..."
install -Dm644 "assets/hyprdrop.desktop" "${APP_DIR}/hyprdrop.desktop"
install -Dm644 "assets/hyprdrop.svg" "${ICON_DIR}/hyprdrop.svg"

# 4. Install systemd user service
echo "==> Installing systemd user daemon service..."
sed "s|ExecStart=.*|ExecStart=${BIN_DIR}/hyprdrop --daemon|g" assets/hyprdrop.service > "${SYSTEMD_DIR}/hyprdrop.service"

# Reload desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_DIR}" || true
fi

# 5. Dependency Check
echo ""
echo "==> Verifying system dependencies:"
for dep in ffmpeg pdftoppm wl-copy tesseract hyprctl; do
    if command -v "$dep" >/dev/null 2>&1; then
        echo "    [✓] $dep (found at $(command -v "$dep"))"
    else
        echo "    [!] $dep (missing - optional for full feature support)"
    fi
done

echo ""
echo "==> Enabling hyprdrop user service (daemon):"
systemctl --user daemon-reload || true
systemctl --user enable --now hyprdrop.service || true

echo ""
echo "==> Running Hyprland auto-configuration..."
"${BIN_DIR}/hyprdrop" settings &

echo ""
echo "=========================================================="
echo "   Hyprdrop installation completed successfully!          "
echo "   Daemon & Tray: hyprdrop --daemon (StatusNotifierItem) "
echo "   Trigger with: hyprdrop trigger                         "
echo "   Settings GUI: hyprdrop settings                        "
echo "   Lua Bind:   hl.bind(\"SHIFT + mouse:272\", hl.dsp.exec_cmd(\"hyprdrop trigger\"))"
echo "   Conf Bind:  bind = SHIFT, mouse:272, exec, hyprdrop trigger"
echo "=========================================================="

