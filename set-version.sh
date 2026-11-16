#!/usr/bin/env bash
# Sets the version of sgpad, sggeo, sgcli and sgweb everywhere it is written down:
#   sgpad/Cargo.toml, sggeo/Cargo.toml, sgcli/Cargo.toml, sgweb/Cargo.toml
#                                                         version of the four crates (and their entries in
#                                                         Cargo.lock)
#   sgpad/winxp/sgpad.rc                                  VERSIONINFO of the Windows XP program (the version
#                                                         Windows shows in the file properties) and the version on
#                                                         its About box
#
# Usage: ./set-version.sh X.Y.Z
set -euo pipefail

cd "$(dirname "$0")"
v=${1:-}
[[ $v =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] || { echo "usage: ./set-version.sh X.Y.Z" >&2; exit 1; }
commas="${BASH_REMATCH[1]},${BASH_REMATCH[2]},${BASH_REMATCH[3]},0"

# the first `version =` line of each manifest is the one of its [package]
for f in sgpad/Cargo.toml sggeo/Cargo.toml sgcli/Cargo.toml sgweb/Cargo.toml; do
  sed -i -E "0,/^version = \"[^\"]*\"/s//version = \"$v\"/" "$f"
done
cargo update --offline --quiet -p sgpad -p sggeo -p sgcli -p sgweb

sed -i -E \
  -e "s/^( *(FILE|PRODUCT)VERSION) [0-9]+,[0-9]+,[0-9]+,[0-9]+/\1 $commas/" \
  -e "s/(VALUE \"(File|Product)Version\", )\"[^\"]*\"/\1\"$v\"/" \
  -e "s/\"Version [0-9.]+\", IDC_VERSION/\"Version $v\", IDC_VERSION/" \
  sgpad/winxp/sgpad.rc

grep -nE '^version = ' sgpad/Cargo.toml sggeo/Cargo.toml sgcli/Cargo.toml sgweb/Cargo.toml
grep -nE '(FILE|PRODUCT)VERSION [0-9]|(File|Product)Version"|IDC_VERSION' sgpad/winxp/sgpad.rc
