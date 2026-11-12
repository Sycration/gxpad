#!/usr/bin/env bash
# Packages the contents of build/ (from ./build.sh) into archives and creates a GitHub release with them.
#
# Usage: ./release.sh [--publish] [--dry-run] [TAG]
#
#   TAG        release tag; defaults to v<version in Cargo.toml>
#   --publish  publishes the release immediately instead of creating a draft
#   --dry-run  only writes the archives to build/dist, without creating a release
#
# Archives: gxpad-TAG-<platform>-<arch>.tar.gz (linux, macos), .zip (windows), gxgeo-TAG-web.tar.gz,
# plus SHA256SUMS. Each one holds a top-level folder with bin/ and lib/ as in build/, LICENSE and README.md.
# The release tag points at the current commit, which must be clean and pushed.
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
  tag=v$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)
fi

die() { echo "error: $*" >&2; exit 1; }

command -v bsdtar >/dev/null || die "bsdtar is not installed (libarchive), it writes the .zip archives"

targets=(linux/x64 linux/arm64 macos/x64 macos/arm64 windows/x64 windows/arm64 web/wasm32)
for t in "${targets[@]}"; do
  [ -d "$BUILD/$t" ] || die "build/$t is missing, run ./build.sh first"
done

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
  if [ "$platform" = web ]; then
    name=gxgeo-$tag-web
  else
    name=gxpad-$tag-$platform-$arch
  fi
  mkdir -p "$stage/$name"
  cp -r "$BUILD/$t/." "$stage/$name/"
  cp LICENSE README.md "$stage/$name/"
  if [ "$platform" = windows ]; then
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
