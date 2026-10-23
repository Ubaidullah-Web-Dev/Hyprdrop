# Maintainer: Ubaid <ubaidullah.web.dev@gmail.com>
pkgname=hyprdrop
pkgver=0.2.0
pkgrel=1
pkgdesc="Radial drag-and-drop file converter, quick-action studio, and drop-shelf utility for Hyprland"
arch=('x86_64')
url="https://github.com/Ubaidullah-Web-Dev/Hyprdrop"
license=('GPL-3.0-or-later')
depends=(
    'gtk4'
    'libadwaita'
    'gtk4-layer-shell'
    'cairo'
    'ffmpeg'
    'at-spi2-core'
)
optdepends=(
    'poppler: for high-DPI PDF page rasterization (pdftoppm)'
    'tesseract: for OCR text extraction from images'
    'wl-clipboard: for direct Wayland clipboard copy support'
    'pandoc: for document format conversion (Markdown/DOCX)'
    'hyprland: for native compositor overlay blur and cursor anchoring'
)
makedepends=('cargo' 'pkgconf' 'gcc')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/Ubaidullah-Web-Dev/Hyprdrop/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
    cargo build --release --locked
}

check() {
    cargo test --release --locked
}

package() {
    install -Dm755 "target/release/hyprdrop" "$pkgdir/usr/bin/hyprdrop"
    install -Dm755 "target/release/hyprdrop-detect-selection" "$pkgdir/usr/bin/hyprdrop-detect-selection"
    install -Dm644 "assets/hyprdrop.desktop" "$pkgdir/usr/share/applications/hyprdrop.desktop"
    install -Dm644 "assets/hyprdrop.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/hyprdrop.svg"

    for s in 16 24 32 48 64 128 256 512; do
        if [ -f "assets/icons/hicolor/${s}x${s}/apps/hyprdrop.png" ]; then
            install -Dm644 "assets/icons/hicolor/${s}x${s}/apps/hyprdrop.png" "$pkgdir/usr/share/icons/hicolor/${s}x${s}/apps/hyprdrop.png"
        fi
    done

    install -Dm644 "assets/hyprdrop.service" "$pkgdir/usr/lib/systemd/user/hyprdrop.service"
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
