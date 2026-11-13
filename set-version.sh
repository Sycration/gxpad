#!/usr/bin/env bash
# Sets the version of gxpad and gxgeo everywhere it is written down:
#   Cargo.toml and gxgeo/Cargo.toml   version of the two crates (and their entries in Cargo.lock)
#   winxp/gxpad.rc                    VERSIONINFO of the Windows XP program (the version Windows shows in
#                                     the file properties) and the version on its About box
#
# Usage: ./set-version.sh X.Y.Z
set -euo pipefail

cd "$(dirname "$0")"
v=${1:-}
[[ $v =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || { echo "usage: ./set-version.sh X.Y.Z" >&2; exit 1; }
commas="${BASH_REMATCH[1]},${BASH_REMATCH[2]},${BASH_REMATCH[3]},0"

# the first `version =` line of each manifest is the one of its [package]
for f in Cargo.toml gxgeo/Cargo.toml; do
  sed -i -E "0,/^version = \"[^\"]*\"/s//version = \"$v\"/" "$f"
done
cargo update --offline --quiet -p gxpad -p gxgeo

sed -i -E \
  -e "s/^( *(FILE|PRODUCT)VERSION) [0-9]+,[0-9]+,[0-9]+,[0-9]+/\1 $commas/" \
  -e "s/(VALUE \"(File|Product)Version\", )\"[^\"]*\"/\1\"$v\"/" \
  -e "s/\"Version [0-9.]+\", IDC_VERSION/\"Version $v\", IDC_VERSION/" \
  winxp/gxpad.rc

grep -nE '^version = ' Cargo.toml gxgeo/Cargo.toml
grep -nE '(FILE|PRODUCT)VERSION [0-9]|(File|Product)Version"|IDC_VERSION' winxp/gxpad.rc
