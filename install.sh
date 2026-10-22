#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Hyprdrop — Ultra-lightweight Radial File Studio & Drop-Shelf for Hyprland
# Automated Installer & Uninstaller
# ==============================================================================

MODE="user"
DO_UNINSTALL=false
BUILD_RELEASE=true

show_help() {
    cat << EOF
Hyprdrop Installer

Usage:
  ./install.sh [options]

Options:
  --user         Install to ~/.local/bin and user directories (Default, no root required)
  --system       Install globally to /usr/local/bin (Requires sudo/root)
  --no-build     Skip building binaries and install existing target/release artifacts
  --uninstall    Cleanly remove installed Hyprdrop files, services, and desktop entries
  -h, --help     Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --user)
            MODE="user"
            shift
            ;;
        --system)
            MODE="system"
            shift
            ;;
        --no-build)
            BUILD_RELEASE=false
            shift
            ;;
        --uninstall)
            DO_UNINSTALL=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

if [ "$MODE" = "system" ]; then
    PREFIX="/usr/local"
    BIN_DIR="${PREFIX}/bin"
    DATA_DIR="/usr/local/share"
    SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
else
    PREFIX="${HOME}/.local"
    BIN_DIR="${PREFIX}/bin"
    DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
    SYSTEMD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
fi

APP_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons/hicolor"

# ------------------------------------------------------------------------------
# Handle Remote Execution (e.g. curl -fsSL ... | bash)
# ------------------------------------------------------------------------------
if [ "$DO_UNINSTALL" = false ] && [ ! -f "Cargo.toml" ] && [ ! -f "target/release/hyprdrop" ]; then
    echo "==> Hyprdrop source not found in current directory."
    echo "==> Fetching latest Hyprdrop source from GitHub..."
    TEMP_CLONE="$(mktemp -d)"
    trap 'rm -rf "${TEMP_CLONE}"' EXIT
    git clone --depth=1 https://github.com/Ubaidullah-Web-Dev/Hyprdrop.git "${TEMP_CLONE}"
    cd "${TEMP_CLONE}"
fi

# ------------------------------------------------------------------------------
# Handle Uninstallation
# ------------------------------------------------------------------------------
if [ "$DO_UNINSTALL" = true ]; then
    echo "==> Uninstalling Hyprdrop..."
    systemctl --user stop hyprdrop.service 2>/dev/null || true
    systemctl --user disable hyprdrop.service 2>/dev/null || true
    rm -f "${SYSTEMD_DIR}/hyprdrop.service"
    systemctl --user daemon-reload 2>/dev/null || true

    rm -f "${BIN_DIR}/hyprdrop"
    rm -f "${BIN_DIR}/hyprdrop-detect-selection"
    rm -f "${APP_DIR}/hyprdrop.desktop"
    rm -f "${ICON_DIR}/scalable/apps/hyprdrop.svg"

    for s in 16 24 32 48 64 128 256 512; do
        rm -f "${ICON_DIR}/${s}x${s}/apps/hyprdrop.png"
    done

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "${APP_DIR}" 2>/dev/null || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t "${ICON_DIR}" 2>/dev/null || true
    fi

    echo "==> Hyprdrop uninstalled cleanly."
    exit 0
fi

echo "=========================================================="
echo "           Hyprdrop — Arch Linux / Hyprland Installer     "
echo "=========================================================="
echo "Installation prefix: ${PREFIX}"
echo "Binary directory:    ${BIN_DIR}"
echo ""

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${SYSTEMD_DIR}" "${ICON_DIR}/scalable/apps"

# 1. Build release binary if requested
if [ "$BUILD_RELEASE" = true ]; then
    if [ ! -f "target/release/hyprdrop" ] || [ ! -f "target/release/hyprdrop-detect-selection" ]; then
        echo "==> Compiling optimized release binary with LTO..."
        cargo build --release
    fi
fi

# Ensure helper binary is compiled
if [ ! -f "target/release/hyprdrop-detect-selection" ]; then
    echo "==> Compiling native hyprdrop-detect-selection helper..."
    gcc -O3 c/hyprdrop-detect-selection.c $(pkg-config --cflags --libs atspi-2 gobject-2.0 glib-2.0) -o target/release/hyprdrop-detect-selection
fi

# 2. Install binaries
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
echo "==> Installing desktop entry and high-resolution icons..."
install -Dm644 "assets/hyprdrop.desktop" "${APP_DIR}/hyprdrop.desktop"
install -Dm644 "assets/hyprdrop.svg" "${ICON_DIR}/scalable/apps/hyprdrop.svg"

for s in 16 24 32 48 64 128 256 512; do
    if [ -f "assets/icons/hicolor/${s}x${s}/apps/hyprdrop.png" ]; then
        mkdir -p "${ICON_DIR}/${s}x${s}/apps"
        install -Dm644 "assets/icons/hicolor/${s}x${s}/apps/hyprdrop.png" "${ICON_DIR}/${s}x${s}/apps/hyprdrop.png"
    fi
done

# 4. Install systemd user service
echo "==> Installing systemd user daemon service..."
sed "s|ExecStart=.*|ExecStart=${BIN_DIR}/hyprdrop --daemon|g" assets/hyprdrop.service > "${SYSTEMD_DIR}/hyprdrop.service"

# Reload desktop and icon databases
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_DIR}" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t "${ICON_DIR}" || true
fi

# 5. Dependency Check
echo ""
echo "==> Verifying system dependencies:"
MISSING_OPT=()
for dep in ffmpeg pdftoppm wl-copy tesseract hyprctl; do
    if command -v "$dep" >/dev/null 2>&1; then
        echo "    [✓] $dep (found at $(command -v "$dep"))"
    else
        echo "    [!] $dep (missing - optional for full feature support)"
        MISSING_OPT+=("$dep")
    fi
done

if [ ${#MISSING_OPT[@]} -gt 0 ]; then
    echo ""
    echo "    To install optional tools for OCR, video conversion, and PDF rendering:"
    echo "      Arch Linux:   sudo pacman -S ffmpeg poppler wl-clipboard tesseract tesseract-data-eng"
    echo "      Fedora:       sudo dnf install ffmpeg poppler-utils wl-clipboard tesseract"
    echo "      Ubuntu/Debian: sudo apt install ffmpeg poppler-utils wl-clipboard tesseract-ocr"
fi

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
