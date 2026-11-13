#!/usr/bin/env bash
# Starts the gxpad.exe built by build.sh on the screen of the XP machine, at the next whole minute, and
# waits until it runs.
#
# Usage: winxp/vm/launch.sh [ARGUMENT...]    (the arguments of gxpad.exe, such as a .gxp file on the XP machine)
#
# A program started over telnet runs on the invisible desktop of the telnet session. The task scheduler of
# XP (`at /interactive`, which needs an administrator) starts it on the screen instead, as the SYSTEM
# account: its settings and recent files go to the registry of SYSTEM, not of the user.
# Settings come from the environment; see README.md.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
dir=${XP_DIR:-'C:\gxpad'}
remote=${dir#*:\\}

# the batch file the scheduler runs: it starts gxpad.exe with the arguments and returns
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
printf '%s\r\n' '@echo off' "cd /d $dir\\winxp\\Release" 'start "" gxpad.exe %*' > "$tmp/launch.bat"
python3 "$here/vm.py" put "$tmp" "$remote" launch.bat

# `at` takes whole minutes and moves a time that has passed to the next day, so the job goes at least
# 15 seconds into the future
t=$(python3 "$here/vm.py" run 'echo T%TIME%' 30 | grep -oE 'T *[0-9]+:[0-9]+:[0-9]+' | tr -d 'T ')
IFS=: read -r h m s <<<"$t"
h=$((10#$h)); m=$((10#$m + 1)); s=$((10#$s))
[ "$s" -ge 45 ] && m=$((m + 1))
if [ $m -ge 60 ]; then m=$((m - 60)); h=$(((h + 1) % 24)); fi
at=$(printf '%02d:%02d' $h $m)
# a job of an earlier launch that has not run yet (moved to the next day) is removed first
for id in $(python3 "$here/vm.py" run at 30 | awk '/launch\.bat/ { for (i = 1; i <= NF; i++) if ($i ~ /^[0-9]+$/) { print $i; break } }'); do
  python3 "$here/vm.py" run "at $id /delete" 30 >/dev/null
done
python3 "$here/vm.py" run "at $at /interactive $dir\\launch.bat $*" 30 >/dev/null
echo "gxpad starts at $at (the time of the XP machine is $t)"
python3 "$here/wait.py"
