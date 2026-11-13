#!/usr/bin/env bash
# Release builds of gxpad (desktop program) and gxgeo (C library / Wasm module), collected under build/:
#
#   build/<platform>/<arch>/bin   gxpad executable
#   build/<platform>/<arch>/lib   gxgeo shared library, import library, gxgeo.h and the gxgeo README.md
#   build/web/wasm32/lib          gxgeo Wasm module, its wasm-bindgen JS glue and the gxgeo README.md
#   build/winxp/<arch>/lib        gxgeo DLL for Windows XP, its import library, gxgeo.h and the gxgeo README.md
#
# Usage: ./build.sh [linux] [windows] [macos] [web] [winxp]    (no arguments builds everything)
#
# Requirements:
#   - rustup targets: x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu
#     aarch64-pc-windows-gnullvm x86_64-apple-darwin aarch64-apple-darwin wasm32-unknown-unknown
#     i686-pc-windows-msvc x86_64-pc-windows-msvc
#   - linux, windows: zig and cargo-zigbuild
#   - macos: osxcross (its target/bin on PATH, or OSXCROSS_BIN set to that directory)
#   - web: wasm-bindgen-cli matching the wasm-bindgen version in Cargo.lock
#   - winxp: zig, thunk (cargo install thunk-cli), bsdtar, llvm-readobj and llvm-dlltool (llvm), and
#       - THUNK_DIR (default ~/.cache/thunk): a directory holding the VC-LTL binary release
#         (VC-LTL-*Binary.7z) and the YY-Thunks obj release (YY-Thunks-*Objs.zip)
#       - XWIN_DIR (default ~/.xwin): the MSVC CRT and Windows SDK libraries for x86 and x86_64, as written by
#         `xwin --accept-license --arch x86,x86_64 splat --output ~/.xwin` (cargo install xwin)
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
[ ${#platforms[@]} -eq 0 ] && platforms=(linux windows macos web winxp)

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
  # rustc requests libunwind for gnullvm with --unwindlib=libunwind, which zig cc ignores, so the
  # _Unwind_* symbols stay undefined unless libunwind is named explicitly.
  export CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_RUSTFLAGS="-C link-arg=-lunwind"
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

# check_xp_imports DLL EXPORTS: fails when DLL imports a function that is missing from EXPORTS, the list of
# the functions that the DLLs of Windows XP export (in the format of the YY-Thunks Config files)
check_xp_imports() {
  llvm-readobj --coff-imports "$1" | awk -v exports="$2" '
    BEGIN {
      while ((getline l < exports) > 0) {
        sub(/\r$/, "", l)
        if (l ~ /^\[.*\]$/) dll = tolower(substr(l, 2, length(l) - 2))
        else if ((i = index(l, "=")) > 0) have[dll SUBSEP substr(l, i + 1)] = 1
      }
    }
    /^ *Name:/ { dll = tolower($2) }
    /^ *Symbol:/ { if (!((dll SUBSEP $2) in have)) { print "error: Windows XP has no " dll "!" $2 > "/dev/stderr"; bad++ } }
    END { exit bad > 0 }'
}

# The XP builds use thunk (VC-LTL as the CRT, which links against msvcrt.dll, and YY-Thunks for the Windows
# functions that are newer than XP) with the lld-link of zig in place of MSVC link.exe; see winxp-link.sh.
build_winxp() {
  need zig "https://ziglang.org"
  need thunk "cargo install thunk-cli"
  need bsdtar "libarchive"
  need llvm-readobj "llvm"
  need llvm-dlltool "llvm"
  local xwin=${XWIN_DIR:-$HOME/.xwin}
  [ -d "$xwin/crt/lib/x86" ] && [ -d "$xwin/sdk/lib/um/x86" ] \
    || { echo "error: no MSVC CRT or Windows SDK in $xwin (see XWIN_DIR at the top of build.sh)" >&2; exit 1; }
  local thunk_dir=${THUNK_DIR:-$HOME/.cache/thunk} vc_ltl_7z yy_zip
  vc_ltl_7z=$(compgen -G "$thunk_dir/VC-LTL*Binary.7z" | head -n1) \
    || { echo "error: no VC-LTL*Binary.7z in $thunk_dir (see THUNK_DIR at the top of build.sh)" >&2; exit 1; }
  yy_zip=$(compgen -G "$thunk_dir/YY-Thunks*Objs.zip" | head -n1) \
    || { echo "error: no YY-Thunks*Objs.zip in $thunk_dir (see THUNK_DIR at the top of build.sh)" >&2; exit 1; }

  local work=$TARGET_DIR/winxp-thunk
  rm -rf "$work"
  mkdir -p "$work/vc-ltl" "$work/yy-thunks"
  bsdtar -xf "$vc_ltl_7z" -C "$work/vc-ltl" --exclude '*.pdb' \
    TargetPlatform/5.1.2600.0/lib/Win32 TargetPlatform/5.2.3790.0/lib/x64
  bsdtar -xf "$yy_zip" -C "$work/yy-thunks" objs/x86/YY_Thunks_for_WinXP.obj objs/x64/YY_Thunks_for_WinXP.obj \
    Config/x86/5.1.2600.txt Config/x64/5.2.3790.txt

  # x86 is 32-bit XP (5.1), x64 is XP x64 (5.2)
  for spec in x86:i686:x86:x86:5.1.2600:i386 x64:x86_64:x86_64:x64:5.2.3790:i386:x86-64; do
    IFS=: read -r arch cpu xwin_arch yy_arch exports machine <<<"$spec"
    local triple=$cpu-pc-windows-msvc
    echo "==> winxp $arch"

    # The YY-Thunks obj names its Windows SDK libraries with a casing that the files in the SDK do not
    # all have, so a directory of links with the names from the obj precedes the SDK on the search path.
    local obj=$work/yy-thunks/objs/$yy_arch/YY_Thunks_for_WinXP.obj case_dir=$work/sdk-names-$arch name file
    mkdir -p "$case_dir"
    for name in $(grep -aoE '/DEFAULTLIB:"[^"]+"' "$obj" | sed -E 's|.*:"(.*)"|\1|' | sort -u); do
      file=$(find "$xwin/sdk/lib/um/$xwin_arch" -maxdepth 1 -iname "$name" | head -n1)
      [ -n "$file" ] || { echo "error: $name is not in $xwin/sdk/lib/um/$xwin_arch" >&2; exit 1; }
      ln -sf "$file" "$case_dir/$name"
    done

    # thunk always builds into ./target/winxp_build. thunk 0.3.3 exits with 0 also when cargo fails, so the
    # DLL of the previous build is deleted first and a missing DLL marks a failed build.
    local rel=$ROOT/target/winxp_build/$triple/release dest=$OUT/winxp/$arch/lib
    rm -f "$rel/gxgeo.dll"
    # thunk 0.3.3 reads the architecture from --target among the cargo arguments; without it, thunk asks
    # rustup for the host architecture and rejects any host that is not Windows.
    local linker_var=CARGO_TARGET_${triple^^}_LINKER
    (
      export VC_LTL=$work/vc-ltl YY_THUNKS=$work/yy-thunks WINXP_ARCH=$arch
      export LIB="$case_dir;$xwin/crt/lib/$xwin_arch;$xwin/sdk/lib/um/$xwin_arch"
      export "${linker_var//-/_}=$ROOT/winxp-link.sh"
      thunk --os xp --lib -- --release -p gxgeo --lib --target "$triple"
    )
    [ -f "$rel/gxgeo.dll" ] || { echo "error: the winxp $arch build failed" >&2; exit 1; }

    check_xp_imports "$rel/gxgeo.dll" "$work/yy-thunks/Config/$yy_arch/$exports.txt"

    # The imports in the x86 import library from lld-link have the name type "export as", which the
    # linkers of Visual C++ 2008 and older reject, so llvm-dlltool writes the import library from the
    # exports of the DLL instead. The last field of the spec (which contains a ':') is the llvm-dlltool
    # machine. The file name gxgeo.lib follows the MSVC convention for import libraries.
    local def=$work/gxgeo-$arch.def
    { echo "LIBRARY gxgeo.dll"; echo EXPORTS
      llvm-readobj --coff-exports "$rel/gxgeo.dll" | awk '/^ *Name:/ { print "  " $2 }'; } > "$def"
    rm -rf "$dest"
    mkdir -p "$dest"
    llvm-dlltool -m "$machine" -d "$def" -l "$dest/gxgeo.lib"
    cp "$rel/gxgeo.dll" "$HEADER" "$LIB_README" "$dest/"
  done
}

for p in "${platforms[@]}"; do
  case $p in
    linux | windows | macos | web | winxp) ;;
    *) echo "error: unknown platform '$p' (expected linux, windows, macos, web or winxp)" >&2; exit 1 ;;
  esac
done

for p in "${platforms[@]}"; do
  rm -rf "${OUT:?}/$p"
  "build_$p"
done

echo "==> done"
find "$OUT" -type f | sort | sed "s|^$ROOT/||"
