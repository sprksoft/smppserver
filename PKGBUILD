# Maintainer: ldev <ldev dot eu dot org>
pkgver=2024.8.1
pkgrel=1

pkgname=(smppgc)
pkgdesc="smppgc"
license=('MIT')
url="https://github.com/Xgames123/smppserver"
arch=('any')

source=("git+file://$(pwd)#tag=$pkgver")
sha256sums=('SKIP')
makedepends=(
cargo
)

prepare() {
  smppgc/gen_js.sh
  cd "$srcdir/smppserver"
  export RUSTUP_TOOLCHAIN=nightly
  cargo fetch
}


build() {
  cd "$srcdir/smppserver"
  export RUSTUP_TOOLCHAIN=nightly
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --workspace
}

install_config() {
  install -d "$pkgdir/etc"
  cp "$pkgname/$1" "$pkgdir/etc/$pkgname.toml"
  chmod 644 "$pkgdir/etc/$pkgname.toml"
}

package() {
  cd "$srcdir/smppserver"
  install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/$pkgname"
  install -Dm0644 -t "$pkgdir/usr/lib/systemd/system/" "$pkgname/$pkgname.service"
  install_config smppgc.toml 

  mkdir -p "$pkgdir/var"
  cd "$srcdir/smppserver/smppgc"
  cp -rf www "$pkgdir/var/smppgc/www"
  chown smppgc:smppgc "$pkgdir/var/smppgc/www"
}
