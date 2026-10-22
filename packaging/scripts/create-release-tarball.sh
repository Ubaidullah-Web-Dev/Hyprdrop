#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Builds release tarball for Hyprdrop GitHub Releases & hyprdrop-bin AUR package
# ==============================================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d '"' -f 2)"
TAR_NAME="hyprdrop-v${VERSION}-linux-x86_64.tar.gz"
DIST_DIR="${ROOT_DIR}/dist"

echo "==> Building Hyprdrop v${VERSION} release artifacts..."
cargo build --release --locked

mkdir -p "${DIST_DIR}"
TMP_PKG="$(mktemp -d)"
trap 'rm -rf "${TMP_PKG}"' EXIT

mkdir -p "${TMP_PKG}/assets"
cp "target/release/hyprdrop" "${TMP_PKG}/"
cp "target/release/hyprdrop-detect-selection" "${TMP_PKG}/"
cp "assets/hyprdrop.desktop" "${TMP_PKG}/assets/"
cp "assets/hyprdrop.svg" "${TMP_PKG}/assets/"
cp -r "assets/icons" "${TMP_PKG}/assets/" 2>/dev/null || true
cp "assets/hyprdrop.service" "${TMP_PKG}/assets/"
cp "README.md" "${TMP_PKG}/"
cp "LICENSE" "${TMP_PKG}/"

echo "==> Creating release archive: ${DIST_DIR}/${TAR_NAME}..."
tar -czf "${DIST_DIR}/${TAR_NAME}" -C "${TMP_PKG}" .

SHA256="$(sha256sum "${DIST_DIR}/${TAR_NAME}" | awk '{print $1}')"
echo "${SHA256}  ${TAR_NAME}" > "${DIST_DIR}/${TAR_NAME}.sha256"

echo ""
echo "==> Package ready:"
echo "    Archive: ${DIST_DIR}/${TAR_NAME}"
echo "    SHA256:  ${SHA256}"
echo ""
echo "==> Updating packaging/aur-bin/PKGBUILD with new version and checksum..."
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" packaging/aur-bin/PKGBUILD
sed -i "s/^sha256sums=.*/sha256sums=('${SHA256}')/" packaging/aur-bin/PKGBUILD
(cd packaging/aur-bin && makepkg --printsrcinfo > .SRCINFO)

echo "==> Updated packaging/aur-bin/.SRCINFO successfully."
