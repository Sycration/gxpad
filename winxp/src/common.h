// Declarations shared by the source files of gxpad for Windows: the window
// classes, the drawing state and the helpers around the gxgeo C interface.
#ifndef GXPAD_COMMON_H
#define GXPAD_COMMON_H

#define WINVER 0x0501
#define _WIN32_WINNT 0x0501
#define _WIN32_IE 0x0600
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#define WIN32_LEAN_AND_MEAN
#define _CRT_SECURE_NO_WARNINGS

#include <windows.h>
#include <windowsx.h>
#include <commctrl.h>
#include <commdlg.h>
#include <shellapi.h>
#include <objidl.h>
#include <gdiplus.h>
#include <process.h>
#include <float.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <algorithm>
#include <string>
#include <vector>
#include <map>
#include <set>

#include "gxgeo.h"
#include "../resource.h"

typedef std::wstring wstr;

// ------------------------------------------------------------ numbers

const double PI = 3.14159265358979323846;
const double TAU = 2.0 * PI;
const size_t NONE = (size_t)-1;

inline double dmin(double a, double b) { return a < b ? a : b; }
inline double dmax(double a, double b) { return a > b ? a : b; }
inline double dclamp(double v, double lo, double hi) { return v < lo ? lo : (v > hi ? hi : v); }
inline bool finite(double v) { return _finite(v) != 0; }
inline double dhypot(double x, double y) { return sqrt(x * x + y * y); }
inline int iround(double v) { return (int)floor(v + 0.5); }

struct V2 {
    double x, y;
    V2() : x(0), y(0) {}
    V2(double x_, double y_) : x(x_), y(y_) {}
    V2 operator+(const V2 &o) const { return V2(x + o.x, y + o.y); }
    V2 operator-(const V2 &o) const { return V2(x - o.x, y - o.y); }
    V2 operator*(double k) const { return V2(x * k, y * k); }
    double len() const { return dhypot(x, y); }
    bool ok() const { return finite(x) && finite(y); }
};

inline V2 nan2() { double n = sqrt(-1.0); return V2(n, n); }
inline double dist(V2 a, V2 b) { return (a - b).len(); }

// Distance from p to the segment ab, and the nearest point of it in `at`.
double seg_dist(V2 p, V2 a, V2 b, V2 *at);
// Foot of the perpendicular from p onto the line through a and b.
V2 foot(V2 p, V2 a, V2 b);

// "1.25", "-3", "1.2346e+08": four decimals at most, without trailing zeros.
wstr fmt_val(double v);
// fmt_val, in degrees with a degree sign for angles.
wstr fmt_dim(double v, bool angle);

// ------------------------------------------------------------ text

wstr widen(const char *s);
std::string narrow(const wstr &s);
// The string returned by the library, released after conversion.
wstr take(char *s);
// Message of the last failed library call.
wstr last_error();
wstr load_str(UINT id);
wstr window_text(HWND h);

// ------------------------------------------------------------ the document

// Shorthands for reading the document. They return a neutral value (false,
// NaN, NONE) when the index does not name a live item.
namespace geo {
bool live(GxDoc *d, size_t e);
int kind(GxDoc *d, size_t e);
bool is_point(GxDoc *d, size_t e);
bool is_line(GxDoc *d, size_t e);
bool is_round(GxDoc *d, size_t e);   // circle or arc
bool is_conic(GxDoc *d, size_t e);
bool is_function(GxDoc *d, size_t e);
bool is_curve(GxDoc *d, size_t e);   // anything but a point
bool is_fixed(GxDoc *d, size_t e);   // origin or an axis
bool is_axis(GxDoc *d, size_t e);
bool hidden(GxDoc *d, size_t e);
bool segment(GxDoc *d, size_t e);
V2 pos(GxDoc *d, size_t p);
void line_pts(GxDoc *d, size_t l, size_t *a, size_t *b);
void line_ends(GxDoc *d, size_t l, V2 *a, V2 *b);
V2 line_dir(GxDoc *d, size_t l);
bool circle(GxDoc *d, size_t e, V2 *c, double *r);
bool conic(GxDoc *d, size_t e, GxConic *kind, V2 *c, double *a, double *b, double *t);
V2 center(GxDoc *d, size_t e);
std::vector<size_t> con_ents(GxDoc *d, size_t c);
std::vector<size_t> dim_ents(GxDoc *d, size_t i);
wstr ent_name(GxDoc *d, size_t e);
wstr dim_name(GxDoc *d, size_t i);
wstr var_name(GxDoc *d, size_t i);
bool lines_parallel(GxDoc *d, size_t a, size_t b);
// Index among the function curves, for picking a color.
size_t function_index(GxDoc *d, size_t e);
}

