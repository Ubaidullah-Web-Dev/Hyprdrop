#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Hyprdrop — Automated AUR Synchronizer
# Pushes PKGBUILD & .SRCINFO to AUR repositories (hyprdrop-bin and hyprdrop-git)
# ==============================================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="${1:-all}"

sync_package() {
    local PKG_NAME="$1"
    local PKG_DIR="$2"

    echo "=========================================================="
    echo "==> Synchronizing ${PKG_NAME} with AUR..."
    echo "=========================================================="

    if [ ! -d "${PKG_DIR}" ]; then
        echo "Error: Directory ${PKG_DIR} not found!"
        exit 1
    fi

    local WORK_DIR="$(mktemp -d)"
    trap 'rm -rf "${WORK_DIR}"' RETURN

    echo "==> Cloning ssh://aur@aur.archlinux.org/${PKG_NAME}.git..."
    if ! git clone "ssh://aur@aur.archlinux.org/${PKG_NAME}.git" "${WORK_DIR}/${PKG_NAME}" 2>/dev/null; then
        echo "Note: Remote repository may not be initialized yet. Creating fresh clone target..."
        mkdir -p "${WORK_DIR}/${PKG_NAME}"
        cd "${WORK_DIR}/${PKG_NAME}"
        git init
        git remote add origin "ssh://aur@aur.archlinux.org/${PKG_NAME}.git"
    else
        cd "${WORK_DIR}/${PKG_NAME}"
    fi

    echo "==> Copying PKGBUILD & .SRCINFO..."
    cp "${PKG_DIR}/PKGBUILD" "${WORK_DIR}/${PKG_NAME}/"
    (cd "${WORK_DIR}/${PKG_NAME}" && makepkg --printsrcinfo > .SRCINFO)

    git add PKGBUILD .SRCINFO

    if git diff-index --quiet HEAD 2>/dev/null; then
        echo "==> No changes detected for ${PKG_NAME}. Already up to date."
        return 0
    fi

    local VERSION="$(grep -m1 '^pkgver=' PKGBUILD | cut -d '=' -f 2)"
    git commit -m "chore(release): bump ${PKG_NAME} to ${VERSION}"
    echo "==> Pushing ${PKG_NAME} to AUR..."
    git push origin master || git push origin main

    echo "==> Successfully pushed ${PKG_NAME} to AUR!"
}

case "${TARGET}" in
    bin)
        sync_package "hyprdrop-bin" "${ROOT_DIR}/packaging/aur-bin"
        ;;
    git)
        sync_package "hyprdrop-git" "${ROOT_DIR}/packaging/aur-git"
        ;;
    all)
        sync_package "hyprdrop-bin" "${ROOT_DIR}/packaging/aur-bin"
        sync_package "hyprdrop-git" "${ROOT_DIR}/packaging/aur-git"
        ;;
    *)
        echo "Usage: $0 [bin|git|all]"
        exit 1
        ;;
esac
