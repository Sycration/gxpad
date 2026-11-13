#!/usr/bin/env python3
"""Waits up to two and a half minutes until gxpad.exe runs on the XP machine,
then three seconds more for its window to appear. Exits with 1 when it does
not start."""
import os
import subprocess
import sys
import time

VM = os.path.join(os.path.dirname(os.path.abspath(__file__)), "vm.py")

for _ in range(15):
    out = subprocess.run([sys.executable, VM, "run", 'tasklist /fi "imagename eq gxpad.exe"', "30"], capture_output=True, text=True).stdout
    if "gxpad.exe" in out:
        time.sleep(3)
        print("gxpad runs")
        sys.exit(0)
    time.sleep(8)
print("gxpad did not start", file=sys.stderr)
sys.exit(1)
