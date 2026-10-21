# Maintainer: Hyprdrop Contributors <maintainers@hyprland.org>
pkgname=hyprdrop
pkgver=0.1.0
pkgrel=1
pkgdesc="Radial drag-and-drop file converter, quick-action editor, and drop-shelf utility for Hyprland"
arch=('x86_64')
url="https://github.com/hyprland-community/hyprdrop"
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
makedepends=('cargo' 'pkgconf')
source=("$pkgname-$pkgver.tar.gz")
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
    install -Dm644 "assets/hyprdrop.service" "$pkgdir/usr/lib/systemd/user/hyprdrop.service"
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 "DESIGN.md" "$pkgdir/usr/share/doc/$pkgname/DESIGN.md"
}
