#!/usr/bin/env bash
# Packages the contents of build/ (from ./build.sh) into archives and creates a GitHub release with them.
#
# Usage: ./release.sh [--publish] [--dry-run] [TAG]
#
#   TAG        release tag; defaults to v<version in sgpad/Cargo.toml>
#   --publish  publishes the release immediately instead of creating a draft
#   --dry-run  only writes the archives to build/dist, without creating a release
#
# Archives: sg-TAG-<platform>-<arch>.tar.gz (linux, macos, web) or .zip (windows, winxp), one for each
# directory of build/, plus SHA256SUMS. web-wasm32 holds the sggeo Wasm module and sgweb for Linux and Windows x64,
# winxp-x64 the sggeo DLL and sgcli, and winxp-x86 the Windows XP program besides. Each one holds a top-level folder with bin/ and lib/ as in
# build/ and LICENSE; the archives with sgpad also hold the README.md of the project, or for Windows XP the
# README.md of sgpad/winxp/ (bin/ holds the sgcli README.md and lib/ the sggeo README.md, from ./build.sh).
# The release tag points at the current commit, which must be clean and pushed. ./set-version.sh X.Y.Z
# sets the version beforehand (the crates, and the VERSIONINFO of the Windows XP program).
set -euo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
BUILD=$ROOT/build
DIST=$BUILD/dist

publish=0 dry=0 tag=
for a in "$@"; do
  case $a in
    --publish) publish=1 ;;
    --dry-run) dry=1 ;;
    -*) echo "error: unknown option $a" >&2; exit 1 ;;
    *) tag=$a ;;
  esac
done
if [ -z "$tag" ]; then
  tag=v$(sed -n 's/^version *= *"\(.*\)"/\1/p' sgpad/Cargo.toml | head -n1)
fi

die() { echo "error: $*" >&2; exit 1; }

command -v bsdtar >/dev/null || die "bsdtar is not installed (libarchive), it writes the .zip archives"

targets=(linux/x64 linux/arm64 macos/x64 macos/arm64 windows/x64 windows/arm64 web/wasm32 winxp/x86 winxp/x64)
for t in "${targets[@]}"; do
  [ -d "$BUILD/$t" ] || die "build/$t is missing, run ./build.sh first"
done
[ -f "$BUILD/winxp/x86/bin/sgpad.exe" ] || die "build/winxp/x86/bin/sgpad.exe is missing, run ./build.sh winxp"

commit=$(git rev-parse HEAD)
if [ $dry -eq 0 ]; then
  command -v gh >/dev/null || die "gh is not installed"
  [ -z "$(git status --porcelain)" ] || die "the working tree has uncommitted changes, so the build may not match $tag"
  git fetch --quiet origin
  [ -n "$(git branch -r --contains "$commit")" ] || die "commit ${commit:0:7} is not on any remote branch, push it first"
  ! gh release view "$tag" >/dev/null 2>&1 || die "release $tag already exists"
fi

rm -rf "$DIST"
mkdir -p "$DIST"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

for t in "${targets[@]}"; do
  platform=${t%/*} arch=${t#*/}
  name=sg-$tag-$platform-$arch
  mkdir -p "$stage/$name"
  cp -r "$BUILD/$t/." "$stage/$name/"
  cp LICENSE "$stage/$name/"
  if [ "$platform" = winxp ]; then
    [ ! -f "$BUILD/$t/bin/sgpad.exe" ] || cp sgpad/winxp/README.md "$stage/$name/"
  elif [ -d "$BUILD/$t/bin" ]; then
    cp README.md "$stage/$name/"
  fi
  if [ "$platform" = windows ] || [ "$platform" = winxp ]; then
    (cd "$stage" && bsdtar -a -cf "$DIST/$name.zip" "$name")
  else
    tar -C "$stage" -czf "$DIST/$name.tar.gz" "$name"
  fi
done

(cd "$DIST" && sha256sum -- *.tar.gz *.zip > SHA256SUMS)
ls -lh "$DIST" | tail -n +2

if [ $dry -eq 1 ]; then
  echo "==> dry run, archives are in build/dist"
  exit 0
fi

flags=(--target "$commit" --title "$tag" --generate-notes)
[ $publish -eq 1 ] || flags+=(--draft)
gh release create "$tag" "${flags[@]}" "$DIST"/*
