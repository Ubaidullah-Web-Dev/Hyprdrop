#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo "           Hyprdrop — Arch Linux / Hyprland Installer     "
echo "=========================================================="

PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${PREFIX}/bin"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

# 1. Build release binary if not present
if [ ! -f "target/release/hyprdrop" ]; then
    echo "==> Building optimized release binary with LTO..."
    cargo build --release
fi

# 2. Install binary
echo "==> Installing hyprdrop binary to ${BIN_DIR}..."
if [ -w "${BIN_DIR}" ]; then
    install -Dm755 "target/release/hyprdrop" "${BIN_DIR}/hyprdrop"
else
    echo "Requesting sudo privileges for ${BIN_DIR} installation..."
    sudo install -Dm755 "target/release/hyprdrop" "${BIN_DIR}/hyprdrop"
fi

# 3. Install desktop entry and icons
echo "==> Installing desktop entry and icons..."
install -Dm644 "assets/hyprdrop.desktop" "${APP_DIR}/hyprdrop.desktop"
install -Dm644 "assets/hyprdrop.svg" "${ICON_DIR}/hyprdrop.svg"

# 4. Install systemd user service
echo "==> Installing systemd user daemon service..."
install -Dm644 "assets/hyprdrop.service" "${SYSTEMD_DIR}/hyprdrop.service"

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
echo "   Trigger with: hyprdrop trigger                         "
echo "   Settings GUI: hyprdrop settings                        "
echo "   Hyprland Bind: bind = SHIFT, mouse:272, exec, hyprdrop trigger"
echo "=========================================================="
