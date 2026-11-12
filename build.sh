#!/usr/bin/env bash
# Release builds of gxpad (desktop program) and gxgeo (C library / Wasm module), collected under build/:
#
#   build/<platform>/<arch>/bin   gxpad executable
#   build/<platform>/<arch>/lib   gxgeo shared library, import library, gxgeo.h and the gxgeo README.md
#   build/web/wasm32/lib          gxgeo Wasm module, its wasm-bindgen JS glue and the gxgeo README.md
#
# Usage: ./build.sh [linux] [windows] [macos] [web]    (no arguments builds everything)
#
# Requirements:
#   - rustup targets: x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu
#     aarch64-pc-windows-gnullvm x86_64-apple-darwin aarch64-apple-darwin wasm32-unknown-unknown
#   - linux, windows: zig and cargo-zigbuild
#   - macos: osxcross (its target/bin on PATH, or OSXCROSS_BIN set to that directory)
#   - web: wasm-bindgen-cli matching the wasm-bindgen version in Cargo.lock
set -euo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
OUT=$ROOT/build
TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target}
HEADER=$ROOT/gxgeo/include/gxgeo.h
LIB_README=$ROOT/gxgeo/README.md

# Linux binaries link against this glibc version so that they load on older distributions.
GLIBC=2.17
# Minimum macOS version of the macOS binaries.
export MACOSX_DEPLOYMENT_TARGET=11.0

platforms=("$@")
[ ${#platforms[@]} -eq 0 ] && platforms=(linux windows macos web)

need() {
  command -v "$1" >/dev/null || { echo "error: $1 is not installed ($2)" >&2; exit 1; }
}

# collect PLATFORM ARCH TRIPLE EXE LIB...: copies the executable and library files of one target into build/
collect() {
  local platform=$1 arch=$2 triple=$3 exe=$4
  shift 4
  local rel=$TARGET_DIR/$triple/release dest=$OUT/$platform/$arch
  mkdir -p "$dest/bin" "$dest/lib"
  cp "$rel/$exe" "$dest/bin/"
  for f in "$@"; do cp "$rel/$f" "$dest/lib/"; done
  cp "$HEADER" "$LIB_README" "$dest/lib/"
}

build_linux() {
  need cargo-zigbuild "cargo install cargo-zigbuild"
  for pair in x64:x86_64 arm64:aarch64; do
    local arch=${pair%%:*} triple=${pair#*:}-unknown-linux-gnu
    echo "==> linux $arch"
    cargo zigbuild --release --workspace --target "$triple.$GLIBC"
    collect linux "$arch" "$triple" gxpad libgxgeo.so
  done
}

build_windows() {
  need cargo-zigbuild "cargo install cargo-zigbuild"
  for pair in x64:x86_64-pc-windows-gnu arm64:aarch64-pc-windows-gnullvm; do
    local arch=${pair%%:*} triple=${pair#*:}
    echo "==> windows $arch"
    cargo zigbuild --release --workspace --target "$triple"
    collect windows "$arch" "$triple" gxpad.exe gxgeo.dll libgxgeo.dll.a
  done
}

# cargo-zigbuild is not used for macOS: zig prefixes the SDK's library paths with the sysroot a second
# time and then fails to find libobjc and libiconv, so the osxcross clang links instead.
build_macos() {
  local bin=${OSXCROSS_BIN:-}
  if [ -z "$bin" ]; then
    need o64-clang "install osxcross and put its target/bin on PATH, or set OSXCROSS_BIN"
    bin=$(dirname "$(command -v o64-clang)")
  fi
  for pair in x64:x86_64 arm64:aarch64; do
    local arch=${pair%%:*} cpu=${pair#*:} triple=${pair#*:}-apple-darwin
    local clang
    clang=$(compgen -G "$bin/$cpu-apple-darwin*-clang" | head -n1) \
      || { echo "error: no $cpu-apple-darwin*-clang in $bin" >&2; exit 1; }
    local up=${cpu^^}
    echo "==> macos $arch"
    export "CARGO_TARGET_${up}_APPLE_DARWIN_LINKER=$clang" "CC_${cpu}_apple_darwin=$clang"
    cargo build --release -p gxpad --target "$triple"
    # The gxgeo cdylib is linked last and separately so that its install name is @rpath/libgxgeo.dylib
    # instead of the absolute path in target/. Setting it at link time keeps the ld64 ad-hoc signature valid.
    cargo rustc --release -p gxgeo --lib --target "$triple" -- \
      -C link-arg=-Wl,-install_name,@rpath/libgxgeo.dylib
    collect macos "$arch" "$triple" gxpad libgxgeo.dylib
  done
}

build_web() {
  need wasm-bindgen "cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>"
  echo "==> web wasm32"
  cargo build --release -p gxgeo --target wasm32-unknown-unknown
  local dest=$OUT/web/wasm32/lib
  mkdir -p "$dest"
  wasm-bindgen --target web --out-dir "$dest" "$TARGET_DIR/wasm32-unknown-unknown/release/gxgeo.wasm"
  cp "$LIB_README" "$dest/"
}

for p in "${platforms[@]}"; do
  case $p in
    linux | windows | macos | web) ;;
    *) echo "error: unknown platform '$p' (expected linux, windows, macos or web)" >&2; exit 1 ;;
  esac
done

for p in "${platforms[@]}"; do
  rm -rf "${OUT:?}/$p"
  "build_$p"
done

echo "==> done"
find "$OUT" -type f | sort | sed "s|^$ROOT/||"
