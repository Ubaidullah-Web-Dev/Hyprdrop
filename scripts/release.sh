#!/usr/bin/env bash
# ==============================================================================
# Hyprdrop Automated Release & Version Synchronization Script
# ==============================================================================
# Usage:
#   ./scripts/release.sh <version|patch|minor|major>
#
# Examples:
#   ./scripts/release.sh 0.1.1
#   ./scripts/release.sh patch
# ==============================================================================
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_DIR}"

CURRENT_VER=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)

# Parse arguments
TARGET=""
CUSTOM_MSG=""
CUSTOM_NOTES=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -m|--message)
            CUSTOM_MSG="$2"
            shift 2
            ;;
        -n|--notes)
            CUSTOM_NOTES="$2"
            shift 2
            ;;
        *)
            if [ -z "$TARGET" ]; then
                TARGET="$1"
            fi
            shift
            ;;
    esac
done

if [ -z "$TARGET" ]; then
    echo "Usage: $0 <version|patch|minor|major> [-m \"commit message\"] [-n \"release notes\"]"
    echo "Current version: ${CURRENT_VER}"
    exit 1
fi

# Parse SemVer
IFS='.' read -r MAJOR MINOR PATCH <<< "${CURRENT_VER}"

case "${TARGET}" in
    patch)
        NEW_VER="${MAJOR}.${MINOR}.$((PATCH + 1))"
        ;;
    minor)
        NEW_VER="${MAJOR}.$((MINOR + 1)).0"
        ;;
    major)
        NEW_VER="$((MAJOR + 1)).0.0"
        ;;
    *)
        # Strip leading 'v' if provided
        NEW_VER="${TARGET#v}"
        if [[ ! "${NEW_VER}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
            echo "[-] Error: Invalid semver format: ${TARGET}"
            exit 1
        fi
        ;;
esac

echo "=========================================================="
echo "  Hyprdrop Automated Release Pipeline: v${CURRENT_VER} -> v${NEW_VER}"
echo "=========================================================="

# 1. Update Cargo.toml
echo "==> Updating Cargo.toml..."
sed -i -E "0,/^version = \".*\"/s//version = \"${NEW_VER}\"/" Cargo.toml

# 2. Update PKGBUILD & .SRCINFO (Root)
if [ -f "PKGBUILD" ]; then
    echo "==> Updating PKGBUILD..."
    sed -i -E "s/^pkgver=.*/pkgver=${NEW_VER}/" PKGBUILD
    sed -i -E "s/pkgrel=.*/pkgrel=1/" PKGBUILD
fi
if [ -f ".SRCINFO" ]; then
    echo "==> Updating .SRCINFO..."
    sed -i -E "s/pkgver = .*/pkgver = ${NEW_VER}/" .SRCINFO
    sed -i -E "s/source = .*refs\/tags\/v.*\.tar\.gz/source = hyprdrop-${NEW_VER}.tar.gz::https:\/\/github.com\/Ubaidullah-Web-Dev\/Hyprdrop\/archive\/refs\/tags\/v${NEW_VER}.tar.gz/" .SRCINFO
fi

# 3. Update packaging/aur-bin/PKGBUILD & .SRCINFO
if [ -f "packaging/aur-bin/PKGBUILD" ]; then
    echo "==> Updating packaging/aur-bin/PKGBUILD..."
    sed -i -E "s/^pkgver=.*/pkgver=${NEW_VER}/" packaging/aur-bin/PKGBUILD
    sed -i -E "s/pkgrel=.*/pkgrel=1/" packaging/aur-bin/PKGBUILD
fi
if [ -f "packaging/aur-bin/.SRCINFO" ]; then
    echo "==> Updating packaging/aur-bin/.SRCINFO..."
    sed -i -E "s/pkgver = .*/pkgver = ${NEW_VER}/" packaging/aur-bin/.SRCINFO
    sed -i -E "s/source = .*hyprdrop-.*-linux-x86_64\.tar\.gz/source = hyprdrop-bin-${NEW_VER}.tar.gz::https:\/\/github.com\/Ubaidullah-Web-Dev\/Hyprdrop\/releases\/download\/v${NEW_VER}\/hyprdrop-v${NEW_VER}-linux-x86_64.tar.gz/" packaging/aur-bin/.SRCINFO
fi

# 4. Update install.sh static fallback
echo "==> Updating install.sh fallback URLs..."
sed -i -E "s|https://github.com/Ubaidullah-Web-Dev/Hyprdrop/releases/download/v[0-9]+\.[0-9]+\.[0-9]+/hyprdrop-v[0-9]+\.[0-9]+\.[0-9]+-linux-x86_64.tar.gz|https://github.com/Ubaidullah-Web-Dev/Hyprdrop/releases/download/v${NEW_VER}/hyprdrop-v${NEW_VER}-linux-x86_64.tar.gz|g" install.sh

# 5. Update README.md version strings
echo "==> Updating README.md tarball references..."
sed -i -E "s/hyprdrop-v[0-9]+\.[0-9]+\.[0-9]+-linux-x86_64\.tar\.gz/hyprdrop-v${NEW_VER}-linux-x86_64.tar.gz/g" README.md
sed -i -E "s/cd hyprdrop-v[0-9]+\.[0-9]+\.[0-9]+-linux-x86_64/cd hyprdrop-v${NEW_VER}-linux-x86_64/g" README.md

# 6. Cargo test & build release binary
echo "==> Running cargo test..."
cargo test

echo "==> Building optimized release binary with LTO..."
cargo build --release

# 7. Package distribution tarballs
echo "==> Packaging distribution tarballs in dist/..."
mkdir -p dist
TMP_STAGE="$(mktemp -d)"
trap 'rm -rf "${TMP_STAGE}"' EXIT

cp target/release/hyprdrop "${TMP_STAGE}/"
cp target/release/hyprdrop-detect-selection "${TMP_STAGE}/"
cp LICENSE "${TMP_STAGE}/"
cp README.md "${TMP_STAGE}/"
cp -r assets "${TMP_STAGE}/"

TARBALL_VER="dist/hyprdrop-v${NEW_VER}-linux-x86_64.tar.gz"
TARBALL_LATEST="dist/hyprdrop-linux-x86_64.tar.gz"

tar -czf "${TARBALL_VER}" -C "${TMP_STAGE}" .
cp "${TARBALL_VER}" "${TARBALL_LATEST}"

# 8. Compute SHA256 Checksums
echo "==> Calculating sha256 checksums..."
SHA_VER=$(sha256sum "${TARBALL_VER}" | cut -d' ' -f1)
echo "${SHA_VER}  hyprdrop-v${NEW_VER}-linux-x86_64.tar.gz" > "${TARBALL_VER}.sha256"

SHA_LATEST=$(sha256sum "${TARBALL_LATEST}" | cut -d' ' -f1)
echo "${SHA_LATEST}  hyprdrop-linux-x86_64.tar.gz" > "${TARBALL_LATEST}.sha256"

echo "    [✓] SHA256: ${SHA_VER}"

# Update PKGBUILD & .SRCINFO with new checksum
if [ -f "packaging/aur-bin/PKGBUILD" ]; then
    sed -i -E "s/sha256sums=\('.*'\)/sha256sums=\('${SHA_VER}'\)/" packaging/aur-bin/PKGBUILD
fi
if [ -f "packaging/aur-bin/.SRCINFO" ]; then
    sed -i -E "s/sha256sums = .*/sha256sums = ${SHA_VER}/" packaging/aur-bin/.SRCINFO
fi

# 9. Update local system binary & restart daemon
if [ -d "${HOME}/.local/bin" ]; then
    echo "==> Updating local user installation in ~/.local/bin..."
    install -Dm755 target/release/hyprdrop "${HOME}/.local/bin/hyprdrop"
    install -Dm755 target/release/hyprdrop-detect-selection "${HOME}/.local/bin/hyprdrop-detect-selection"
    systemctl --user daemon-reload || true
    systemctl --user restart hyprdrop.service || true
fi

# 10. Commit & Tag Git
if [ -z "${CUSTOM_MSG}" ]; then
    if [ -t 0 ] || [ -c /dev/tty ]; then
        read -r -p "Enter commit message [default: chore(release): bump version to v${NEW_VER}]: " USER_MSG < /dev/tty || true
        CUSTOM_MSG="${USER_MSG:-chore(release): bump version to v${NEW_VER}}"
    else
        CUSTOM_MSG="chore(release): bump version to v${NEW_VER}"
    fi
fi

if [ -z "${CUSTOM_NOTES}" ]; then
    PREV_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [ -n "${PREV_TAG}" ]; then
        CHANGELOG=$(git log "${PREV_TAG}..HEAD" --oneline --no-merges 2>/dev/null || true)
    else
        CHANGELOG=$(git log -n 5 --oneline --no-merges 2>/dev/null || true)
    fi

    if [ -t 0 ] || [ -c /dev/tty ]; then
        echo ""
        echo "Recent commits since last release:"
        echo "${CHANGELOG}"
        echo ""
        read -r -p "Enter release notes summary (or press Enter to use recent commits): " USER_NOTES < /dev/tty || true
        if [ -n "${USER_NOTES}" ]; then
            CUSTOM_NOTES="${USER_NOTES}"
        else
            CUSTOM_NOTES="${CHANGELOG}"
        fi
    else
        CUSTOM_NOTES="${CHANGELOG}"
    fi
fi

echo "==> Committing version bump and tagging v${NEW_VER}..."
git add Cargo.toml Cargo.lock PKGBUILD .SRCINFO packaging/ install.sh README.md src/ tests/ docs/ scripts/
git commit -m "${CUSTOM_MSG}" || echo "    (Nothing new to commit)"
git tag -f "v${NEW_VER}"

echo "==> Pushing commits and tag to remote origin..."
git push origin main
git push -f origin "v${NEW_VER}"

# 11. GitHub Release Asset Upload (if token available)
TOKEN=""
if [ -f "${HOME}/.git-credentials" ]; then
    TOKEN=$(grep "github.com" "${HOME}/.git-credentials" | sed -E 's/.*:(.*)@github.com/\1/' || true)
fi

if [ -n "${TOKEN}" ]; then
    echo "==> Syncing release assets to GitHub (Release v${NEW_VER})..."
    
    # Check if release exists
    RELEASE_DATA=$(curl -s -H "Authorization: token ${TOKEN}" "https://api.github.com/repos/Ubaidullah-Web-Dev/Hyprdrop/releases/tags/v${NEW_VER}" || true)
    RELEASE_ID=$(echo "${RELEASE_DATA}" | jq -r '.id // empty' 2>/dev/null || true)

    RELEASE_BODY="### Hyprdrop v${NEW_VER} Release

${CUSTOM_NOTES}"

    if [ -z "${RELEASE_ID}" ]; then
        echo "    Creating new release v${NEW_VER} on GitHub..."
        CREATE_PAYLOAD=$(jq -n \
            --arg tag "v${NEW_VER}" \
            --arg name "Hyprdrop v${NEW_VER} — Native Wayland Radial File Studio" \
            --arg body "${RELEASE_BODY}" \
            '{tag_name: $tag, name: $name, body: $body, draft: false, prerelease: false}')
        RELEASE_DATA=$(curl -s -X POST -H "Authorization: token ${TOKEN}" \
            -H "Content-Type: application/json" \
            -d "${CREATE_PAYLOAD}" \
            "https://api.github.com/repos/Ubaidullah-Web-Dev/Hyprdrop/releases")
        RELEASE_ID=$(echo "${RELEASE_DATA}" | jq -r '.id // empty' 2>/dev/null || true)
    fi

    if [ -n "${RELEASE_ID}" ]; then
        echo "    Uploading release assets to Release ID ${RELEASE_ID}..."
        # Delete existing assets with same names if re-releasing
        EXISTING_ASSETS=$(curl -s -H "Authorization: token ${TOKEN}" "https://api.github.com/repos/Ubaidullah-Web-Dev/Hyprdrop/releases/${RELEASE_ID}/assets" | jq -r '.[] | "\(.id):\(.name)"' 2>/dev/null || true)
        
        for file in "${TARBALL_VER}" "${TARBALL_VER}.sha256" "${TARBALL_LATEST}" "${TARBALL_LATEST}.sha256"; do
            fname="$(basename "${file}")"
            for item in ${EXISTING_ASSETS}; do
                aid="${item%%:*}"
                aname="${item##*:}"
                if [ "${aname}" = "${fname}" ]; then
                    echo "    Removing prior asset ${fname} (id ${aid})..."
                    curl -s -X DELETE -H "Authorization: token ${TOKEN}" "https://api.github.com/repos/Ubaidullah-Web-Dev/Hyprdrop/releases/assets/${aid}" >/dev/null || true
                fi
            done

            ctype="application/gzip"
            if [[ "${fname}" == *.sha256 ]]; then
                ctype="text/plain"
            fi
            echo "    Uploading ${fname}..."
            curl -s -X POST -H "Authorization: token ${TOKEN}" \
                -H "Content-Type: ${ctype}" \
                --data-binary "@${file}" \
                "https://uploads.github.com/repos/Ubaidullah-Web-Dev/Hyprdrop/releases/${RELEASE_ID}/assets?name=${fname}" >/dev/null || true
        done
        echo "    [✓] All release assets uploaded successfully to GitHub!"
    else
        echo "    [!] Warning: Could not find or create release on GitHub."
    fi
else
    echo "    [i] No GitHub token in ~/.git-credentials. Skipped direct asset upload."
fi

echo ""
echo "=========================================================="
echo "  Hyprdrop v${NEW_VER} successfully released & synchronized! "
echo "=========================================================="
