# gxpad for Windows

A native Windows version of gxpad in the style of Windows XP programs: menus and toolbars, a tool
box like Paint's, Properties dialogs, right-click menus, printing and a help window. It draws
constructions with constraints and dimensions and shows the formula of the selected measured
dimension, with the gxgeo library (`gxgeo.dll`) doing the geometry and typesetting the formulas.

It runs on Windows XP (SP2 or later; the processor needs SSE2) and on later versions of Windows. It is
a 32-bit program, so it also runs on Windows XP x64.

## Contents

| File | |
| --- | --- |
| `bin/gxpad.exe` | the program |
| `bin/gxgeo.dll` | the geometry library it uses; keep it next to `gxpad.exe` |
| `lib/` | the library for C programs (see `lib/README.md`) |

Drawings are saved as `.gxp` files, the same format the other versions of gxpad open.

## Using it

1. Draw with the tools in the tool box, or from the **Insert** and **Construct** menus. The status
   bar says what to click next. Lines and circles can also be drawn by dragging.
2. Select shapes (Shift or Ctrl adds to the selection) and choose a constraint on the **Constrain**
   menu or toolbar.
3. Add dimensions from the **Dimension** menu, or with the Dimension tool: click one or two shapes,
   then where the label goes. A driving dimension sets a value; a measured one only reports it.
4. Select a measured dimension: the **Formula** box at the right shows its formula in terms of the
   driving dimensions and variables.

**Help > Help Topics** (F1) explains each part.

## Building

The program is a Visual Studio 2008 project in C++ and the Win32 API, with GDI+ for drawing. It
builds with the Visual C++ 2008 compiler and its Windows SDK 6.0A, links the C runtime statically
and needs no redistributable.

1. Build the XP library with `./build.sh winxp` (see the top of `build.sh`). The project takes
   `gxgeo.h` from `gxgeo/include` and `gxgeo.lib` and `gxgeo.dll` from `build/winxp/x86/lib`.
2. On Windows with Visual Studio 2008, open `winxp/gxpad.sln` and build the Release
   configuration, or run in a Visual Studio 2008 Command Prompt:

   ```bat
   cd winxp
   vcbuild gxpad.sln "Release|Win32"
   ```

   The build writes `winxp/Release/gxpad.exe` and copies `gxgeo.dll` next to it.
3. For a release, copy both files to `build/winxp/x86/bin`; `./release.sh` then packs them as
   `gxpad-TAG-winxp-x86.zip`.

`vm/` holds scripts that do these steps on a Windows XP virtual machine from the Linux machine that
hosts it, and drive the program there to try it out; see [vm/README.md](vm/README.md).

`res/toolbar.bmp` (the toolbar and tool box images) and `res/gxpad.ico` come from
`res/make_icons.py`; run `python3 make_icons.py` in `res/` (requires Pillow) after changing the
images.

The source files:

| File | |
| --- | --- |
| `src/main.cpp` | the main window: menus, toolbars, tool box, status bar, files, printing, settings |
| `src/canvas.cpp` | the drawing: painting, scrolling and zooming, selecting and dragging, the tools |
| `src/pane.cpp` | the side pane: lists of dimensions, variables and functions, and the formula |
| `src/commands.cpp` | the Constrain and Dimension commands |
| `src/dialogs.cpp` | the dialog boxes and the help topics |
| `src/util.cpp` | text conversion, reading the document, the selection, colors |
| `gxpad.rc`, `resource.h` | menus, accelerators, dialogs and the status bar texts |

Settings and the list of recent files are stored under `HKEY_CURRENT_USER\Software\gxpad`.
