pkgname=songrec
_pkgname=SongRec
pkgver=0.1.8
pkgrel=1
provides=('songrec')
conflicts=('songrec-git')
pkgdesc='An open-source, unofficial Shazam client for Linux, written in Rust.'
url='https://github.com/marin-m/SongRec'
arch=('x86_64')
license=('GPL3')
makedepends=('cargo' 'rust' 'git')
depends=('gtk3' 'alsa-lib' 'openssl' 'ffmpeg')
optdepends=('libpulse')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/marin-m/songrec/archive/${pkgver}.tar.gz")
sha256sums=('ca1e9b3c6f30524aafdf3e73b308ddee25e8b2f7303d0d17e2f78df7d2bf978d')

build() {
  cd "$_pkgname-$pkgver"
  cargo build --locked --release
}

package() {
  cd "$_pkgname-$pkgver"
  install -Dm755 "${CARGO_TARGET_DIR:-target}/release/songrec" "$pkgdir/usr/bin/songrec"
  install -Dm755 "packaging/rootfs/usr/share/applications/com.github.marinm.songrec.desktop" "$pkgdir/usr/share/applications/com.github.marinm.songrec.desktop"
  install -Dm755 "packaging/rootfs/usr/share/icons/hicolor/scalable/apps/com.github.marinm.songrec.svg" \
                    "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.github.marinm.songrec.svg"
  install -Dm755 "packaging/rootfs/usr/share/metainfo/com.github.marinm.songrec.metainfo.xml" \
                    "$pkgdir/usr/share/metainfo/com.github.marinm.songrec.metainfo.xml"
  install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