// ------------------------------------------------------------ selection

struct Sel {
    enum Kind { ENT, CON, DIM, VAR };
    Kind kind;
    size_t i;
    Sel() : kind(ENT), i(0) {}
    Sel(Kind k, size_t i_) : kind(k), i(i_) {}
    bool operator==(const Sel &o) const { return kind == o.kind && i == o.i; }
    bool operator!=(const Sel &o) const { return !(*this == o); }
};

// Where a click lands: on an existing point, on a curve, or on empty paper.
struct Snap {
    enum Kind { POINT, CURVE, FREE };
    Kind kind;
    size_t e;
    V2 at;
    Snap() : kind(FREE), e(NONE) {}
    static Snap point(size_t e, V2 at) { Snap s; s.kind = POINT; s.e = e; s.at = at; return s; }
    static Snap curve(size_t e, V2 at) { Snap s; s.kind = CURVE; s.e = e; s.at = at; return s; }
    static Snap paper(V2 at) { Snap s; s.at = at; return s; }
};

// The point a snap names, adding a free point or a point on the curve when
// it names none. NONE on failure.
size_t realize(GxDoc *d, const Snap &s);

// ------------------------------------------------------------ the application

enum Tool {
    T_SELECT, T_POINT, T_LINE, T_CIRCLE, T_CIRCLE3, T_ARC, T_ARC3, T_ELLIPSE, T_HYPERBOLA, T_PARABOLA,
    T_ELLIPSE_FOCI, T_HYPERBOLA_FOCI, T_PARABOLA_FOCUS, T_DIMENSION,
    T_MIDPOINT, T_INTERSECT, T_PERP_LINE, T_PAR_LINE, T_PERP_BISECTOR, T_ANGLE_BISECTOR, T_TANGENT_LINE,
    T_REFLECT, T_FOCI, T_DIRECTRIX, T_ASYMPTOTES,
    T_COUNT
};

// The command of each tool (ID_TOOL_*), and the tool of a command (-1 when none).
UINT tool_cmd(Tool t);
int cmd_tool(UINT cmd);
// Clicks a drawing tool needs before it adds its shape.
int tool_clicks(Tool t);

// The document and everything the windows show of it.
struct App {
    GxDoc *doc;
    wstr path;
    std::vector<Sel> sel;
    Tool tool;
    bool show_grid, show_symbols;
    uint64_t saved_gen;
    size_t undo_seen;  // undo history length when last looked at
    int redo_steps;    // undos that redo can take back
    std::vector<size_t> flash; // entities shown in red after a rejected change
    DWORD flash_until;

    App();
    bool dirty() const;
    bool selected(Sel s) const;
    std::vector<size_t> sel_ents() const;
    // The dimension or variable the formula pane shows: the last one selected.
    bool formula_target(Sel *t) const;
    void select_only(Sel s);
    void prune_selection();
    void flash_ents(const std::vector<size_t> &ents);
    // Runs `run` on a copy of the document first; when it succeeds there, runs
    // it on the document and joins its undo steps into one. False when it fails.
    struct Change {
        virtual ~Change() {}
        virtual bool run(GxDoc *d) = 0;
    };
    bool apply(Change &c);
};

extern App app;
extern HINSTANCE g_inst;
extern HWND g_frame;
extern HFONT g_ui_font;   // the message font of the system (Tahoma 8 pt on XP)

// Called after every change of the document or the selection: repaints the
// views and updates the menus, toolbars, lists and the formula.
void doc_changed();
void sel_changed();
void set_status(const wstr &text);
void set_tool(Tool t);

// ------------------------------------------------------------ windows

