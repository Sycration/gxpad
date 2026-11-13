#!/usr/bin/env python3
"""Moves files to and from the Windows XP machine over FTP and runs commands on
it over telnet.

  vm.py put LOCAL_ROOT REMOTE_DIR FILE...   uploads each FILE (a path relative to LOCAL_ROOT)
                                            to REMOTE_DIR/FILE, creating directories
  vm.py get REMOTE_PATH LOCAL_FILE          downloads a file
  vm.py run 'COMMAND' [TIMEOUT]             runs a cmd.exe command line, prints its output and
                                            exits with its ERRORLEVEL

Remote paths are relative to the root of the FTP server (C:\\ on the XP machine).
Settings come from the environment; see README.md.
"""
import os
import re
import socket
import subprocess
import sys
import time

HOST = os.environ.get("XP_HOST", "192.168.122.112")
USER = os.environ.get("XP_USER", "user")
PASS = os.environ.get("XP_PASS", "user")
# The FTP server of the XP machine answers PASV with a port its firewall blocks, so
# transfers use active mode: the server connects back to this address.
LOCAL_IP = os.environ.get("XP_LOCAL_IP", "192.168.122.1")

IAC, DONT, DO, WONT, WILL, SB, SE = 255, 254, 253, 252, 251, 250, 240


def ftp_url(path):
    return f"ftp://{HOST}/" + path.replace("\\", "/").lstrip("/").replace(" ", "%20")


def curl(args):
    cmd = ["curl", "-sS", "--max-time", "120", "-P", LOCAL_IP, "--user", f"{USER}:{PASS}"] + args
    # an old server sometimes drops a transfer (426); a second attempt goes through
    for _ in range(3):
        if subprocess.run(cmd).returncode == 0:
            return 0
        time.sleep(1)
    return 1


def recv(s, buf):
    """Reads from the telnet connection, refusing every option the server offers."""
    try:
        data = s.recv(65536)
    except socket.timeout:
        return buf
    if not data:
        raise EOFError(buf)
    out = bytearray()
    i = 0
    while i < len(data):
        b = data[i]
        if b == IAC and i + 1 < len(data):
            cmd = data[i + 1]
            if cmd in (DO, DONT, WILL, WONT) and i + 2 < len(data):
                s.sendall(bytes([IAC, WONT if cmd in (DO, DONT) else DONT, data[i + 2]]))
                i += 3
                continue
            if cmd == SB:
                j = data.find(bytes([IAC, SE]), i)
                i = j + 2 if j >= 0 else len(data)
                continue
            i += 2
            continue
        out.append(b)
        i += 1
    return buf + out.decode("cp437", "replace")


def wait_for(s, pattern, timeout):
    buf = ""
    end = time.time() + timeout
    while time.time() < end:
        buf = recv(s, buf)
        if re.search(pattern, buf):
            return buf
    raise TimeoutError(buf)


def run(cmd, timeout):
    s = socket.create_connection((HOST, 23), timeout=0.5)
    wait_for(s, r"login:", 20)
    s.sendall(USER.encode() + b"\r\n")
    wait_for(s, r"password:", 20)
    s.sendall(PASS.encode() + b"\r\n")
    wait_for(s, r">\s*$", 30)
    s.sendall(f"{cmd} & echo XPDONE%ERRORLEVEL%\r\n".encode())
    buf = wait_for(s, r"XPDONE-?\d+", timeout)
    body = buf.split("\n", 1)[1] if "\n" in buf else buf
    body = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", body).replace("\r", "")
    print(body.rsplit("XPDONE", 1)[0].rstrip())
    code = int(re.search(r"XPDONE(-?\d+)", buf).group(1))
    s.sendall(b"exit\r\n")
    s.close()
    return code


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    what = sys.argv[1]
    if what == "put":
        root, rdir, files = sys.argv[2], sys.argv[3], sys.argv[4:]
        bad = 0
        for f in files:
            if curl(["--ftp-create-dirs", "-T", os.path.join(root, f), ftp_url(rdir + "/" + f)]) != 0:
                print("upload failed:", f, file=sys.stderr)
                bad = 1
        sys.exit(bad)
    if what == "get":
        sys.exit(curl(["-o", sys.argv[3], ftp_url(sys.argv[2])]))
    if what == "run":
        sys.exit(run(sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 300))
    sys.exit(__doc__)


main()
