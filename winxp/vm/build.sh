#!/usr/bin/env bash
# Uploads the sources of the Windows program to the XP machine and builds them with Visual Studio 2008.
#
# Usage: winxp/vm/build.sh [all] [rebuild]
#   all      also uploads the bitmaps and the sggeo header, import library and DLL (after ./build.sh winxp)
#   rebuild  rebuilds everything instead of what changed
#
# A running sgpad.exe keeps its file open, so the build first renames it out of the way. The `if` is in
# parentheses: cmd.exe would otherwise take the rest of the line as part of it and skip the build when
# there is no sgpad.exe (after a failed build). A running copy
# also keeps sggeo.dll open; then the build fails at "Copying sggeo.dll" when the DLL changed, although
# sgpad.exe itself was built.
# Settings come from the environment; see README.md.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.."
dir=${XP_DIR:-'C:\sgpad'}
vs=${XP_VS:-'C:\Program Files\Microsoft Visual Studio 9.0'}
remote=${dir#*:\\}  # the same directory, as the FTP server names it

all=0 action=
for a in "$@"; do
  case $a in
    all) all=1 ;;
    rebuild) action=/rebuild ;;
    *) echo "usage: winxp/vm/build.sh [all] [rebuild]" >&2; exit 1 ;;
  esac
done

files=(winxp/sgpad.sln winxp/sgpad.vcproj winxp/sgpad.rc winxp/resource.h winxp/src/*)
if [ $all -eq 1 ]; then
  files+=(winxp/res/sgpad.ico winxp/res/toolbar.bmp sggeo/include/sggeo.h
          build/winxp/x86/lib/sggeo.lib build/winxp/x86/lib/sggeo.dll)
fi
python3 "$here/vm.py" put . "$remote" "${files[@]}"
python3 "$here/vm.py" run "call \"$vs\\Common7\\Tools\\vsvars32.bat\" >nul & cd /d $dir\\winxp & del Release\\sgpad-old-*.exe >nul 2>&1 & (if exist Release\\sgpad.exe ren Release\\sgpad.exe sgpad-old-%RANDOM%.exe) & vcbuild /nologo $action sgpad.sln \"Release|Win32\"" 600 |
  grep -v '^\s*$' | tail -40
