// Helpers: numbers, text, reading the document, the selection and colors.
#include "common.h"

using namespace Gdiplus;

// ------------------------------------------------------------ numbers

double seg_dist(V2 p, V2 a, V2 b, V2 *at) {
    V2 ab = b - a;
    double l2 = ab.x * ab.x + ab.y * ab.y;
    double t = l2 > 0 ? dclamp(((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / l2, 0, 1) : 0;
    V2 q = a + ab * t;
    if (at) *at = q;
    return dist(p, q);
}

V2 foot(V2 p, V2 a, V2 b) {
    V2 d = b - a;
    double l2 = d.x * d.x + d.y * d.y;
    if (l2 < 1e-18) return a;
    double t = ((p.x - a.x) * d.x + (p.y - a.y) * d.y) / l2;
    return a + d * t;
}

wstr fmt_val(double v) {
    if (!finite(v)) return L"\x2014";
    wchar_t buf[64];
    double a = fabs(v);
    if (a != 0 && (a < 1e-4 || a >= 1e7)) {
        swprintf(buf, 64, L"%.4e", v);
        return buf;
    }
    swprintf(buf, 64, L"%.4f", v);
    wstr s = buf;
    while (!s.empty() && s[s.size() - 1] == L'0') s.erase(s.size() - 1);
    if (!s.empty() && s[s.size() - 1] == L'.') s.erase(s.size() - 1);
    if (s == L"-0") s = L"0";
    return s;
}

wstr fmt_dim(double v, bool angle) {
    if (angle) return fmt_val(v * 180.0 / PI) + L"\x00b0";
    return fmt_val(v);
}

// ------------------------------------------------------------ text

wstr widen(const char *s) {
    if (!s || !*s) return wstr();
    int n = MultiByteToWideChar(CP_UTF8, 0, s, -1, 0, 0);
    std::vector<wchar_t> buf(n > 0 ? n : 1);
    MultiByteToWideChar(CP_UTF8, 0, s, -1, &buf[0], n);
    return wstr(&buf[0]);
}

std::string narrow(const wstr &s) {
    if (s.empty()) return std::string();
    int n = WideCharToMultiByte(CP_UTF8, 0, s.c_str(), -1, 0, 0, 0, 0);
    std::vector<char> buf(n > 0 ? n : 1);
    WideCharToMultiByte(CP_UTF8, 0, s.c_str(), -1, &buf[0], n, 0, 0);
    return std::string(&buf[0]);
}

wstr take(char *s) {
    wstr w = widen(s);
    gx_string_free(s);
    return w;
}

wstr last_error() {
    const char *e = gx_last_error();
    return e ? widen(e) : wstr();
}

wstr load_str(UINT id) {
    wchar_t buf[1024];
    int n = LoadStringW(g_inst, id, buf, 1024);
    return wstr(buf, n > 0 ? n : 0);
}

wstr window_text(HWND h) {
    int n = GetWindowTextLengthW(h);
    std::vector<wchar_t> buf(n + 1);
    GetWindowTextW(h, &buf[0], n + 1);
    return wstr(&buf[0]);
}

// ------------------------------------------------------------ the document

namespace geo {

bool live(GxDoc *d, size_t e) { return e != NONE && gx_doc_ent_kind(d, e) >= 0; }
int kind(GxDoc *d, size_t e) { return e == NONE ? -1 : gx_doc_ent_kind(d, e); }
bool is_point(GxDoc *d, size_t e) { return kind(d, e) == GX_ENT_POINT; }
bool is_line(GxDoc *d, size_t e) { return kind(d, e) == GX_ENT_LINE; }
bool is_round(GxDoc *d, size_t e) { int k = kind(d, e); return k == GX_ENT_CIRCLE || k == GX_ENT_ARC; }
bool is_conic(GxDoc *d, size_t e) { return kind(d, e) == GX_ENT_CONIC; }
bool is_function(GxDoc *d, size_t e) { return kind(d, e) == GX_ENT_CURVE; }
bool is_curve(GxDoc *d, size_t e) { int k = kind(d, e); return k > 0; }

bool is_axis(GxDoc *d, size_t e) {
    return e != NONE && ((ptrdiff_t)e == gx_doc_x_axis(d) || (ptrdiff_t)e == gx_doc_y_axis(d));
}

bool is_fixed(GxDoc *d, size_t e) { return e != NONE && ((ptrdiff_t)e == gx_doc_origin(d) || is_axis(d, e)); }
bool hidden(GxDoc *d, size_t e) { return gx_doc_ent_hidden(d, e) != 0; }

bool segment(GxDoc *d, size_t e) {
    size_t a, b;
    bool inf = false;
    return gx_doc_line_pts(d, e, &a, &b, &inf) == 1 && !inf;
}

V2 pos(GxDoc *d, size_t p) {
    V2 v;
    if (p == NONE || gx_doc_point_pos(d, p, &v.x, &v.y) != 1) return nan2();
    return v;
}

void line_pts(GxDoc *d, size_t l, size_t *a, size_t *b) {
    bool inf;
    if (gx_doc_line_pts(d, l, a, b, &inf) != 1) *a = *b = NONE;
}

void line_ends(GxDoc *d, size_t l, V2 *a, V2 *b) {
    size_t pa, pb;
    line_pts(d, l, &pa, &pb);
    *a = pos(d, pa);
    *b = pos(d, pb);
}

V2 line_dir(GxDoc *d, size_t l) {
    V2 a, b;
    line_ends(d, l, &a, &b);
    return b - a;
}

bool circle(GxDoc *d, size_t e, V2 *c, double *r) {
    size_t ci;
    if (gx_doc_circle_vals(d, e, &ci, r) != 1) return false;
    *c = pos(d, ci);
    return true;
}

bool conic(GxDoc *d, size_t e, GxConic *kind, V2 *c, double *a, double *b, double *t) {
    size_t ci;
    if (gx_doc_conic_vals(d, e, kind, &ci, a, b, t) != 1) return false;
    *c = pos(d, ci);
    return true;
}

V2 center(GxDoc *d, size_t e) {
    V2 c;
    double r, a, b, t;
    GxConic k;
    switch (kind(d, e)) {
    case GX_ENT_POINT: return pos(d, e);
    case GX_ENT_CIRCLE:
    case GX_ENT_ARC: if (circle(d, e, &c, &r)) return c; break;
    case GX_ENT_CONIC: if (conic(d, e, &k, &c, &a, &b, &t)) return c; break;
    }
    return nan2();
}

std::vector<size_t> con_ents(GxDoc *d, size_t c) {
    size_t buf[8];
    ptrdiff_t n = gx_doc_con_ents(d, c, buf, 8);
    return n > 0 ? std::vector<size_t>(buf, buf + (n < 8 ? n : 8)) : std::vector<size_t>();
}

std::vector<size_t> dim_ents(GxDoc *d, size_t i) {
    size_t buf[4];
    ptrdiff_t n = gx_doc_dim_ents(d, i, buf, 4);
    return n > 0 ? std::vector<size_t>(buf, buf + (n < 4 ? n : 4)) : std::vector<size_t>();
}

wstr ent_name(GxDoc *d, size_t e) { return take(gx_doc_ent_name(d, e)); }
wstr dim_name(GxDoc *d, size_t i) { return take(gx_doc_dim_name(d, i)); }
wstr var_name(GxDoc *d, size_t i) { return take(gx_doc_var_name(d, i)); }

bool lines_parallel(GxDoc *d, size_t a, size_t b) {
    V2 u = line_dir(d, a), v = line_dir(d, b);
    double cr = u.x * v.y - u.y * v.x;
    return fabs(cr) < 1e-6 * u.len() * v.len();
}

size_t function_index(GxDoc *d, size_t e) {
    size_t n = 0;
    for (size_t i = 0; i < e; i++)
        if (is_function(d, i)) n++;
    return n;
}

} // namespace geo

size_t realize(GxDoc *d, const Snap &s) {
    ptrdiff_t r = -1;
    switch (s.kind) {
    case Snap::POINT: return s.e;
    case Snap::CURVE: r = gx_doc_add_point_on(d, s.e, s.at.x, s.at.y); break;
    case Snap::FREE: r = gx_doc_add_point(d, s.at.x, s.at.y); break;
    }
    return r < 0 ? NONE : (size_t)r;
}

// ------------------------------------------------------------ tools

UINT tool_cmd(Tool t) { return ID_TOOL_FIRST + (UINT)t; }

int cmd_tool(UINT cmd) {
    if (cmd >= ID_TOOL_FIRST && cmd <= ID_TOOL_LAST) return (int)(cmd - ID_TOOL_FIRST);
    return -1;
}

int tool_clicks(Tool t) {
    switch (t) {
    case T_POINT: return 1;
    case T_LINE: case T_CIRCLE: case T_PARABOLA: case T_PARABOLA_FOCUS: return 2;
    case T_ARC: case T_ELLIPSE: case T_HYPERBOLA: case T_CIRCLE3: case T_ARC3:
    case T_ELLIPSE_FOCI: case T_HYPERBOLA_FOCI: return 3;
    default: return 0;
    }
}

// ------------------------------------------------------------ the application

App::App()
    : doc(0), tool(T_SELECT), show_grid(true), show_symbols(true), saved_gen(0), undo_seen(0), redo_steps(0),
      flash_until(0) {}

bool App::dirty() const { return gx_doc_generation(doc) != saved_gen; }

bool App::selected(Sel s) const {
    for (size_t k = 0; k < sel.size(); k++)
        if (sel[k] == s) return true;
    return false;
}

std::vector<size_t> App::sel_ents() const {
    std::vector<size_t> v;
    for (size_t k = 0; k < sel.size(); k++)
        if (sel[k].kind == Sel::ENT) v.push_back(sel[k].i);
    return v;
}

bool App::formula_target(Sel *t) const {
    for (size_t k = sel.size(); k-- > 0;) {
        if (sel[k].kind == Sel::DIM || sel[k].kind == Sel::VAR) {
            *t = sel[k];
            return true;
        }
    }
    return false;
}

void App::select_only(Sel s) {
    sel.clear();
    sel.push_back(s);
}

void App::prune_selection() {
    std::vector<Sel> keep;
    for (size_t k = 0; k < sel.size(); k++) {
        Sel s = sel[k];
        bool ok = false;
        switch (s.kind) {
        case Sel::ENT: ok = gx_doc_ent_kind(doc, s.i) >= 0; break;
        case Sel::CON: ok = gx_doc_con_kind(doc, s.i) >= 0; break;
        case Sel::DIM: ok = gx_doc_dim_kind(doc, s.i) >= 0; break;
        case Sel::VAR: ok = gx_doc_var_free(doc, s.i) >= 0; break;
        }
        if (ok) keep.push_back(s);
    }
    sel = keep;
}

void App::flash_ents(const std::vector<size_t> &ents) {
    flash = ents;
    flash_until = GetTickCount() + 700;
    MessageBeep(MB_ICONASTERISK);
}

bool App::apply(Change &c) {
    GxDoc *trial = gx_doc_copy(doc);
    bool ok = trial && c.run(trial);
    gx_doc_free(trial);
    if (!ok) return false;
    ptrdiff_t len = gx_doc_undo_len(doc);
    if (!c.run(doc)) return false;
    if (len >= 0) gx_doc_join_undo(doc, (size_t)len);
    return true;
}

// ------------------------------------------------------------ colors

namespace col {
const Color paper(255, 255, 255);
const Color grid_minor(255, 232, 238, 247);
const Color grid_major(255, 200, 214, 235);
const Color axis(255, 112, 128, 150);
const Color axis_text(255, 120, 130, 145);
const Color free_ent(255, 0, 0, 200);
const Color fixed_ent(255, 0, 0, 0);
const Color conflict(255, 220, 0, 0);
const Color construction(255, 120, 120, 120);
const Color selected(255, 0, 150, 0);
const Color hover(255, 255, 140, 0);
const Color snap(255, 0, 160, 0);
const Color driving(255, 128, 0, 160);
const Color measured(255, 96, 96, 96);
const Color ghost(160, 0, 100, 220);
const Color functions[6] = {
    Color(255, 0, 140, 130), Color(255, 200, 30, 120), Color(255, 50, 140, 0),
    Color(255, 220, 100, 0), Color(255, 160, 130, 0), Color(255, 70, 90, 220),
};
}