// The 16 x 16 images of res/toolbar.bmp, in this order.
enum Icon {
    I_SELECT, I_POINT, I_LINE, I_CIRCLE, I_CIRCLE3, I_ARC, I_ARC3, I_ELLIPSE, I_HYPERBOLA, I_PARABOLA,
    I_ELLIPSE_FOCI, I_HYPERBOLA_FOCI, I_PARABOLA_FOCUS, I_FUNCTION, I_DIMENSION,
    I_MIDPOINT, I_INTERSECT, I_PERP_LINE, I_PAR_LINE, I_PERP_BISECTOR, I_ANGLE_BISECTOR, I_TANGENT_LINE,
    I_REFLECT, I_FOCI, I_DIRECTRIX, I_ASYMPTOTES,
    I_COINCIDENT, I_ON_CURVE, I_MIDPOINT_CON, I_HORIZONTAL, I_VERTICAL, I_PARALLEL, I_PERPENDICULAR,
    I_COLLINEAR, I_EQUAL, I_TANGENT, I_CONCENTRIC, I_SYMMETRIC, I_LOCK, I_CONSTRUCTION, I_INFINITE,
    I_DISTANCE, I_HDIST, I_VDIST, I_LENGTH, I_RADIUS, I_DIAMETER, I_ANGLE, I_COORD_X, I_COORD_Y,
    I_AXIS_A, I_AXIS_B,
    I_NEW, I_OPEN, I_SAVE, I_PRINT, I_UNDO, I_REDO, I_DELETE, I_ZOOM_IN, I_ZOOM_OUT, I_FIT,
    I_PROPERTIES, I_HELP, I_VARIABLE, I_COPY,
    I_COUNT
};
// The images of res/toolbar.bmp (main.cpp loads them).
extern HIMAGELIST g_icons;

// canvas.cpp
HWND canvas_create(HWND parent);
void canvas_fit();
void canvas_zoom(double factor);
void canvas_set_zoom(double percent);
double canvas_zoom_percent();
V2 canvas_mouse();               // the mouse in world units, NaN when outside
void canvas_refresh();          // after the document changed
void canvas_cancel();           // drops the clicks of the current tool
wstr canvas_prompt();           // what the current tool asks for
// Prints the drawing, fitted into `r` (in device units) of `dc`.
void canvas_print(HDC dc, RECT r);
// Copies the drawing to the clipboard as a picture.
void canvas_copy_picture();
// Places the label of a new dimension of kind `kind` on a and b, adds it and
// opens its Properties when it sets a value.
void add_dimension(GxDim kind, size_t a, size_t b, bool place_auto, V2 label);

// pane.cpp
HWND pane_create(HWND parent);
void pane_refresh();            // lists
void pane_formula_update();     // starts the derivation for the selection
void pane_search_longer();
bool pane_has_formula();
void pane_copy_formula(bool latex);
void pane_copy_picture();
void pane_save_picture();
void pane_layout();
void pane_cancel_jobs();
bool pane_can_search_longer();
// Prints the formula at 12 pt with its top left at (x, y), at most w wide.
void pane_print_formula(HDC dc, int x, int y, int w, int dpi);

// dialogs.cpp
// Each returns true when the user pressed OK and the change went through.
bool dlg_dimension(HWND owner, size_t i, bool fresh);
bool dlg_variable(HWND owner, size_t i);   // NONE adds a variable
bool dlg_function(HWND owner, size_t e);   // NONE adds a function
bool dlg_point(HWND owner, size_t e);
bool dlg_shape(HWND owner, size_t e);
void dlg_about(HWND owner);
void dlg_help(HWND owner);
// Properties of the selection: the dialog that fits it, if any.
bool dlg_properties(HWND owner);

// commands.cpp: menu commands that work on the selection
struct Action;
// True when command `cmd` (a constraint or dimension command) applies to the
// selection; `checked` tells whether the menu shows a check mark.
bool action_enabled(UINT cmd, bool *checked);
void action_run(UINT cmd);
void delete_selection();
void select_all();

// Colors of the drawing.
namespace col {
extern const Gdiplus::Color paper, grid_minor, grid_major, axis, axis_text;
extern const Gdiplus::Color free_ent, fixed_ent, conflict, construction, selected, hover, snap;
extern const Gdiplus::Color driving, measured, ghost;
extern const Gdiplus::Color functions[6];
}

const UINT WM_APP_FORMULA = WM_APP + 1;

#endif
