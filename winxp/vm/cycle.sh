#!/usr/bin/env bash
# Closes gxpad on the XP machine, builds it again with build.sh and starts it with launch.sh: the loop of
# changing the program and looking at the change.
#
# Usage: winxp/vm/cycle.sh [ARGUMENT...]    (the arguments of gxpad.exe)
# Settings come from the environment; see README.md.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
python3 "$here/vm.py" run 'taskkill /f /im gxpad.exe >nul 2>&1 & echo closed' 30
out=$("$here/build.sh" 2>&1) || true
echo "$out" | grep -E 'error|warning C|Projects succeeded' | head -30
echo "$out" | grep -q '1 Projects succeeded' || exit 1
"$here/launch.sh" "$@"
