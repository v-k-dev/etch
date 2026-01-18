pkgname=etch
pkgver=0.1.0
pkgrel=1
pkgdesc="A minimal, fast, and reliable USB/ISO writer for Linux"
arch=('x86_64' 'aarch64')
url="https://github.com/v-k-dev/etch"
license=('MIT' 'Apache-2.0')
depends=(
  'gtk4'
  'glib2'
  'pango'
  'cairo'
  'polkit'
)
makedepends=(
  'rust'
  'cargo'
  'pkg-config'
  'libxcb'
)
source=("git+${url}.git#branch=main-stable")
sha256sums=('SKIP')

build() {
  cd "$pkgname"
  cargo build --release --locked
}

package() {
  cd "$pkgname"
  
  # Install binaries
  install -Dm755 target/release/etch "$pkgdir/usr/bin/etch"
  install -Dm755 target/release/etch-helper "$pkgdir/usr/bin/etch-helper"
  install -Dm755 target/release/etch-updater "$pkgdir/usr/bin/etch-updater"
  
  # Desktop entry
  install -Dm644 org.etch.Etch.desktop "$pkgdir/usr/share/applications/org.etch.Etch.desktop"
  
  # AppData metadata
  install -Dm644 org.etch.Etch.appdata.xml "$pkgdir/usr/share/metainfo/org.etch.Etch.appdata.xml"
  
  # Polkit policy
  install -Dm644 org.etch.Etch.policy "$pkgdir/usr/share/polkit-1/actions/org.etch.Etch.policy"
  
  # Icon (1024x1024 PNG)
  install -Dm644 org.etch.Etch.png "$pkgdir/usr/share/icons/hicolor/1024x1024/apps/org.etch.Etch.png"
  
  # License
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE" || true
}
