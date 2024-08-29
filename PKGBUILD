# Maintainer: ldev <ldev dot eu dot org>
pkgver=2024.8.2
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


NIGHTLY="false"

prepare() {
  cd "$srcdir/smppserver"
  smppgc/gen_js.sh
  export RUSTUP_TOOLCHAIN=stable
  if [[ "$NIGHTLY" == "true" ]] ; then
  export RUSTUP_TOOLCHAIN=nightly
  export RUSTFLAGS="-Z threads=8"
  fi
  cargo fetch
}


build() {
  cd "$srcdir/smppserver"

  export RUSTUP_TOOLCHAIN=stable
  if [[ "$NIGHTLY" == "true" ]] ; then
  export RUSTUP_TOOLCHAIN=nightly
  export RUSTFLAGS="-Z threads=8"
  fi
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
  install_config Rocket.toml 

  install -D -osmppgc -gsmppgc "" "$pkgdir/var/smppgc"
  cp -rf "$srcdir/smppserver/smppgc/www" "$pkgdir/var/smppgc"
}
