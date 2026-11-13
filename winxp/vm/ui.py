#!/usr/bin/env python3
"""Moves the mouse and types on the screen of the Windows XP virtual machine
(through QMP, the QEMU monitor that libvirt gives access to) and takes
screenshots of it. Coordinates are pixels of the XP screen.

  move X Y                 moves the mouse
  click X Y                left click; rclick: right click; dclick: double click
  cclick X Y               left click with Ctrl held down
  drag X1 Y1 X2 Y2         drags with the left button
  key COMBO                presses keys, such as ctrl-z, alt-f4, enter, esc, del, f1
  type TEXT                types text
  wait SECONDS             waits
  shot NAME                saves the screen as NAME.png in $XP_SHOTS

Commands follow each other on one command line, for example
  ui.py click 40 170 drag 200 560 520 560 wait 0.5 shot after-line
Settings come from the environment; see README.md.
"""
import json
import os
import subprocess
import sys
import time

DOMAIN = os.environ.get("XP_DOMAIN", "winxp")
LIBVIRT = os.environ.get("XP_LIBVIRT", "qemu:///system")
W, H = (int(v) for v in os.environ.get("XP_SCREEN", "1280x960").split("x"))
SHOTS = os.environ.get("XP_SHOTS", os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../target/xp-shots"))


def qmp(events):
    cmd = {"execute": "input-send-event", "arguments": {"events": events}}
    r = subprocess.run(["virsh", "-c", LIBVIRT, "qemu-monitor-command", DOMAIN, json.dumps(cmd)], capture_output=True, text=True)
    if r.returncode != 0 or "error" in r.stdout:
        print(r.stdout, r.stderr, file=sys.stderr)


def at(x, y):
    """The absolute position of the USB tablet, which spans 0 to 32767 on each axis."""
    return [{"type": "abs", "data": {"axis": "x", "value": round(x * 32767 / (W - 1))}},
            {"type": "abs", "data": {"axis": "y", "value": round(y * 32767 / (H - 1))}}]


def btn(b, down):
    return {"type": "btn", "data": {"down": down, "button": b}}


def key(k, down):
    return {"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}}


# characters that are not their own QEMU key code
CHARS = {" ": "spc", ".": "dot", ",": "comma", "-": "minus", "=": "equal", "/": "slash", "*": "kp_multiply",
         ";": "semicolon", "\\": "backslash", "'": "apostrophe", ":": ("shift", "semicolon"),
         "(": ("shift", "9"), ")": ("shift", "0"), "^": ("shift", "6"), "+": ("shift", "equal"), "_": ("shift", "minus")}
NAMES = {"ctrl": "ctrl", "shift": "shift", "alt": "alt", "enter": "ret", "esc": "esc", "del": "delete", "tab": "tab",
         "home": "home", "bksp": "backspace"}


def press(combo):
    keys = ["minus"] if combo == "-" else [NAMES.get(k, k) for k in combo.split("-")]
    for k in keys:
        qmp([key(k, True)])
    for k in reversed(keys):
        qmp([key(k, False)])
    time.sleep(0.05)


def typ(text):
    for ch in text:
        m = CHARS.get(ch)
        if isinstance(m, tuple):
            press(m[0] + "-" + m[1])
        elif m:
            press(m)
        elif ch.isupper():
            press("shift-" + ch.lower())
        else:
            press(ch)


def shot(name):
    os.makedirs(SHOTS, exist_ok=True)
    # virsh writes the PNG that QEMU hands it, whatever the file is called
    path = os.path.join(SHOTS, name + ".png")
    subprocess.run(["virsh", "-c", LIBVIRT, "screenshot", DOMAIN, path], capture_output=True)
    print(os.path.normpath(path))


def click(x, y, b="left", times=1):
    qmp(at(x, y))
    time.sleep(0.05)
    for _ in range(times):
        qmp([btn(b, True)])
        time.sleep(0.03)
        qmp([btn(b, False)])
        time.sleep(0.05)
    time.sleep(0.15)


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    i = 0
    while i < len(a):
        c = a[i]
        if c == "move":
            qmp(at(float(a[i + 1]), float(a[i + 2])))
            i += 3
        elif c in ("click", "rclick", "dclick"):
            click(float(a[i + 1]), float(a[i + 2]), "right" if c == "rclick" else "left", 2 if c == "dclick" else 1)
            i += 3
        elif c == "cclick":
            qmp([key("ctrl", True)])
            click(float(a[i + 1]), float(a[i + 2]))
            qmp([key("ctrl", False)])
            i += 3
        elif c == "drag":
            x1, y1, x2, y2 = map(float, a[i + 1:i + 5])
            qmp(at(x1, y1))
            time.sleep(0.05)
            qmp([btn("left", True)])
            time.sleep(0.05)
            for k in range(1, 11):
                qmp(at(x1 + (x2 - x1) * k / 10, y1 + (y2 - y1) * k / 10))
                time.sleep(0.03)
            qmp([btn("left", False)])
            time.sleep(0.15)
            i += 5
        elif c == "key":
            press(a[i + 1])
            i += 2
        elif c == "type":
            typ(a[i + 1])
            i += 2
        elif c == "wait":
            time.sleep(float(a[i + 1]))
            i += 2
        elif c == "shot":
            shot(a[i + 1])
            i += 2
        else:
            sys.exit(f"unknown command {c}\n\n{__doc__}")


main()
