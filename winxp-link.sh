#!/usr/bin/env bash
# Linker of the Windows XP builds of ./build.sh: rustc runs this script in place of MSVC link.exe, and the
# script runs the lld-link of zig with the same arguments.
#
# thunk sets the subsystem version and the entry point only for executables. For a DLL this script adds
# the XP subsystem version (XP refuses to load an image with a newer one) and the YY-Thunks DLL entry
# point, which sets up thread_local storage on XP before the CRT entry point runs.
#
# Environment: WINXP_ARCH (x86 or x64), LIB (library search path, entries separated by ';')
set -euo pipefail

extra=()
for a in "$@"; do
  [ "${a,,}" = /dll ] || continue
  case ${WINXP_ARCH:?} in
    x86) extra=(/subsystem:windows,5.01 /entry:DllMainCRTStartupForYY_Thunks
                "/alternatename:_YY_ThunksOriginalDllMainCRTStartup@12=__DllMainCRTStartup@12") ;;
    x64) extra=(/subsystem:windows,5.02 /entry:DllMainCRTStartupForYY_Thunks
                /alternatename:YY_ThunksOriginalDllMainCRTStartup=_DllMainCRTStartup) ;;
    *) echo "winxp-link.sh: unknown WINXP_ARCH '$WINXP_ARCH'" >&2; exit 1 ;;
  esac
done

exec zig lld-link "${extra[@]}" "$@"
