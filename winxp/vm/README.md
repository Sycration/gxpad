# Working with a Windows XP virtual machine

Scripts that build and try out the Windows program (`winxp/`) on a Windows XP virtual machine with
Visual Studio 2008, from the Linux machine that hosts it. They were made for one libvirt/QEMU
machine; the environment variables below describe another one.

| Script | |
| --- | --- |
| `vm.py` | uploads and downloads files over FTP and runs `cmd.exe` command lines over telnet |
| `build.sh` | uploads the sources and builds them with `vcbuild` (Visual Studio 2008) |
| `launch.sh` | starts the built `gxpad.exe` on the screen of the XP machine and waits until it runs |
| `wait.py` | waits until `gxpad.exe` runs (`launch.sh` uses it) |
| `cycle.sh` | closes gxpad, builds it and starts it again |
| `ui.py` | moves the mouse, clicks and types on the XP screen and takes screenshots |
| `v4proxy.py` | an IPv4-only proxy for downloading the MSVC libraries with `xwin` (see `build.sh`) |

## A typical session

```sh
./build.sh winxp                       # the XP library, which the program links against
winxp/vm/build.sh all                  # first time: sources, bitmaps and the gxgeo files
winxp/vm/launch.sh 'C:\gxpad\some.gxp' # the program on the XP screen, with a drawing
winxp/vm/ui.py click 40 170 drag 200 560 520 560 shot line   # draw a line, save target/xp-shots/line.png
# change the sources, then
winxp/vm/cycle.sh 'C:\gxpad\some.gxp'  # close, build, start again
winxp/vm/vm.py get gxpad/winxp/Release/gxpad.exe build/winxp/x86/bin/gxpad.exe   # for a release
```

`./build.sh` deletes `build/winxp`, so for a release the program goes into `build/winxp/x86/bin` after it,
next to a copy of `build/winxp/x86/lib/gxgeo.dll` (see `winxp/README.md`).

## The XP machine

- Windows XP with Visual Studio 2008 (its `vcbuild`, Visual C++ and Windows SDK 6.0A); the Express edition
  lacks some of the libraries the project uses.
- The telnet server running, and an account that is an administrator (for `at /interactive`).
- An FTP server whose root is `C:\` and where the account may write. With FileZilla Server, passive
  mode needs its data ports open in the firewall of XP; `vm.py` uses active mode instead, where the
  server connects to `XP_LOCAL_IP`.
- For `ui.py`: a USB tablet as the pointing device of the virtual machine (absolute positions), and
  access to libvirt (`virsh -c qemu:///system list` works).

## Settings

| Variable | Default | |
| --- | --- | --- |
| `XP_HOST` | `192.168.122.112` | address of the XP machine |
| `XP_USER`, `XP_PASS` | `user`, `user` | the account for telnet and FTP |
| `XP_LOCAL_IP` | `192.168.122.1` | address of this machine on the network of the XP machine |
| `XP_DIR` | `C:\gxpad` | where the repository files go on the XP machine |
| `XP_VS` | `C:\Program Files\Microsoft Visual Studio 9.0` | the installation of Visual Studio 2008 |
| `XP_DOMAIN` | `winxp` | the libvirt name of the virtual machine |
| `XP_LIBVIRT` | `qemu:///system` | the libvirt connection |
| `XP_SCREEN` | `1280x960` | the screen size of XP, for mouse positions |
| `XP_SHOTS` | `target/xp-shots` | where screenshots go |

## Things to know

- A program started over telnet runs on the invisible desktop of the telnet session, so `launch.sh`
  starts gxpad with the task scheduler (`at /interactive`), which runs it as the SYSTEM account. Its
  settings and recent files are then under `HKEY_USERS\S-1-5-18\Software\gxpad`, and a file saved
  without a folder goes to `C:\Documents and Settings\NetworkService\My Documents`.
- `at` takes whole minutes, so gxpad starts up to a minute and a quarter after `launch.sh`.
- Keys typed by `ui.py` arrive faster than menus open; a `wait 0.5` after a key that opens a menu
  keeps the next key from going elsewhere.
- Someone else may be using the screen of the XP machine: `ui.py` moves the real pointer.
