// The drawing: painting, scrolling and zooming, selecting and dragging, and
// the drawing and construction tools.
#include "common.h"

using namespace Gdiplus;

namespace {

const double HIT_PT = 7.0, HIT_CURVE = 5.0, SNAP_PT = 9.0;
const double UNITS_PER_INCH = 2.54;  // a unit of the drawing is a centimetre at 100 %

// The part of the plane a window shows.
struct View {
    double cx, cy, s;  // center in world units, pixels per unit
    double w, h;       // size in pixels
    double ox, oy;     // top left corner on the device
    V2 to_s(V2 p) const { return V2(ox + w / 2 + (p.x - cx) * s, oy + h / 2 - (p.y - cy) * s); }
    V2 to_w(V2 p) const { return V2(cx + (p.x - ox - w / 2) / s, cy - (p.y - oy - h / 2) / s); }
    GxWindow window() const {
        GxWindow g;
        g.x0 = cx - w / 2 / s;
        g.x1 = cx + w / 2 / s;
        g.y0 = cy - h / 2 / s;
        g.y1 = cy + h / 2 / s;
        g.px = 1.0 / s;
        return g;
    }
};

typedef std::vector<V2> Poly;
typedef std::vector<Poly> Polys;

enum DragKind { D_NONE, D_POINT, D_TRANSLATE, D_CIRCLE_EDGE, D_HANDLE, D_CURVE, D_DIM_LABEL, D_PAN, D_BOX };

struct Hover {
    bool any;
    Sel s;
    Hover() : any(false) {}
};

HWND hwnd;
View view;
double base_s;  // pixels per unit at 100 %

// screen polylines of the entities, for the view and document they were made for
std::map<size_t, Polys> shapes;
uint64_t shapes_gen = (uint64_t)-1;
double shapes_key[5];

std::vector<std::pair<RECT, size_t> > dim_rects, glyph_rects;
struct Handle { V2 at; size_t e; int which; };
std::vector<Handle> handles;

Hover hover;
int hover_handle = -1;
V2 mouse, mouse_w;
bool mouse_in;

// dragging with the left button (or panning with the others)
DragKind drag = D_NONE;
bool pressed, dragging, moved_doc;
POINT press_at;
WPARAM press_keys;
Sel press_sel;
bool press_hit;
size_t drag_e;
int drag_which;
V2 grab;
std::vector<size_t> drag_pts;
std::vector<V2> drag_from;
V2 drag_start_w;
V2 pan_last;

// the clicks of the current tool so far
std::vector<Snap> pending;
std::vector<size_t> picks;
bool press_added;  // the press that is down added a click (drawing by dragging)

// ------------------------------------------------------------ helpers

bool rect_has(const RECT &r, V2 p) { return p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom; }

double nearest_on(const Polys &ps, V2 p, V2 *at) {
    double best = 1e300;
    for (size_t i = 0; i < ps.size(); i++) {
        const Poly &pl = ps[i];
        if (pl.size() == 1) {
            double d = dist(p, pl[0]);
            if (d < best) { best = d; if (at) *at = pl[0]; }
        }
        for (size_t k = 1; k < pl.size(); k++) {
            V2 q;
            double d = seg_dist(p, pl[k - 1], pl[k], &q);
            if (d < best) { best = d; if (at) *at = q; }
        }
    }
    return best;
}

Polys samples(GxDoc *d, const View &v, size_t e) {
    Polys out;
    GxWindow win = v.window();
    size_t n_polys = 0;
    ptrdiff_t n = gx_doc_ent_samples(d, e, win, 0, 0, 0, 0, &n_polys);
    if (n <= 0 || n_polys == 0) return out;
    std::vector<double> xy(2 * n);
    std::vector<size_t> starts(n_polys);
    gx_doc_ent_samples(d, e, win, &xy[0], n, &starts[0], n_polys, &n_polys);
    for (size_t k = 0; k < n_polys; k++) {
        size_t a = starts[k], b = k + 1 < n_polys ? starts[k + 1] : (size_t)n;
        Poly pl;
        for (size_t i = a; i < b; i++) {
            V2 q = v.to_s(V2(xy[2 * i], xy[2 * i + 1]));
            if (q.ok()) pl.push_back(q);
        }
        if (!pl.empty()) out.push_back(pl);
    }
    return out;
}

void build_shapes() {
    GxDoc *d = app.doc;
    double key[5] = {view.cx, view.cy, view.s, view.w, view.h};
    uint64_t gen = gx_doc_generation(d);
    if (gen == shapes_gen && memcmp(key, shapes_key, sizeof key) == 0) return;
    shapes.clear();
    size_t n = gx_doc_ent_count(d);
    for (size_t e = 0; e < n; e++) {
        if (!geo::live(d, e) || geo::hidden(d, e)) continue;
        shapes[e] = samples(d, view, e);
    }
    shapes_gen = gen;
    memcpy(shapes_key, key, sizeof key);
}

void invalidate() { InvalidateRect(hwnd, 0, FALSE); }

Color with_alpha(const Color &c, BYTE a) { return Color(a, c.GetR(), c.GetG(), c.GetB()); }

Color lighter(const Color &c, double k) {
    return Color(c.GetA(), (BYTE)(c.GetR() + (255 - c.GetR()) * k), (BYTE)(c.GetG() + (255 - c.GetG()) * k),
                 (BYTE)(c.GetB() + (255 - c.GetB()) * k));
}

PointF pf(V2 p) { return PointF((REAL)p.x, (REAL)p.y); }

void draw_poly(Graphics &g, const Poly &pl, const Pen &pen) {
    if (pl.size() < 2) return;
    std::vector<PointF> pts(pl.size());
    for (size_t i = 0; i < pl.size(); i++) pts[i] = pf(pl[i]);
    g.DrawLines(&pen, &pts[0], (INT)pts.size());
}

void dot(Graphics &g, V2 p, double r, const Color &c) {
    SolidBrush b(c);
    g.FillEllipse(&b, (REAL)(p.x - r), (REAL)(p.y - r), (REAL)(2 * r), (REAL)(2 * r));
}

void ring(Graphics &g, V2 p, double r, const Color &c, double w) {
    Pen pen(c, (REAL)w);
    g.DrawEllipse(&pen, (REAL)(p.x - r), (REAL)(p.y - r), (REAL)(2 * r), (REAL)(2 * r));
}

void arrow(Graphics &g, V2 tip, V2 dir, const Color &c, double k) {
    double l = dir.len();
    if (!(l > 1e-9)) return;
    V2 u = dir * (1 / l), n(-u.y, u.x);
    double a = 7 * k, b = 2.6 * k;
    PointF pts[3] = {pf(tip), pf(tip - u * a + n * b), pf(tip - u * a - n * b)};
    SolidBrush br(c);
    g.FillPolygon(&br, pts, 3);
}

// ------------------------------------------------------------ hit testing

Hover hit(V2 m) {
    Hover h;
    GxDoc *d = app.doc;
    for (size_t k = dim_rects.size(); k-- > 0;) {
        RECT r = dim_rects[k].first;
        InflateRect(&r, 2, 2);
        if (rect_has(r, m)) { h.any = true; h.s = Sel(Sel::DIM, dim_rects[k].second); return h; }
    }
    if (app.show_symbols) {
        for (size_t k = glyph_rects.size(); k-- > 0;) {
            if (rect_has(glyph_rects[k].first, m)) { h.any = true; h.s = Sel(Sel::CON, glyph_rects[k].second); return h; }
        }
    }
    double best = 1e300;
    size_t be = NONE;
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (!geo::is_point(d, it->first) || it->second.empty() || it->second[0].empty()) continue;
        double dd = dist(it->second[0][0], m);
        if (dd < HIT_PT && dd < best) { best = dd; be = it->first; }
    }
    if (be != NONE) { h.any = true; h.s = Sel(Sel::ENT, be); return h; }
    best = 1e300;
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (geo::is_point(d, it->first)) continue;
        double dd = nearest_on(it->second, m, 0);
        double thr = geo::is_axis(d, it->first) ? HIT_CURVE * 0.7 : HIT_CURVE;
        if (dd < thr && (dd < best - 0.5 || (fabs(dd - best) <= 0.5 && it->first > be))) { best = dd; be = it->first; }
    }
    if (be != NONE) { h.any = true; h.s = Sel(Sel::ENT, be); }
    return h;
}

int hit_handle(V2 m) {
    for (size_t k = 0; k < handles.size(); k++)
        if (dist(handles[k].at, m) < HIT_PT) return (int)k;
    return -1;
}

// The curve (not a point) nearest to m within reach, or NONE; `pred` filters.
size_t curve_at(V2 m, bool (*pred)(GxDoc *, size_t)) {
    double best = 1e300;
    size_t be = NONE;
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (!geo::is_curve(app.doc, it->first) || (pred && !pred(app.doc, it->first))) continue;
        double dd = nearest_on(it->second, m, 0);
        if (dd < HIT_CURVE + 2 && dd < best) { best = dd; be = it->first; }
    }
    return be;
}

Snap snap_at(V2 m) {
    GxDoc *d = app.doc;
    double best = 1e300;
    size_t be = NONE;
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (!geo::is_point(d, it->first) || it->second.empty() || it->second[0].empty()) continue;
        double dd = dist(it->second[0][0], m);
        if (dd < SNAP_PT && dd < best) { best = dd; be = it->first; }
    }
    if (be != NONE) return Snap::point(be, geo::pos(d, be));
    best = 1e300;
    V2 bq;
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (geo::is_point(d, it->first)) continue;
        V2 q;
        double dd = nearest_on(it->second, m, &q);
        if (dd < HIT_CURVE && dd < best) { best = dd; be = it->first; bq = q; }
    }
    if (be != NONE) {
        V2 w = view.to_w(bq);
        if (geo::is_line(d, be)) {
            V2 a, b;
            geo::line_ends(d, be, &a, &b);
            w = foot(view.to_w(m), a, b);
        }
        return Snap::curve(be, w);
    }
    return Snap::paper(view.to_w(m));
}

// ------------------------------------------------------------ building shapes

// A line between two clicks snaps to horizontal or vertical within 3 degrees.
int hv_of(const Snap &a, const Snap &b) {
    if (a.kind == Snap::POINT && b.kind == Snap::POINT) return -1;
    V2 d = b.at - a.at;
    if (d.len() * view.s < 12) return -1;
    double ang = fmod(atan2(d.y, d.x) * 180 / PI + 360.0, 180.0);
    if (ang < 3 || ang > 177) return 1;
    if (fabs(ang - 90) < 3) return 0;
    return -1;
}

Snap aligned(const Snap &a, Snap b, int hv) {
    if (hv < 0 || b.kind != Snap::FREE) return b;
    if (hv == 1) b.at.y = a.at.y; else b.at.x = a.at.x;
    return b;
}

bool same_point(const Snap &a, const Snap &b) { return a.kind == Snap::POINT && b.kind == Snap::POINT && a.e == b.e; }

// Adds the shape of drawing tool `t` from its clicks `s` to `d`. Returns the
// new entity, or -1.
ptrdiff_t build_shape(GxDoc *d, Tool t, const std::vector<Snap> &s) {
    for (size_t i = 0; i < s.size(); i++)
        for (size_t j = i + 1; j < s.size(); j++)
            if (same_point(s[i], s[j])) return -1;
    size_t a, b, c;
    switch (t) {
    case T_POINT:
        if (s[0].kind == Snap::POINT) return -1;
        return (ptrdiff_t)realize(d, s[0]);
    case T_LINE: {
        int hv = hv_of(s[0], s[1]);
        Snap e = aligned(s[0], s[1], hv);
        if ((a = realize(d, s[0])) == NONE || (b = realize(d, e)) == NONE) return -1;
        return hv < 0 ? gx_doc_add_line(d, a, b, false) : gx_doc_add_line_aligned(d, a, b, false, hv == 1);
    }
    case T_CIRCLE:
        if ((c = realize(d, s[0])) == NONE) return -1;
        if (s[1].kind == Snap::FREE) {
            double r = dist(geo::pos(d, c), s[1].at);
            return r > 1e-9 ? gx_doc_add_circle(d, c, r) : -1;
        }
        if ((b = realize(d, s[1])) == NONE) return -1;
        return gx_doc_add_circle_to(d, c, b);
    case T_ARC:
        if ((c = realize(d, s[0])) == NONE || (a = realize(d, s[1])) == NONE) return -1;
        if (s[2].kind == Snap::POINT) return gx_doc_add_arc(d, c, a, s[2].e);
        return gx_doc_add_arc_toward(d, c, a, s[2].at.x, s[2].at.y);
    case T_ELLIPSE:
    case T_HYPERBOLA:
        if ((c = realize(d, s[0])) == NONE) return -1;
        return gx_doc_add_conic_from(d, t == T_ELLIPSE ? GX_CONIC_ELLIPSE : GX_CONIC_HYPERBOLA, c, s[1].at.x, s[1].at.y,
                                     s[2].at.x, s[2].at.y);
    case T_PARABOLA:
        if ((c = realize(d, s[0])) == NONE) return -1;
        return gx_doc_add_parabola_toward(d, c, s[1].at.x, s[1].at.y);
    case T_CIRCLE3:
    case T_ARC3:
    case T_ELLIPSE_FOCI:
    case T_HYPERBOLA_FOCI:
        if ((a = realize(d, s[0])) == NONE || (b = realize(d, s[1])) == NONE || (c = realize(d, s[2])) == NONE) return -1;
        if (t == T_CIRCLE3) return gx_doc_add_circle_through(d, a, b, c, 0);
        if (t == T_ARC3) return gx_doc_add_arc_through(d, a, b, c, 0);
        return gx_doc_add_conic_by_foci(d, t == T_ELLIPSE_FOCI ? GX_CONIC_ELLIPSE : GX_CONIC_HYPERBOLA, a, b, c);
    case T_PARABOLA_FOCUS:
        if (s[1].kind != Snap::CURVE || !geo::is_line(d, s[1].e)) return -1;
        if ((a = realize(d, s[0])) == NONE) return -1;
        return gx_doc_add_parabola_by_focus(d, a, s[1].e);
    default:
        return -1;
    }
}

struct ShapeChange : App::Change {
    Tool t;
    std::vector<Snap> s;
    ptrdiff_t made;
    bool run(GxDoc *d) { made = build_shape(d, t, s); return made >= 0; }
};

// A construction from the tool's picks and clicks.
struct ConstructChange : App::Change {
    Tool t;
    std::vector<size_t> picks;
    std::vector<Snap> s;
    V2 m;
    bool run(GxDoc *d) {
        size_t a, b;
        size_t out[4];
        switch (t) {
        case T_MIDPOINT:
            if (!picks.empty()) return gx_doc_add_line_midpoint(d, picks[0]) >= 0;
            if (same_point(s[0], s[1]) || (a = realize(d, s[0])) == NONE || (b = realize(d, s[1])) == NONE) return false;
            return gx_doc_add_midpoint(d, a, b) >= 0;
        case T_INTERSECT: return gx_doc_add_intersection_near(d, picks[0], picks[1], m.x, m.y) >= 0;
        case T_PERP_LINE:
        case T_PAR_LINE:
            if ((a = realize(d, s[0])) == NONE) return false;
            return (t == T_PERP_LINE ? gx_doc_add_perp_line(d, picks[0], a) : gx_doc_add_parallel_line(d, picks[0], a)) >= 0;
        case T_PERP_BISECTOR:
            if (!picks.empty()) {
                geo::line_pts(d, picks[0], &a, &b);
                return gx_doc_add_perp_bisector(d, a, b, 0) >= 0;
            }
            if (same_point(s[0], s[1]) || (a = realize(d, s[0])) == NONE || (b = realize(d, s[1])) == NONE) return false;
            return gx_doc_add_perp_bisector(d, a, b, 0) >= 0;
        case T_ANGLE_BISECTOR: return gx_doc_add_angle_bisector(d, picks[0], picks[1], m.x, m.y, 0) >= 0;
        case T_TANGENT_LINE:
            if ((a = realize(d, s[0])) == NONE) return false;
            return gx_doc_add_tangent_line_near(d, a, picks[0], m.x, m.y, 0) >= 0;
        case T_REFLECT:
            if ((a = realize(d, s[0])) == NONE) return false;
            return gx_doc_add_reflection(d, a, picks[0]) >= 0;
        case T_FOCI: return gx_doc_add_foci(d, picks[0], out, 4) > 0;
        case T_DIRECTRIX: return gx_doc_add_directrix(d, picks[0], m.x, m.y) >= 0;
        case T_ASYMPTOTES: return gx_doc_add_asymptotes(d, picks[0], out, 4) > 0;
        default: return false;
        }
    }
};

bool is_hyperbola(GxDoc *d, size_t e) {
    GxConic k;
    V2 c;
    double a, b, t;
    return geo::conic(d, e, &k, &c, &a, &b, &t) && k == GX_CONIC_HYPERBOLA;
}

bool round_or_conic_or_function(GxDoc *d, size_t e) { return geo::is_curve(d, e) && !geo::is_line(d, e); }

std::vector<size_t> snap_ents(const std::vector<Snap> &s) {
    std::vector<size_t> v;
    for (size_t i = 0; i < s.size(); i++)
        if (s[i].kind != Snap::FREE) v.push_back(s[i].e);
    return v;
}

void reset_clicks() {
    pending.clear();
    picks.clear();
}

void report_failure(const std::vector<size_t> &ents) {
    wstr e = last_error();
    set_status(e.empty() ? load_str(IDS_READY) : L"Could not add it: " + e);
    app.flash_ents(ents);
}

void finish(bool ok, const std::vector<size_t> &ents) {
    reset_clicks();
    if (ok) {
        doc_changed();
        set_status(canvas_prompt());
    } else {
        report_failure(ents);
        invalidate();
    }
}

// The dimension the dimension tool would add for the picked entities, with
// the mouse at world position m.
bool dim_kind_for(const std::vector<size_t> &e, V2 m, GxDim *kind, size_t *a, size_t *b) {
    GxDoc *d = app.doc;
    *b = 0;
    if (e.size() == 1) {
        *a = e[0];
        if (geo::is_point(d, e[0])) {
            V2 q = geo::pos(d, e[0]);
            *kind = fabs(m.y - q.y) > fabs(m.x - q.x) ? GX_DIM_X : GX_DIM_Y;
            return true;
        }
        if (geo::is_line(d, e[0])) { *kind = GX_DIM_LENGTH; return true; }
        if (geo::is_round(d, e[0])) { *kind = GX_DIM_RADIUS; return true; }
        if (geo::is_conic(d, e[0])) { *kind = GX_DIM_CONIC_A; return true; }
        return false;
    }
    if (e.size() != 2) return false;
    size_t p = e[0], q = e[1];
    if (geo::is_point(d, p) && geo::is_point(d, q)) { *kind = GX_DIM_DISTANCE; *a = p; *b = q; return true; }
    if (geo::is_line(d, p) && geo::is_point(d, q)) std::swap(p, q);
    if (geo::is_point(d, p) && geo::is_line(d, q)) { *kind = GX_DIM_POINT_LINE; *a = p; *b = q; return true; }
    if (geo::is_line(d, p) && geo::is_line(d, q)) {
        if (geo::lines_parallel(d, p, q)) {
            size_t pa, pb;
            geo::line_pts(d, q, &pa, &pb);
            *kind = GX_DIM_POINT_LINE;
            *a = pa;
            *b = p;
        } else {
            *kind = GX_DIM_ANGLE;
            *a = p;
            *b = q;
        }
        return true;
    }
    return false;
}

void tool_click(V2 m, bool from_press) {
    GxDoc *d = app.doc;
    Snap s = snap_at(m);
    V2 mw = view.to_w(m);
    Tool t = app.tool;
    int need = tool_clicks(t);
    if (need > 0) {
        if (t == T_PARABOLA_FOCUS && pending.size() == 1) {
            size_t l = curve_at(m, geo::is_line);
            if (l == NONE) return;
            s = Snap::curve(l, mw);
        }
        if (t == T_LINE && pending.size() == 1) s = aligned(pending[0], s, hv_of(pending[0], s));
        pending.push_back(s);
        press_added = from_press;
        if ((int)pending.size() < need) {
            invalidate();
            set_status(canvas_prompt());
            return;
        }
        ShapeChange c;
        c.t = t;
        c.s = pending;
        bool ok = app.apply(c);
        std::vector<size_t> ents = snap_ents(pending);
        if (!ok && need == 3 && (t == T_CIRCLE3 || t == T_ARC3 || t == T_ELLIPSE_FOCI || t == T_HYPERBOLA_FOCI)) {
            // a third click that determines nothing is dropped; another can follow
            pending.pop_back();
            report_failure(ents);
            invalidate();
            return;
        }
        finish(ok, ents);
        return;
    }
    ConstructChange c;
    c.t = t;
    c.m = mw;
    size_t hv_line = curve_at(m, geo::is_line);
    size_t hv_curve = curve_at(m, 0);
    switch (t) {
    case T_MIDPOINT:
    case T_PERP_BISECTOR:
        if (pending.empty() && hv_line != NONE && geo::segment(d, hv_line) && s.kind != Snap::POINT) {
            c.picks.push_back(hv_line);
            break;
        }
        pending.push_back(s);
        if (pending.size() < 2) { invalidate(); set_status(canvas_prompt()); return; }
        c.s = pending;
        break;
    case T_INTERSECT: {
        // the curves under the cursor, nearest first
        std::vector<std::pair<double, size_t> > close_by;
        for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
            if (!geo::is_curve(d, it->first)) continue;
            double dd = nearest_on(it->second, m, 0);
            if (dd < HIT_CURVE + 4) close_by.push_back(std::make_pair(dd, it->first));
        }
        std::sort(close_by.begin(), close_by.end());
        if (picks.empty() && close_by.size() >= 2) {
            c.picks.push_back(close_by[0].second);
            c.picks.push_back(close_by[1].second);
        } else if (picks.size() == 1) {
            size_t other = NONE;
            for (size_t k = 0; k < close_by.size(); k++)
                if (close_by[k].second != picks[0]) { other = close_by[k].second; break; }
            if (other == NONE) return;
            c.picks.push_back(picks[0]);
            c.picks.push_back(other);
        } else {
            if (!close_by.empty()) picks.push_back(close_by[0].second);
            invalidate();
            set_status(canvas_prompt());
            return;
        }
        break;
    }
    case T_PERP_LINE:
    case T_PAR_LINE:
        if (picks.empty()) {
            if (hv_line != NONE) picks.push_back(hv_line);
            invalidate();
            set_status(canvas_prompt());
            return;
        }
        c.picks = picks;
        c.s.push_back(s);
        break;
    case T_ANGLE_BISECTOR:
        if (picks.size() < 2) {
            if (hv_line != NONE && (picks.empty() || picks[0] != hv_line)) picks.push_back(hv_line);
            invalidate();
            set_status(canvas_prompt());
            return;
        }
        c.picks = picks;
        break;
    case T_TANGENT_LINE:
    case T_REFLECT:
        if (pending.empty()) {
            pending.push_back(s);
            invalidate();
            set_status(canvas_prompt());
            return;
        }
        if (t == T_TANGENT_LINE) {
            size_t cv = curve_at(m, round_or_conic_or_function);
            if (cv == NONE) return;
            c.picks.push_back(cv);
        } else {
            if (hv_line == NONE) return;
            c.picks.push_back(hv_line);
        }
        c.s = pending;
        break;
    case T_FOCI:
    case T_DIRECTRIX:
    case T_ASYMPTOTES: {
        size_t cv = curve_at(m, t == T_ASYMPTOTES ? is_hyperbola : geo::is_conic);
        if (cv == NONE) {
            if (hv_curve != NONE) app.flash_ents(std::vector<size_t>(1, hv_curve));
            invalidate();
            return;
        }
        c.picks.push_back(cv);
        break;
    }
    default:
        return;
    }
    std::vector<size_t> ents = c.picks;
    std::vector<size_t> se = snap_ents(c.s);
    ents.insert(ents.end(), se.begin(), se.end());
    finish(app.apply(c), ents);
}

void dimension_click(V2 m) {
    GxDoc *d = app.doc;
    Hover h = hit(m);
    if (h.any && h.s.kind == Sel::DIM) {
        app.select_only(h.s);
        sel_changed();
        dlg_dimension(g_frame, h.s.i, false);
        return;
    }
    if (h.any && h.s.kind == Sel::ENT && !geo::hidden(d, h.s.i)) {
        std::vector<size_t>::iterator it = std::find(picks.begin(), picks.end(), h.s.i);
        if (it != picks.end()) picks.erase(it);
        else {
            if (picks.size() >= 2) picks.clear();
            picks.push_back(h.s.i);
        }
        invalidate();
        set_status(canvas_prompt());
        return;
    }
    GxDim kind;
    size_t a, b;
    if (dim_kind_for(picks, view.to_w(m), &kind, &a, &b)) {
        reset_clicks();
        add_dimension(kind, a, b, false, view.to_w(m));
    } else {
        reset_clicks();
        invalidate();
    }
    set_status(canvas_prompt());
}

// ------------------------------------------------------------ painting

struct Painter {
    Graphics &g;
    View v;
    double k;      // device pixels per screen pixel (above 1 when printing)
    bool print;
    Font *font, *small_font;

    Painter(Graphics &g_, const View &v_, double k_, bool print_) : g(g_), v(v_), k(k_), print(print_) {
        LOGFONTW lf;
        GetObjectW(g_ui_font, sizeof lf, &lf);
        HDC dc = g.GetHDC();
        font = new Font(dc, &lf);
        lf.lfHeight = lf.lfHeight * 4 / 5;
        small_font = new Font(dc, &lf);
        g.ReleaseHDC(dc);
        if (print) {
            // fonts are made for the screen; scale them for the printer
            delete font;
            delete small_font;
            FontFamily fam(L"Tahoma");
            font = new Font(&fam, (REAL)(8.0 * k * 96.0 / 72.0), FontStyleRegular, UnitPixel);
            small_font = new Font(&fam, (REAL)(6.5 * k * 96.0 / 72.0), FontStyleRegular, UnitPixel);
        }
    }
    ~Painter() {
        delete font;
        delete small_font;
    }
};

Color ent_color(GxDoc *d, size_t e, const std::set<size_t> &conflicts, bool print) {
    if (!print && GetTickCount() < app.flash_until && std::find(app.flash.begin(), app.flash.end(), e) != app.flash.end())
        return col::conflict;
    if (!print && (app.selected(Sel(Sel::ENT, e)) || std::find(picks.begin(), picks.end(), e) != picks.end()))
        return col::selected;
    if (conflicts.count(e)) return col::conflict;
    if (geo::is_axis(d, e)) return col::axis;
    if (geo::is_function(d, e)) return col::functions[geo::function_index(d, e) % 6];
    return gx_doc_ent_fixed(d, e) == 1 ? col::fixed_ent : col::free_ent;
}

std::set<size_t> conflict_ents(GxDoc *d) {
    std::set<size_t> s;
    if (gx_doc_conflict_count(d) == 0) return s;
    for (size_t c = 0; c < gx_doc_con_count(d); c++)
        if (gx_doc_con_conflict(d, c) == 1) {
            std::vector<size_t> v = geo::con_ents(d, c);
            s.insert(v.begin(), v.end());
        }
    for (size_t i = 0; i < gx_doc_dim_count(d); i++)
        if (gx_doc_dim_conflict(d, i) == 1) {
            std::vector<size_t> v = geo::dim_ents(d, i);
            s.insert(v.begin(), v.end());
        }
    for (size_t e = 0; e < gx_doc_ent_count(d); e++)
        if (gx_doc_ent_conflict(d, e) == 1) s.insert(e);
    return s;
}

void paint_grid(Painter &p) {
    const View &v = p.v;
    double raw = 24.0 * p.k / v.s;
    double base = pow(10.0, floor(log10(raw)));
    double step = 10 * base;
    double m[3] = {1, 2, 5};
    for (int i = 0; i < 3; i++)
        if (m[i] * base >= raw) { step = m[i] * base; break; }
    double major = step * 5;
    GxWindow w = v.window();
    Pen minor(col::grid_minor, 1), maj(col::grid_major, 1);
    for (double x = floor(w.x0 / step) * step; x <= w.x1; x += step) {
        bool is_major = fabs(floor(x / major + 0.5) * major - x) < step * 0.01;
        REAL sx = (REAL)v.to_s(V2(x, 0)).x;
        p.g.DrawLine(is_major ? &maj : &minor, sx, (REAL)v.oy, sx, (REAL)(v.oy + v.h));
    }
    for (double y = floor(w.y0 / step) * step; y <= w.y1; y += step) {
        bool is_major = fabs(floor(y / major + 0.5) * major - y) < step * 0.01;
        REAL sy = (REAL)v.to_s(V2(0, y)).y;
        p.g.DrawLine(is_major ? &maj : &minor, (REAL)v.ox, sy, (REAL)(v.ox + v.w), sy);
    }
    // numbers along the axes
    SolidBrush tb(col::axis_text);
    V2 o = v.to_s(V2(0, 0));
    double ly = dclamp(o.y, v.oy + 2, v.oy + v.h - 14 * p.k);
    for (double x = floor(w.x0 / major) * major; x <= w.x1; x += major) {
        if (fabs(x) < major * 1e-6) continue;
        V2 q = v.to_s(V2(x, 0));
        p.g.DrawString(fmt_val(x).c_str(), -1, p.small_font, PointF((REAL)(q.x + 2), (REAL)(ly + 1)), &tb);
    }
    double lx = dclamp(o.x, v.ox + 2, v.ox + v.w - 40 * p.k);
    for (double y = floor(w.y0 / major) * major; y <= w.y1; y += major) {
        if (fabs(y) < major * 1e-6) continue;
        V2 q = v.to_s(V2(0, y));
        p.g.DrawString(fmt_val(y).c_str(), -1, p.small_font, PointF((REAL)(lx + 2), (REAL)(q.y - 12 * p.k)), &tb);
    }
}

// The label of dimension i (or of the dimension kind/a/b about to be added
// when i is NONE), drawn with its leaders; returns the label rectangle.
RECT paint_dim(Painter &p, size_t i, GxDim kind, size_t a, size_t b, V2 anchor, V2 lw, bool supp, const Color &c,
               const wstr &txt, const wstr &name) {
    GxDoc *d = app.doc;
    const View &v = p.v;
    double k = p.k;
    V2 lp = v.to_s(lw);
    Pen st(c, (REAL)(1.0 * k)), thin(with_alpha(c, 150), (REAL)(0.8 * k));
    struct L {
        static void linear(Painter &p, const Pen &st, const Pen &thin, const Color &c, V2 a, V2 b, V2 dir, V2 lw, V2 lp) {
            const View &v = p.v;
            double l = dmax(dir.len(), 1e-12);
            V2 u = dir * (1 / l), n(-u.y, u.x);
            double h = (lw.x - a.x) * n.x + (lw.y - a.y) * n.y;
            double hb = (lw.x - b.x) * n.x + (lw.y - b.y) * n.y;
            V2 pa = a + n * h, pb = b + n * hb;
            V2 sa = v.to_s(pa), sb = v.to_s(pb);
            V2 ends[2][2] = {{v.to_s(a), sa}, {v.to_s(b), sb}};
            for (int j = 0; j < 2; j++) {
                V2 dd = ends[j][1] - ends[j][0];
                double dl = dd.len();
                if (dl > 5 * p.k) {
                    V2 du = dd * (1 / dl);
                    p.g.DrawLine(&thin, pf(ends[j][0] + du * (3 * p.k)), pf(ends[j][1] + du * (4 * p.k)));
                }
            }
            p.g.DrawLine(&st, pf(sa), pf(sb));
            arrow(p.g, sa, sa - sb, c, p.k);
            arrow(p.g, sb, sb - sa, c, p.k);
            double t = (lw.x - pa.x) * u.x + (lw.y - pa.y) * u.y;
            double len = (pb.x - pa.x) * u.x + (pb.y - pa.y) * u.y;
            if (t < 0 || t > len) p.g.DrawLine(&thin, pf(t < 0 ? sa : sb), pf(lp));
        }
    };
    V2 origin = geo::pos(d, (size_t)gx_doc_origin(d));
    switch (kind) {
    case GX_DIM_DISTANCE: {
        V2 pa = geo::pos(d, a), pb = geo::pos(d, b);
        L::linear(p, st, thin, c, pa, pb, pb - pa, lw, lp);
        break;
    }
    case GX_DIM_LENGTH: {
        V2 pa, pb;
        geo::line_ends(d, a, &pa, &pb);
        L::linear(p, st, thin, c, pa, pb, pb - pa, lw, lp);
        break;
    }
    case GX_DIM_HDIST: L::linear(p, st, thin, c, geo::pos(d, a), geo::pos(d, b), V2(1, 0), lw, lp); break;
    case GX_DIM_VDIST: L::linear(p, st, thin, c, geo::pos(d, a), geo::pos(d, b), V2(0, 1), lw, lp); break;
    case GX_DIM_X: L::linear(p, st, thin, c, origin, geo::pos(d, a), V2(1, 0), lw, lp); break;
    case GX_DIM_Y: L::linear(p, st, thin, c, origin, geo::pos(d, a), V2(0, 1), lw, lp); break;
    case GX_DIM_POINT_LINE: {
        V2 pp = geo::pos(d, a), la, lb;
        geo::line_ends(d, b, &la, &lb);
        V2 f = foot(pp, la, lb);
        V2 sp = v.to_s(pp), sf = v.to_s(f);
        p.g.DrawLine(&st, pf(sp), pf(sf));
        arrow(p.g, sp, sp - sf, c, k);
        arrow(p.g, sf, sf - sp, c, k);
        V2 sa = v.to_s(la), sb = v.to_s(lb);
        Pen dash(with_alpha(c, 150), (REAL)(0.8 * k));
        dash.SetDashStyle(DashStyleDash);
        p.g.DrawLine(&dash, pf(dist(sa, sf) < dist(sb, sf) ? sa : sb), pf(sf));
        p.g.DrawLine(&thin, pf((sp + sf) * 0.5), pf(lp));
        break;
    }
    case GX_DIM_RADIUS:
    case GX_DIM_DIAMETER: {
        V2 cc;
        double r;
        if (!geo::circle(d, a, &cc, &r)) break;
        V2 dir = lw - cc;
        double l = dmax(dir.len(), 1e-12);
        V2 u = dir * (1 / l);
        V2 edge = v.to_s(cc + u * r);
        V2 start = kind == GX_DIM_DIAMETER ? v.to_s(cc - u * r) : v.to_s(cc);
        p.g.DrawLine(&st, pf(start), pf(edge));
        arrow(p.g, edge, edge - start, c, k);
        if (kind == GX_DIM_DIAMETER) arrow(p.g, start, start - edge, c, k);
        if (l > r) p.g.DrawLine(&thin, pf(edge), pf(lp));
        break;
    }
    case GX_DIM_ANGLE: {
        V2 x = anchor;
        V2 d1 = geo::line_dir(d, a), d2 = geo::line_dir(d, b);
        double a0 = atan2(d1.y, d1.x), a1 = atan2(d2.y, d2.x);
        double sweep = fmod(fmod(a1 - a0, TAU) + TAU, TAU);
        if (sweep > PI) sweep -= TAU;
        if (supp) {
            a0 += PI;
            sweep = fmod(fmod(a1 - a0, TAU) + TAU, TAU);
            if (sweep > PI) sweep -= TAU;
        }
        double rad = dmax(dist(lw, x), 4.0 / v.s);
        double la = atan2(lw.y - x.y, lw.x - x.x);
        double rel = fmod(fmod(la - a0, TAU) + TAU, TAU);
        bool inside = sweep >= 0 ? rel <= sweep : (TAU - rel) <= -sweep;
        double s0 = inside ? a0 : a0 + PI;
        Poly pts;
        for (int j = 0; j <= 48; j++) {
            double tt = s0 + sweep * j / 48.0;
            pts.push_back(v.to_s(V2(x.x + rad * cos(tt), x.y + rad * sin(tt))));
        }
        draw_poly(p.g, pts, st);
        arrow(p.g, pts[0], pts[0] - pts[2], c, k);
        arrow(p.g, pts[48], pts[48] - pts[46], c, k);
        break;
    }
    case GX_DIM_CONIC_A:
    case GX_DIM_CONIC_B: {
        GxConic ck;
        V2 cc;
        double av, bv, tv;
        if (!geo::conic(d, a, &ck, &cc, &av, &bv, &tv)) break;
        V2 tip = kind == GX_DIM_CONIC_A ? V2(cc.x + av * cos(tv), cc.y + av * sin(tv)) : V2(cc.x - bv * sin(tv), cc.y + bv * cos(tv));
        V2 sc = v.to_s(cc), stp = v.to_s(tip);
        p.g.DrawLine(&st, pf(sc), pf(stp));
        arrow(p.g, stp, stp - sc, c, k);
        p.g.DrawLine(&thin, pf((sc + stp) * 0.5), pf(lp));
        break;
    }
    }
    RECT r = {0, 0, 0, 0};
    if (txt.empty()) return r;
    // the label: the name in small gray letters, then the value
    RectF tb, nb;
    p.g.MeasureString(txt.c_str(), -1, p.font, PointF(0, 0), &tb);
    if (!name.empty()) p.g.MeasureString(name.c_str(), -1, p.small_font, PointF(0, 0), &nb);
    double nw = name.empty() ? 0 : nb.Width + 1 * k;
    double w = tb.Width + nw + 4 * k, h = tb.Height + 2 * k;
    RectF box((REAL)(lp.x - w / 2), (REAL)(lp.y - h / 2), (REAL)w, (REAL)h);
    SolidBrush bg(Color(235, 255, 255, 255));
    p.g.FillRectangle(&bg, box);
    Pen frame(with_alpha(c, 110), (REAL)(1.0 * k));
    p.g.DrawRectangle(&frame, box);
    double x0 = box.X + 2 * k;
    if (!name.empty()) {
        SolidBrush nbr(col::measured);
        p.g.DrawString(name.c_str(), -1, p.small_font, PointF((REAL)x0, (REAL)(box.Y + h - nb.Height - 1 * k)), &nbr);
        x0 += nw;
    }
    SolidBrush tbr(c);
    p.g.DrawString(txt.c_str(), -1, p.font, PointF((REAL)x0, (REAL)(box.Y + 1 * k)), &tbr);
    r.left = (LONG)floor(box.X);
    r.top = (LONG)floor(box.Y);
    r.right = (LONG)ceil(box.X + box.Width);
    r.bottom = (LONG)ceil(box.Y + box.Height);
    (void)i;
    return r;
}

void paint_dims(Painter &p) {
    GxDoc *d = app.doc;
    if (!p.print) dim_rects.clear();
    for (size_t i = 0; i < gx_doc_dim_count(d); i++) {
        int kind = gx_doc_dim_kind(d, i);
        if (kind < 0) continue;
        std::vector<size_t> ents = geo::dim_ents(d, i);
        if (ents.empty()) continue;
        bool driving = gx_doc_dim_driving(d, i) == 1;
        bool sel = !p.print && app.selected(Sel(Sel::DIM, i));
        bool hov = !p.print && hover.any && hover.s == Sel(Sel::DIM, i);
        char *err = gx_doc_dim_error(d, i);
        bool bad = gx_doc_dim_conflict(d, i) == 1 || err != 0;
        gx_string_free(err);
        Color c = sel ? col::selected : bad ? col::conflict : driving ? col::driving : col::measured;
        if (hov && !sel) c = col::hover;
        double v = gx_doc_dim_value(d, i);
        wstr txt = fmt_dim(v, kind == GX_DIM_ANGLE);
        if (kind == GX_DIM_DIAMETER) txt = L"\x2300" + txt;
        wstr expr = take(gx_doc_dim_expr(d, i));
        if (!driving) txt = L"(" + txt + L")";
        else if (!expr.empty()) txt = expr + L" = " + txt;
        V2 anchor, off;
        if (gx_doc_dim_anchor(d, i, &anchor.x, &anchor.y) != 1 || gx_doc_dim_offset(d, i, &off.x, &off.y) != 1) continue;
        RECT r = paint_dim(p, i, (GxDim)kind, ents[0], ents.size() > 1 ? ents[1] : 0, anchor, anchor + off,
                           gx_doc_dim_supp(d, i) == 1, c, txt, geo::dim_name(d, i));
        if (!p.print) dim_rects.push_back(std::make_pair(r, i));
    }
}

// Where the symbols of constraint c go: an anchor and an offset for each.
std::vector<std::pair<V2, V2> > glyph_spots(size_t c) {
    GxDoc *d = app.doc;
    std::vector<std::pair<V2, V2> > out;
    std::vector<size_t> e = geo::con_ents(d, c);
    int kind = gx_doc_con_kind(d, c);
    if (e.empty() || kind < 0) return out;
    struct S {
        static std::pair<V2, V2> top_of(size_t e) {
            GxDoc *d = app.doc;
            if (geo::is_line(d, e)) {
                V2 a, b;
                geo::line_ends(d, e, &a, &b);
                V2 m = view.to_s((a + b) * 0.5), dir = view.to_s(b) - view.to_s(a);
                double l = dir.len();
                V2 n = l > 0 ? V2(-dir.y / l, dir.x / l) : V2(0, -1);
                if (n.y > 0) n = n * -1;
                return std::make_pair(m, n * 14);
            }
            V2 cc;
            double r;
            if (geo::is_round(d, e) && geo::circle(d, e, &cc, &r))
                return std::make_pair(view.to_s(cc + V2(r * 0.7071, r * 0.7071)), V2(10, -10));
            std::map<size_t, Polys>::iterator it = shapes.find(e);
            if (it != shapes.end() && !it->second.empty() && !it->second[0].empty())
                return std::make_pair(it->second[0][it->second[0].size() / 3], V2(10, -12));
            return std::make_pair(view.to_s(geo::center(d, e)), V2(10, -10));
        }
    };
    V2 p0 = view.to_s(geo::pos(d, e[0]));
    V2 p1 = e.size() > 1 ? view.to_s(geo::pos(d, e[1])) : p0;
    switch (kind) {
    case GX_CON_COINCIDENT: out.push_back(std::make_pair(p0, V2(11, 11))); break;
    case GX_CON_ON_CURVE: case GX_CON_FOCUS: out.push_back(std::make_pair(p0, V2(11, -11))); break;
    case GX_CON_MIDPOINT: case GX_CON_MIDPOINT_OF: out.push_back(std::make_pair(p0, V2(-11, 11))); break;
    case GX_CON_HORIZONTAL:
        if (geo::is_line(d, e[0])) out.push_back(S::top_of(e[0]));
        else out.push_back(std::make_pair((p0 + p1) * 0.5, V2(0, -14)));
        break;
    case GX_CON_VERTICAL:
        if (geo::is_line(d, e[0])) out.push_back(S::top_of(e[0]));
        else out.push_back(std::make_pair((p0 + p1) * 0.5, V2(14, 0)));
        break;
    case GX_CON_PARALLEL: case GX_CON_PERPENDICULAR: case GX_CON_COLLINEAR: case GX_CON_EQUAL:
        out.push_back(S::top_of(e[0]));
        if (e.size() > 1) out.push_back(S::top_of(e[1]));
        break;
    case GX_CON_CONCENTRIC: out.push_back(std::make_pair(view.to_s(geo::center(d, e[0])), V2(12, 12))); break;
    case GX_CON_SYMMETRIC: out.push_back(std::make_pair((p0 + p1) * 0.5, V2(0, 14))); break;
    case GX_CON_TANGENT_AT: out.push_back(std::make_pair(p1, V2(12, -12))); break;
    case GX_CON_DIRECTRIX:
        if (e.size() == 4) out.push_back(std::make_pair(p0, V2(0, 0)));
        else out.push_back(S::top_of(e[0]));
        break;
    case GX_CON_TANGENT: {
        V2 at;
        bool solved;
        if (gx_doc_con_contact(d, c, &at.x, &at.y, &solved) == 1) out.push_back(std::make_pair(view.to_s(at), V2(12, -12)));
        break;
    }
    default: out.push_back(std::make_pair(p0, V2(0, 0))); break;
    }
    return out;
}

Icon con_icon(size_t c) {
    switch (gx_doc_con_kind(app.doc, c)) {
    case GX_CON_COINCIDENT: return I_COINCIDENT;
    case GX_CON_ON_CURVE: return I_ON_CURVE;
    case GX_CON_MIDPOINT: case GX_CON_MIDPOINT_OF: return I_MIDPOINT_CON;
    case GX_CON_HORIZONTAL: return I_HORIZONTAL;
    case GX_CON_VERTICAL: return I_VERTICAL;
    case GX_CON_PARALLEL: return I_PARALLEL;
    case GX_CON_PERPENDICULAR: return I_PERPENDICULAR;
    case GX_CON_COLLINEAR: return I_COLLINEAR;
    case GX_CON_EQUAL: return I_EQUAL;
    case GX_CON_TANGENT: case GX_CON_TANGENT_AT: return I_TANGENT;
    case GX_CON_CONCENTRIC: return I_CONCENTRIC;
    case GX_CON_SYMMETRIC: return I_SYMMETRIC;
    case GX_CON_OFFSET: return gx_doc_con_turned(app.doc, c) == 1 ? I_PERPENDICULAR : I_PARALLEL;
    case GX_CON_BISECTOR: return I_ANGLE_BISECTOR;
    case GX_CON_FOCUS: return I_FOCI;
    case GX_CON_DIRECTRIX: return I_DIRECTRIX;
    case GX_CON_ASYMPTOTE: return I_ASYMPTOTES;
    }
    return I_COINCIDENT;
}

// Constraint symbols: small framed images from the toolbar strip.
void paint_glyphs(Graphics &g) {
    GxDoc *d = app.doc;
    glyph_rects.clear();
    std::map<std::pair<int, int>, int> used;
    HDC dc = g.GetHDC();
    for (size_t c = 0; c < gx_doc_con_count(d); c++) {
        if (gx_doc_con_kind(d, c) < 0) continue;
        std::vector<size_t> ents = geo::con_ents(d, c);
        bool skip = false;
        for (size_t k = 0; k < ents.size(); k++)
            if (geo::hidden(d, ents[k])) skip = true;
        if (skip) continue;
        bool sel = app.selected(Sel(Sel::CON, c));
        bool hov = hover.any && (hover.s == Sel(Sel::CON, c) ||
                                 (hover.s.kind == Sel::ENT && std::find(ents.begin(), ents.end(), hover.s.i) != ents.end()));
        bool bad = gx_doc_con_conflict(d, c) == 1;
        std::vector<std::pair<V2, V2> > spots = glyph_spots(c);
        for (size_t k = 0; k < spots.size(); k++) {
            V2 q = spots[k].first + spots[k].second;
            if (!q.ok()) continue;
            std::pair<int, int> key((int)floor(q.x / 16), (int)floor(q.y / 16));
            int n = used[key]++;
            q.x += n * 19;
            RECT r = {iround(q.x) - 9, iround(q.y) - 9, iround(q.x) + 9, iround(q.y) + 9};
            if (r.right < 0 || r.bottom < 0 || r.left > view.w || r.top > view.h) continue;
            HBRUSH bg = CreateSolidBrush(sel ? RGB(200, 235, 200) : hov ? RGB(255, 236, 200) : RGB(255, 255, 225));
            FillRect(dc, &r, bg);
            DeleteObject(bg);
            HBRUSH fr = CreateSolidBrush(bad ? RGB(220, 0, 0) : sel ? RGB(0, 150, 0) : hov ? RGB(255, 140, 0) : RGB(160, 160, 140));
            FrameRect(dc, &r, fr);
            DeleteObject(fr);
            ImageList_Draw(g_icons, con_icon(c), dc, r.left + 1, r.top + 1, ILD_TRANSPARENT);
            glyph_rects.push_back(std::make_pair(r, c));
        }
    }
    g.ReleaseHDC(dc);
}

void paint_handles(Painter &p, const std::set<size_t> &conflicts) {
    GxDoc *d = app.doc;
    handles.clear();
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        size_t e = it->first;
        GxConic kind;
        V2 cc;
        double av, bv, tv;
        if (!geo::conic(d, e, &kind, &cc, &av, &bv, &tv)) continue;
        double ct = cos(tv), st = sin(tv);
        Handle h0 = {view.to_s(V2(cc.x + av * ct, cc.y + av * st)), e, 0};
        Handle h1 = {view.to_s(V2(cc.x - bv * st, cc.y + bv * ct)), e, 1};
        handles.push_back(h0);
        if (kind != GX_CONIC_PARABOLA) handles.push_back(h1);
        bool show = app.selected(Sel(Sel::ENT, e)) || (hover.any && hover.s == Sel(Sel::ENT, e)) ||
                    (drag == D_HANDLE && drag_e == e);
        if (!show) continue;
        Color c = with_alpha(ent_color(d, e, conflicts, false), 140);
        Pen pen(c, 1);
        V2 cs = view.to_s(cc);
        p.g.DrawLine(&pen, pf(cs), pf(h0.at));
        if (kind != GX_CONIC_PARABOLA) p.g.DrawLine(&pen, pf(cs), pf(h1.at));
        Pen dash(with_alpha(c, 100), 1);
        dash.SetDashStyle(DashStyleDash);
        double ext = dhypot(view.w, view.h) / view.s * 2;
        if (kind == GX_CONIC_HYPERBOLA) {
            for (int sg = -1; sg <= 1; sg += 2) {
                V2 dd(av * ct - sg * bv * st, av * st + sg * bv * ct);
                double l = dmax(dd.len(), 1e-12);
                V2 u = dd * (ext / l);
                p.g.DrawLine(&dash, pf(view.to_s(cc - u)), pf(view.to_s(cc + u)));
            }
        }
        if (kind == GX_CONIC_PARABOLA) {
            V2 q(cc.x - av * ct, cc.y - av * st);
            p.g.DrawLine(&dash, pf(view.to_s(V2(q.x - st * ext, q.y + ct * ext))), pf(view.to_s(V2(q.x + st * ext, q.y - ct * ext))));
        }
        for (int i = 0; i < (kind == GX_CONIC_PARABOLA ? 1 : 2); i++) {
            V2 h = i == 0 ? h0.at : h1.at;
            bool hv = hover_handle >= 0 && handles[hover_handle].e == e && handles[hover_handle].which == i;
            double r = hv ? 5.5 : 4.0;
            PointF pts[4] = {pf(h + V2(0, -r)), pf(h + V2(r, 0)), pf(h + V2(0, r)), pf(h + V2(-r, 0))};
            SolidBrush wb(Color(255, 255, 255, 255));
            p.g.FillPolygon(&wb, pts, 4);
            Pen hp(col::selected, 1.5f);
            p.g.DrawPolygon(&hp, pts, 4);
        }
    }
}

// The shape a click at m would complete, built on a copy of the document.
Polys ghost(V2 m) {
    Polys out;
    Tool t = app.tool;
    int need = tool_clicks(t);
    GxDoc *d = app.doc;
    std::vector<Snap> s = pending;
    Snap last = snap_at(m);
    if (t == T_PARABOLA_FOCUS && pending.size() == 1) {
        size_t l = curve_at(m, geo::is_line);
        if (l == NONE) return out;
        last = Snap::curve(l, view.to_w(m));
    }
    if (t == T_LINE && pending.size() == 1) last = aligned(pending[0], last, hv_of(pending[0], last));
    s.push_back(last);
    GxDoc *copy = 0;
    ptrdiff_t made = -1;
    if (need > 0) {
        if ((int)s.size() != need) return out;
        copy = gx_doc_copy(d);
        made = build_shape(copy, t, s);
    } else {
        ConstructChange c;
        c.t = t;
        c.m = view.to_w(m);
        if ((t == T_PERP_LINE || t == T_PAR_LINE) && picks.size() == 1) {
            c.picks = picks;
            c.s.push_back(last);
        } else if (t == T_ANGLE_BISECTOR && picks.size() == 2) {
            c.picks = picks;
        } else if ((t == T_MIDPOINT || t == T_PERP_BISECTOR) && pending.size() == 1) {
            c.s = s;
        } else {
            return out;
        }
        copy = gx_doc_copy(d);
        size_t n0 = gx_doc_ent_count(copy);
        if (c.run(copy))
            for (size_t e = n0; e < gx_doc_ent_count(copy); e++) {
                Polys p = samples(copy, view, e);
                out.insert(out.end(), p.begin(), p.end());
            }
        gx_doc_free(copy);
        return out;
    }
    if (made >= 0) out = samples(copy, view, (size_t)made);
    gx_doc_free(copy);
    return out;
}

void paint_preview(Painter &p) {
    if (!mouse_in || app.tool == T_SELECT || drag != D_NONE) return;
    GxDoc *d = app.doc;
    Graphics &g = p.g;
    Pen gp(col::ghost, 1.5f);
    if (app.tool == T_DIMENSION) {
        GxDim kind;
        size_t a, b;
        V2 anchor;
        if (dim_kind_for(picks, mouse_w, &kind, &a, &b) && gx_doc_dim_anchor_of(d, kind, a, b, &anchor.x, &anchor.y) == 1)
            paint_dim(p, NONE, kind, a, b, anchor, mouse_w, false, col::ghost, L"", L"");
        return;
    }
    Snap s = snap_at(mouse);
    if (s.kind == Snap::POINT) {
        V2 q = view.to_s(s.at);
        Pen sp(col::snap, 1.5f);
        g.DrawRectangle(&sp, (REAL)(q.x - 6), (REAL)(q.y - 6), 12.0f, 12.0f);
    } else if (s.kind == Snap::CURVE) {
        V2 q = view.to_s(s.at);
        Pen sp(col::snap, 1.5f);
        g.DrawLine(&sp, pf(q + V2(-4, -4)), pf(q + V2(4, 4)));
        g.DrawLine(&sp, pf(q + V2(-4, 4)), pf(q + V2(4, -4)));
    }
    Polys gh = ghost(mouse);
    for (size_t i = 0; i < gh.size(); i++) {
        if (gh[i].size() == 1) dot(g, gh[i][0], 3.5, col::ghost);
        else draw_poly(g, gh[i], gp);
    }
    // a rubber line from the last click while more clicks are needed
    if (!pending.empty() && gh.empty()) {
        Pen rp(col::ghost, 1);
        rp.SetDashStyle(DashStyleDash);
        g.DrawLine(&rp, pf(view.to_s(pending.back().at)), pf(mouse));
    }
    for (size_t i = 0; i < pending.size(); i++) dot(g, view.to_s(pending[i].at), 3.5, col::selected);
}

void paint_all(Graphics &g, const View &v, double k, bool print) {
    GxDoc *d = app.doc;
    Painter p(g, v, k, print);
    g.SetSmoothingMode(SmoothingModeAntiAlias);
    g.SetTextRenderingHint(TextRenderingHintSystemDefault);
    SolidBrush paper(col::paper);
    g.FillRectangle(&paper, (REAL)v.ox, (REAL)v.oy, (REAL)v.w, (REAL)v.h);
    if (app.show_grid && !print) paint_grid(p);
    std::set<size_t> conflicts = conflict_ents(d);
    std::vector<size_t> hov_ents;
    if (!print && hover.any && hover.s.kind == Sel::CON) hov_ents = geo::con_ents(d, hover.s.i);
    if (!print && hover.any && hover.s.kind == Sel::DIM) hov_ents = geo::dim_ents(d, hover.s.i);
    // the shapes of the view, or of the printed page
    std::map<size_t, Polys> printed;
    std::map<size_t, Polys> &sh = print ? printed : shapes;
    if (print)
        for (size_t e = 0; e < gx_doc_ent_count(d); e++)
            if (geo::live(d, e) && !geo::hidden(d, e)) printed[e] = samples(d, v, e);
    for (std::map<size_t, Polys>::iterator it = sh.begin(); it != sh.end(); ++it) {
        size_t e = it->first;
        if (geo::is_point(d, e)) continue;
        Color c = ent_color(d, e, conflicts, print);
        bool axis = geo::is_axis(d, e);
        if (axis && print) continue;
        bool hov = !print && ((hover.any && hover.s == Sel(Sel::ENT, e)) ||
                              std::find(hov_ents.begin(), hov_ents.end(), e) != hov_ents.end());
        bool sel = !print && app.selected(Sel(Sel::ENT, e));
        double w = (axis ? 1.0 : sel ? 2.2 : 1.5) * k;
        if (hov && !axis) {
            Pen glow(with_alpha(col::hover, 90), (REAL)(w + 5 * k));
            glow.SetLineJoin(LineJoinRound);
            for (size_t i = 0; i < it->second.size(); i++) draw_poly(g, it->second[i], glow);
        }
        Pen pen(c, (REAL)w);
        pen.SetLineJoin(LineJoinRound);
        if (gx_doc_ent_construction(d, e) == 1 && !axis) pen.SetDashStyle(DashStyleDash);
        for (size_t i = 0; i < it->second.size(); i++) draw_poly(g, it->second[i], pen);
    }
    if (!print) paint_handles(p, conflicts);
    // contact points of tangencies that the solver finds
    for (size_t c = 0; c < gx_doc_con_count(d); c++) {
        V2 at;
        bool solved = false;
        if (gx_doc_con_contact(d, c, &at.x, &at.y, &solved) == 1 && solved) ring(g, v.to_s(at), 2.5 * k, col::measured, k);
    }
    paint_dims(p);
    if (!print) {
        if (app.show_symbols) paint_glyphs(g);
        else glyph_rects.clear();
    }
    // points on top
    size_t origin = (size_t)gx_doc_origin(d);
    for (std::map<size_t, Polys>::iterator it = sh.begin(); it != sh.end(); ++it) {
        size_t e = it->first;
        if (!geo::is_point(d, e) || it->second.empty() || it->second[0].empty()) continue;
        V2 q = it->second[0][0];
        Color c = ent_color(d, e, conflicts, print);
        bool hov = !print && ((hover.any && hover.s == Sel(Sel::ENT, e)) ||
                              std::find(hov_ents.begin(), hov_ents.end(), e) != hov_ents.end());
        bool sel = !print && app.selected(Sel(Sel::ENT, e));
        if (e == origin) {
            if (print) continue;
            ring(g, q, 4.5, sel ? col::selected : hov ? col::hover : col::axis, 1.3);
            dot(g, q, 1.6, sel ? col::selected : col::axis);
            continue;
        }
        double r = (hov || sel ? 4.2 : 3.4) * k;
        if (hov) dot(g, q, r + 4 * k, with_alpha(col::hover, 80));
        SolidBrush white(col::paper), fill(c);
        if (gx_doc_point_locked(d, e) == 1) {
            g.FillRectangle(&white, (REAL)(q.x - r - 1.5 * k), (REAL)(q.y - r - 1.5 * k), (REAL)(2 * r + 3 * k), (REAL)(2 * r + 3 * k));
            g.FillRectangle(&fill, (REAL)(q.x - r), (REAL)(q.y - r), (REAL)(2 * r), (REAL)(2 * r));
        } else {
            dot(g, q, r + 1.5 * k, col::paper);
            dot(g, q, r, c);
        }
        if (hov || sel) {
            SolidBrush tb(c);
            g.DrawString(geo::ent_name(d, e).c_str(), -1, p.font, PointF((REAL)(q.x + 6), (REAL)(q.y - 18)), &tb);
        }
    }
    if (!print) {
        // selection handles at the ends of selected shapes
        for (size_t k2 = 0; k2 < app.sel.size(); k2++) {
            if (app.sel[k2].kind != Sel::ENT || !geo::is_line(d, app.sel[k2].i)) continue;
            V2 a, b;
            geo::line_ends(d, app.sel[k2].i, &a, &b);
            V2 ends[2] = {v.to_s(a), v.to_s(b)};
            for (int j = 0; j < 2; j++) {
                if (!ends[j].ok()) continue;
                SolidBrush wb(Color(255, 255, 255, 255));
                Pen hp(col::selected, 1);
                g.FillRectangle(&wb, (REAL)(ends[j].x - 3), (REAL)(ends[j].y - 3), 6.0f, 6.0f);
                g.DrawRectangle(&hp, (REAL)(ends[j].x - 3), (REAL)(ends[j].y - 3), 6.0f, 6.0f);
            }
        }
        paint_preview(p);
    }
}

// ------------------------------------------------------------ view

// Lower-left and upper-right corners of what the document shows.
bool bounds(V2 *lo, V2 *hi) {
    GxDoc *d = app.doc;
    *lo = V2(1e300, 1e300);
    *hi = V2(-1e300, -1e300);
    bool any = false;
    // the origin counts when a shape, constraint or dimension uses it
    size_t origin = (size_t)gx_doc_origin(d);
    bool origin_used = false;
    for (size_t e = 0; e < gx_doc_ent_count(d) && !origin_used; e++) {
        size_t p[3] = {NONE, NONE, NONE};
        double r;
        if (geo::is_line(d, e) && !geo::is_axis(d, e)) geo::line_pts(d, e, &p[0], &p[1]);
        else if (geo::kind(d, e) == GX_ENT_ARC) gx_doc_arc_pts(d, e, &p[0], &p[1], &p[2]);
        else if (geo::kind(d, e) == GX_ENT_CIRCLE) gx_doc_circle_vals(d, e, &p[0], &r);
        origin_used = p[0] == origin || p[1] == origin || p[2] == origin;
    }
    for (size_t c = 0; c < gx_doc_con_count(d) && !origin_used; c++) {
        std::vector<size_t> v = geo::con_ents(d, c);
        origin_used = std::find(v.begin(), v.end(), origin) != v.end();
    }
    for (size_t i = 0; i < gx_doc_dim_count(d) && !origin_used; i++) {
        std::vector<size_t> v = geo::dim_ents(d, i);
        origin_used = std::find(v.begin(), v.end(), origin) != v.end();
    }
    std::vector<V2> extra;
    if (origin_used) extra.push_back(V2(0, 0));
    // the labels of the dimensions, and the ends of the lines of linear ones,
    // which run beside the shapes at the distance of the label
    for (size_t i = 0; i < gx_doc_dim_count(d); i++) {
        V2 a, off;
        if (gx_doc_dim_anchor(d, i, &a.x, &a.y) != 1 || gx_doc_dim_offset(d, i, &off.x, &off.y) != 1) continue;
        extra.push_back(a + off);
        std::vector<size_t> e = geo::dim_ents(d, i);
        V2 p, q;
        switch (gx_doc_dim_kind(d, i)) {
        case GX_DIM_DISTANCE: case GX_DIM_HDIST: case GX_DIM_VDIST:
            if (e.size() < 2) continue;
            p = geo::pos(d, e[0]);
            q = geo::pos(d, e[1]);
            break;
        case GX_DIM_LENGTH: geo::line_ends(d, e[0], &p, &q); break;
        case GX_DIM_X: case GX_DIM_Y: p = V2(0, 0); q = geo::pos(d, e[0]); break;
        default: continue;
        }
        V2 c = a + off - (p + q) * 0.5;
        extra.push_back(p + c);
        extra.push_back(q + c);
    }
    for (size_t k = 0; k < extra.size(); k++) {
        if (!extra[k].ok()) continue;
        lo->x = dmin(lo->x, extra[k].x);
        lo->y = dmin(lo->y, extra[k].y);
        hi->x = dmax(hi->x, extra[k].x);
        hi->y = dmax(hi->y, extra[k].y);
        any = true;
    }
    for (size_t e = 0; e < gx_doc_ent_count(d); e++) {
        if (!geo::live(d, e) || geo::hidden(d, e) || geo::is_fixed(d, e)) continue;
        std::vector<V2> pts;
        V2 c;
        double r, a, b, t;
        GxConic k;
        switch (geo::kind(d, e)) {
        case GX_ENT_POINT: pts.push_back(geo::pos(d, e)); break;
        case GX_ENT_CIRCLE:
        case GX_ENT_ARC:
            if (geo::circle(d, e, &c, &r)) { pts.push_back(c - V2(r, r)); pts.push_back(c + V2(r, r)); }
            break;
        case GX_ENT_CONIC:
            if (geo::conic(d, e, &k, &c, &a, &b, &t)) {
                double m = dmax(fabs(a), fabs(b));
                pts.push_back(c - V2(m, m));
                pts.push_back(c + V2(m, m));
            }
            break;
        }
        for (size_t i = 0; i < pts.size(); i++) {
            if (!pts[i].ok()) continue;
            lo->x = dmin(lo->x, pts[i].x);
            lo->y = dmin(lo->y, pts[i].y);
            hi->x = dmax(hi->x, pts[i].x);
            hi->y = dmax(hi->y, pts[i].y);
            any = true;
        }
    }
    return any;
}

void update_scrollbars() {
    V2 lo, hi;
    GxWindow w = view.window();
    if (!bounds(&lo, &hi)) {
        lo = V2(-5, -5);
        hi = V2(5, 5);
    }
    // the drawing and what the window shows, with half a window around it
    double mx = (w.x1 - w.x0) / 2, my = (w.y1 - w.y0) / 2;
    double x0 = dmin(lo.x - mx, w.x0), x1 = dmax(hi.x + mx, w.x1);
    double y0 = dmin(lo.y - my, w.y0), y1 = dmax(hi.y + my, w.y1);
    SCROLLINFO si;
    si.cbSize = sizeof si;
    si.fMask = SIF_ALL | SIF_DISABLENOSCROLL;
    si.nMin = (int)floor(x0 * view.s);
    si.nMax = (int)ceil(x1 * view.s);
    si.nPage = (UINT)view.w;
    si.nPos = (int)floor(w.x0 * view.s + 0.5);
    SetScrollInfo(hwnd, SB_HORZ, &si, TRUE);
    si.nMin = (int)floor(-y1 * view.s);
    si.nMax = (int)ceil(-y0 * view.s);
    si.nPage = (UINT)view.h;
    si.nPos = (int)floor(-w.y1 * view.s + 0.5);
    SetScrollInfo(hwnd, SB_VERT, &si, TRUE);
}

void view_changed() {
    update_scrollbars();
    invalidate();
    SendMessage(g_frame, WM_COMMAND, MAKEWPARAM(IDC_CANVAS, 1), 0);  // tells the frame to show the zoom
}

void zoom_at(double factor, V2 m) {
    V2 before = view.to_w(m);
    view.s = dclamp(view.s * factor, base_s * 0.05, base_s * 50);
    V2 after = view.to_w(m);
    view.cx += before.x - after.x;
    view.cy += before.y - after.y;
    view_changed();
}

void on_scroll(int bar, int code) {
    SCROLLINFO si;
    si.cbSize = sizeof si;
    si.fMask = SIF_ALL;
    GetScrollInfo(hwnd, bar, &si);
    int pos = si.nPos, line = 30;
    switch (code) {
    case SB_LINEUP: pos -= line; break;
    case SB_LINEDOWN: pos += line; break;
    case SB_PAGEUP: pos -= (int)si.nPage; break;
    case SB_PAGEDOWN: pos += (int)si.nPage; break;
    case SB_THUMBTRACK: case SB_THUMBPOSITION: pos = si.nTrackPos; break;
    case SB_TOP: pos = si.nMin; break;
    case SB_BOTTOM: pos = si.nMax; break;
    default: return;
    }
    if (bar == SB_HORZ) view.cx = pos / view.s + view.w / 2 / view.s;
    else view.cy = -pos / view.s - view.h / 2 / view.s;
    if (code == SB_THUMBTRACK) {
        si.fMask = SIF_POS;
        si.nPos = pos;
        SetScrollInfo(hwnd, bar, &si, TRUE);
        invalidate();
    } else {
        view_changed();
    }
}

// ------------------------------------------------------------ input

void set_hover(V2 m) {
    Hover h;
    int hh = -1;
    if (drag == D_NONE) {
        if (app.tool == T_SELECT) hh = hit_handle(m);
        if (hh < 0) h = hit(m);
    }
    bool changed = h.any != hover.any || (h.any && h.s != hover.s) || hh != hover_handle;
    hover = h;
    hover_handle = hh;
    if (changed || app.tool != T_SELECT) invalidate();
}

void begin_drag(V2 p) {
    GxDoc *d = app.doc;
    V2 pw = view.to_w(p);
    drag = D_NONE;
    int hh = hit_handle(p);
    if (hh >= 0) {
        drag = D_HANDLE;
        drag_e = handles[hh].e;
        drag_which = handles[hh].which;
    } else if (press_hit && press_sel.kind == Sel::DIM) {
        V2 a, off;
        if (gx_doc_dim_anchor(d, press_sel.i, &a.x, &a.y) == 1 && gx_doc_dim_offset(d, press_sel.i, &off.x, &off.y) == 1) {
            drag = D_DIM_LABEL;
            drag_e = press_sel.i;
            grab = pw - (a + off);
        }
    } else if (press_hit && press_sel.kind == Sel::ENT) {
        size_t e = press_sel.i;
        drag_e = e;
        drag_pts.clear();
        drag_from.clear();
        drag_start_w = pw;
        size_t pts[3];
        size_t n = 0;
        switch (geo::kind(d, e)) {
        case GX_ENT_POINT:
            if (gx_doc_point_locked(d, e) == 0) {
                drag = D_POINT;
                grab = pw - geo::pos(d, e);
            }
            break;
        case GX_ENT_LINE: geo::line_pts(d, e, &pts[0], &pts[1]); n = 2; drag = D_TRANSLATE; break;
        case GX_ENT_CIRCLE: drag = D_CIRCLE_EDGE; break;
        case GX_ENT_ARC:
            if (gx_doc_arc_pts(d, e, &pts[0], &pts[1], &pts[2]) == 1) { n = 3; drag = D_TRANSLATE; }
            break;
        case GX_ENT_CONIC: {
            GxConic k;
            double a, b, t;
            if (gx_doc_conic_vals(d, e, &k, &pts[0], &a, &b, &t) == 1) { n = 1; drag = D_TRANSLATE; }
            break;
        }
        case GX_ENT_CURVE: drag = D_CURVE; break;
        }
        for (size_t i = 0; i < n; i++)
            if (gx_doc_point_locked(d, pts[i]) == 0) {
                drag_pts.push_back(pts[i]);
                drag_from.push_back(geo::pos(d, pts[i]));
            }
        if (drag == D_TRANSLATE && drag_pts.empty()) drag = D_NONE;
        if (geo::is_fixed(d, e)) drag = D_NONE;
    } else if (!press_hit) {
        drag = D_BOX;
    }
    if (drag != D_NONE && drag != D_BOX) gx_doc_checkpoint(d);
    moved_doc = false;
}

void drag_to(V2 m) {
    GxDoc *d = app.doc;
    V2 w = view.to_w(m);
    switch (drag) {
    case D_POINT: gx_doc_drag_point(d, drag_e, w.x - grab.x, w.y - grab.y); break;
    case D_TRANSLATE: {
        std::vector<double> xy;
        for (size_t i = 0; i < drag_pts.size(); i++) {
            xy.push_back(drag_from[i].x + w.x - drag_start_w.x);
            xy.push_back(drag_from[i].y + w.y - drag_start_w.y);
        }
        gx_doc_drag_points(d, &drag_pts[0], &xy[0], drag_pts.size());
        break;
    }
    case D_CIRCLE_EDGE: gx_doc_drag_circle(d, drag_e, w.x, w.y); break;
    case D_HANDLE: gx_doc_drag_conic_handle(d, drag_e, (uint8_t)drag_which, w.x, w.y); break;
    case D_CURVE: gx_doc_drag_curve(d, drag_e, w.x, w.y); break;
    case D_DIM_LABEL: {
        V2 a;
        if (gx_doc_dim_anchor(d, drag_e, &a.x, &a.y) == 1) gx_doc_drag_dim_offset(d, drag_e, w.x - grab.x - a.x, w.y - grab.y - a.y);
        break;
    }
    default: return;
    }
    moved_doc = true;
    invalidate();
}

void end_box(V2 a, V2 b, bool add) {
    GxDoc *d = app.doc;
    RECT r = {(LONG)dmin(a.x, b.x), (LONG)dmin(a.y, b.y), (LONG)dmax(a.x, b.x) + 1, (LONG)dmax(a.y, b.y) + 1};
    if (!add) app.sel.clear();
    for (std::map<size_t, Polys>::iterator it = shapes.begin(); it != shapes.end(); ++it) {
        if (geo::is_fixed(d, it->first) || it->second.empty()) continue;
        bool all = true;
        for (size_t i = 0; i < it->second.size() && all; i++)
            for (size_t j = 0; j < it->second[i].size(); j++)
                if (!rect_has(r, it->second[i][j])) { all = false; break; }
        Sel s(Sel::ENT, it->first);
        if (all && !app.selected(s)) app.sel.push_back(s);
    }
    for (size_t k = 0; k < dim_rects.size(); k++) {
        RECT dr = dim_rects[k].first;
        Sel s(Sel::DIM, dim_rects[k].second);
        if (dr.left >= r.left && dr.right <= r.right && dr.top >= r.top && dr.bottom <= r.bottom && !app.selected(s))
            app.sel.push_back(s);
    }
    sel_changed();
}

void toggle_sel(Sel s) {
    for (size_t k = 0; k < app.sel.size(); k++)
        if (app.sel[k] == s) {
            app.sel.erase(app.sel.begin() + k);
            return;
        }
    app.sel.push_back(s);
}

void context_menu(POINT screen, V2 m) {
    Hover h = hit(m);
    if (h.any && !app.selected(h.s)) {
        app.select_only(h.s);
        sel_changed();
    }
    HMENU menu = LoadMenuW(g_inst, MAKEINTRESOURCEW(app.sel.empty() ? IDR_CONTEXT_PAPER : IDR_CONTEXT));
    HMENU popup = GetSubMenu(menu, 0);
    if (!app.sel.empty()) SetMenuDefaultItem(popup, ID_EDIT_PROPERTIES, FALSE);
    TrackPopupMenu(popup, TPM_RIGHTBUTTON, screen.x, screen.y, 0, g_frame, 0);
    DestroyMenu(menu);
}

HCURSOR cursor_for(V2 m) {
    if (drag == D_PAN) return LoadCursor(0, IDC_SIZEALL);
    if (app.tool != T_SELECT) return LoadCursor(0, IDC_CROSS);
    if (drag != D_NONE && drag != D_BOX) return LoadCursor(0, IDC_SIZEALL);
    if (hit_handle(m) >= 0) return LoadCursor(0, IDC_SIZEALL);
    Hover h = hit(m);
    if (h.any && (h.s.kind == Sel::DIM || (h.s.kind == Sel::ENT && !geo::is_fixed(app.doc, h.s.i)))) return LoadCursor(0, IDC_SIZEALL);
    return LoadCursor(0, IDC_ARROW);
}

LRESULT CALLBACK canvas_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CREATE:
        hwnd = h;
        SetTimer(h, 1, 100, 0);
        return 0;
    case WM_SIZE: {
        double ow = view.w, oh = view.h;
        view.w = LOWORD(lp);
        view.h = HIWORD(lp);
        (void)ow;
        (void)oh;
        update_scrollbars();
        invalidate();
        return 0;
    }
    case WM_ERASEBKGND:
        return 1;
    case WM_PAINT: {
        PAINTSTRUCT ps;
        HDC dc = BeginPaint(h, &ps);
        RECT rc;
        GetClientRect(h, &rc);
        if (rc.right > 0 && rc.bottom > 0) {
            build_shapes();
            HDC mem = CreateCompatibleDC(dc);
            HBITMAP bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
            HGDIOBJ old = SelectObject(mem, bmp);
            {
                Graphics g(mem);
                paint_all(g, view, 1.0, false);
            }
            if (drag == D_BOX && dragging) {
                RECT r = {press_at.x, press_at.y, (LONG)mouse.x, (LONG)mouse.y};
                if (r.left > r.right) std::swap(r.left, r.right);
                if (r.top > r.bottom) std::swap(r.top, r.bottom);
                DrawFocusRect(mem, &r);
            }
            BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
        }
        EndPaint(h, &ps);
        return 0;
    }
    case WM_TIMER:
        // the red of rejected entities fades after a moment
        if (app.flash_until && GetTickCount() >= app.flash_until) {
            app.flash_until = 0;
            app.flash.clear();
            invalidate();
        }
        return 0;
    case WM_HSCROLL: on_scroll(SB_HORZ, LOWORD(wp)); return 0;
    case WM_VSCROLL: on_scroll(SB_VERT, LOWORD(wp)); return 0;
    case WM_MOUSEWHEEL: {
        int delta = GET_WHEEL_DELTA_WPARAM(wp);
        POINT pt = {GET_X_LPARAM(lp), GET_Y_LPARAM(lp)};
        ScreenToClient(h, &pt);
        if (GET_KEYSTATE_WPARAM(wp) & MK_CONTROL) {
            zoom_at(pow(1.2, delta / 120.0), V2(pt.x, pt.y));
        } else if (GET_KEYSTATE_WPARAM(wp) & MK_SHIFT) {
            view.cx -= delta / 120.0 * 40 / view.s;
            view_changed();
        } else {
            view.cy += delta / 120.0 * 40 / view.s;
            view_changed();
        }
        return 0;
    }
    case WM_SETCURSOR:
        if (LOWORD(lp) == HTCLIENT) {
            POINT pt;
            GetCursorPos(&pt);
            ScreenToClient(h, &pt);
            SetCursor(cursor_for(V2(pt.x, pt.y)));
            return TRUE;
        }
        break;
    case WM_MOUSEMOVE: {
        V2 m(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        if (!mouse_in) {
            TRACKMOUSEEVENT t = {sizeof t, TME_LEAVE, h, 0};
            TrackMouseEvent(&t);
            mouse_in = true;
        }
        mouse = m;
        mouse_w = view.to_w(m);
        if (drag == D_PAN) {
            view.cx -= (m.x - pan_last.x) / view.s;
            view.cy += (m.y - pan_last.y) / view.s;
            pan_last = m;
            view_changed();
        } else if (pressed && app.tool == T_SELECT) {
            if (!dragging && (abs((int)m.x - press_at.x) > 3 || abs((int)m.y - press_at.y) > 3)) {
                dragging = true;
                begin_drag(V2(press_at.x, press_at.y));
                // dragging a shape that is not selected selects it
                if (press_hit && !app.selected(press_sel) && drag != D_BOX && drag != D_HANDLE) {
                    app.select_only(press_sel);
                    sel_changed();
                }
            }
            if (dragging) {
                if (drag == D_BOX) invalidate();
                else drag_to(m);
            }
        } else {
            set_hover(m);
        }
        SendMessage(g_frame, WM_COMMAND, MAKEWPARAM(IDC_CANVAS, 2), 0);  // coordinates in the status bar
        return 0;
    }
    case WM_MOUSELEAVE:
        mouse_in = false;
        hover = Hover();
        hover_handle = -1;
        invalidate();
        return 0;
    case WM_LBUTTONDOWN: {
        SetFocus(h);
        SetCapture(h);
        V2 m(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        mouse = m;
        mouse_w = view.to_w(m);
        build_shapes();
        press_at.x = (LONG)m.x;
        press_at.y = (LONG)m.y;
        press_keys = wp;
        pressed = true;
        dragging = false;
        press_added = false;
        if (app.tool == T_SELECT) {
            Hover hh = hit(m);
            press_hit = hh.any;
            press_sel = hh.s;
            if (hit_handle(m) >= 0) press_hit = true, press_sel = Sel(Sel::ENT, handles[hit_handle(m)].e);
        } else if (app.tool == T_DIMENSION) {
            dimension_click(m);
        } else {
            tool_click(m, true);
        }
        return 0;
    }
    case WM_LBUTTONUP: {
        V2 m(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        // releasing the capture sends WM_CAPTURECHANGED, which must not see the press
        bool was_pressed = pressed;
        pressed = false;
        if (GetCapture() == h) ReleaseCapture();
        if (!was_pressed) return 0;
        if (app.tool == T_SELECT) {
            bool add = (press_keys & (MK_SHIFT | MK_CONTROL)) != 0;
            if (dragging) {
                if (drag == D_BOX) end_box(V2(press_at.x, press_at.y), m, add);
                else if (moved_doc) {
                    gx_doc_solve(app.doc, 0);
                    doc_changed();
                }
            } else if (press_hit) {
                if (add) toggle_sel(press_sel);
                else app.select_only(press_sel);
                sel_changed();
            } else if (!add && !app.sel.empty()) {
                app.sel.clear();
                sel_changed();
            }
            drag = D_NONE;
            dragging = false;
            set_hover(m);
            invalidate();
        } else if (tool_clicks(app.tool) >= 2 && press_added && pending.size() == 1 &&
                   (abs((int)m.x - press_at.x) > 5 || abs((int)m.y - press_at.y) > 5)) {
            // drawing by dragging: the release is the second click
            tool_click(m, false);
        }
        return 0;
    }
    case WM_LBUTTONDBLCLK: {
        V2 m(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        if (app.tool != T_SELECT) {
            // a double click is two clicks for the drawing tools
            SendMessage(h, WM_LBUTTONDOWN, wp, lp);
            return 0;
        }
        Hover hh = hit(m);
        if (hh.any) {
            app.select_only(hh.s);
            sel_changed();
            dlg_properties(g_frame);
        }
        return 0;
    }
    case WM_MBUTTONDOWN:
    case WM_RBUTTONDOWN:
        SetFocus(h);
        SetCapture(h);
        drag = D_PAN;
        dragging = false;
        pan_last = V2(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        press_at.x = GET_X_LPARAM(lp);
        press_at.y = GET_Y_LPARAM(lp);
        return 0;
    case WM_MBUTTONUP:
    case WM_RBUTTONUP: {
        if (GetCapture() == h) ReleaseCapture();
        V2 m(GET_X_LPARAM(lp), GET_Y_LPARAM(lp));
        bool still = abs((int)m.x - press_at.x) <= 3 && abs((int)m.y - press_at.y) <= 3;
        drag = D_NONE;
        if (msg == WM_RBUTTONUP && still) {
            if (!pending.empty() || !picks.empty()) {
                reset_clicks();
                invalidate();
                set_status(canvas_prompt());
            } else if (app.tool != T_SELECT) {
                set_tool(T_SELECT);
            } else {
                POINT pt = {(LONG)m.x, (LONG)m.y};
                ClientToScreen(h, &pt);
                context_menu(pt, m);
            }
        }
        return 0;
    }
    case WM_CONTEXTMENU:
        if (GET_X_LPARAM(lp) == -1) {  // the menu key
            POINT pt = {(LONG)(view.w / 2), (LONG)(view.h / 2)};
            V2 m(pt.x, pt.y);
            ClientToScreen(h, &pt);
            context_menu(pt, m);
        }
        return 0;
    case WM_KEYDOWN:
        switch (wp) {
        case VK_ESCAPE:
            if (!pending.empty() || !picks.empty()) reset_clicks();
            else if (app.tool != T_SELECT) set_tool(T_SELECT);
            else if (!app.sel.empty()) { app.sel.clear(); sel_changed(); }
            invalidate();
            set_status(canvas_prompt());
            return 0;
        case VK_RETURN:
            if (!app.sel.empty()) dlg_properties(g_frame);
            return 0;
        case VK_LEFT: view.cx -= 40 / view.s; view_changed(); return 0;
        case VK_RIGHT: view.cx += 40 / view.s; view_changed(); return 0;
        case VK_UP: view.cy += 40 / view.s; view_changed(); return 0;
        case VK_DOWN: view.cy -= 40 / view.s; view_changed(); return 0;
        }
        break;
    case WM_CAPTURECHANGED:
        if ((HWND)lp != h && pressed) {
            pressed = false;
            dragging = false;
            drag = D_NONE;
            invalidate();
        }
        return 0;
    case WM_DESTROY:
        KillTimer(h, 1);
        return 0;
    }
    return DefWindowProc(h, msg, wp, lp);
}

} // namespace

// ------------------------------------------------------------ interface

HWND canvas_create(HWND parent) {
    WNDCLASSEXW wc = {sizeof wc};
    wc.style = CS_DBLCLKS;
    wc.lpfnWndProc = canvas_proc;
    wc.hInstance = g_inst;
    wc.hCursor = LoadCursor(0, IDC_ARROW);
    wc.lpszClassName = L"GxpadCanvas";
    RegisterClassExW(&wc);
    HDC dc = GetDC(0);
    base_s = GetDeviceCaps(dc, LOGPIXELSX) / UNITS_PER_INCH;
    ReleaseDC(0, dc);
    view.cx = 4;
    view.cy = 2;
    view.s = base_s;
    view.ox = view.oy = 0;
    view.w = 400;
    view.h = 300;
    return CreateWindowExW(WS_EX_CLIENTEDGE, L"GxpadCanvas", L"", WS_CHILD | WS_VISIBLE | WS_HSCROLL | WS_VSCROLL | WS_TABSTOP,
                           0, 0, 400, 300, parent, (HMENU)IDC_CANVAS, g_inst, 0);
}

void canvas_fit() {
    V2 lo, hi;
    if (!bounds(&lo, &hi)) {
        view.cx = 4;
        view.cy = 2;
        view.s = base_s;
    } else {
        double w = dmax(hi.x - lo.x, 1.0), h = dmax(hi.y - lo.y, 1.0);
        view.cx = (lo.x + hi.x) / 2;
        view.cy = (lo.y + hi.y) / 2;
        // a margin for the labels and leaders around the shapes
        view.s = dclamp(dmin(view.w * 0.8 / w, view.h * 0.8 / h), base_s * 0.05, base_s * 50);
    }
    view_changed();
}

void canvas_zoom(double factor) { zoom_at(factor, V2(view.w / 2, view.h / 2)); }

void canvas_set_zoom(double percent) {
    view.s = dclamp(base_s * percent / 100.0, base_s * 0.05, base_s * 50);
    view_changed();
}

double canvas_zoom_percent() { return view.s / base_s * 100.0; }

V2 canvas_mouse() { return mouse_in ? mouse_w : nan2(); }

void canvas_cancel() {
    reset_clicks();
    drag = D_NONE;
    pressed = dragging = false;
    if (hwnd) invalidate();
}

void canvas_refresh() {
    if (hwnd) {
        update_scrollbars();
        invalidate();
    }
}

wstr canvas_prompt() {
    size_t n = pending.size(), np = picks.size();
    switch (app.tool) {
    case T_SELECT: return load_str(IDS_READY);
    case T_POINT: return L"Click to place a point. Click a line or curve to place the point on it.";
    case T_LINE: return n == 0 ? L"Click where the line starts, or drag to draw it." : L"Click where the line ends.";
    case T_CIRCLE: return n == 0 ? L"Click the center of the circle, or drag from the center." : L"Click a point on the circle.";
    case T_CIRCLE3: case T_ARC3:
        return n == 0 ? L"Click the first of three points." : n == 1 ? L"Click the second point." : L"Click the third point.";
    case T_ARC: return n == 0 ? L"Click the center of the arc." : n == 1 ? L"Click where the arc starts." : L"Click where the arc ends (counterclockwise).";
    case T_ELLIPSE: case T_HYPERBOLA:
        return n == 0 ? L"Click the center." : n == 1 ? L"Click the end of the first axis." : L"Click to set the width of the second axis.";
    case T_PARABOLA: return n == 0 ? L"Click the vertex of the parabola." : L"Click the direction it opens toward.";
    case T_ELLIPSE_FOCI: case T_HYPERBOLA_FOCI:
        return n == 0 ? L"Click the first focus." : n == 1 ? L"Click the second focus." : L"Click a point on the curve.";
    case T_PARABOLA_FOCUS: return n == 0 ? L"Click the focus." : L"Click the directrix line.";
    case T_DIMENSION:
        if (np == 0) return L"Click a point, line, circle or conic to measure. Click a second one to measure between them.";
        return L"Click where to put the dimension, or click another object.";
    case T_MIDPOINT: return n == 0 ? L"Click a segment, or click the first of two points." : L"Click the second point.";
    case T_INTERSECT: return np == 0 ? L"Click where two curves cross, or click the first curve." : L"Click the second curve close_by the crossing.";
    case T_PERP_LINE: case T_PAR_LINE: return np == 0 ? L"Click a line." : L"Click the point the new line goes through.";
    case T_PERP_BISECTOR: return n == 0 ? L"Click a segment, or click the first of two points." : L"Click the second point.";
    case T_ANGLE_BISECTOR: return np == 0 ? L"Click the first line." : np == 1 ? L"Click the second line." : L"Click inside the angle to bisect.";
    case T_TANGENT_LINE: return n == 0 ? L"Click the point the tangent goes through." : L"Click the curve close_by where the tangent touches it.";
    case T_REFLECT: return n == 0 ? L"Click the point to reflect." : L"Click the mirror line.";
    case T_FOCI: return L"Click an ellipse, hyperbola or parabola.";
    case T_DIRECTRIX: return L"Click a conic on the side of the focus.";
    case T_ASYMPTOTES: return L"Click a hyperbola.";
    default: return load_str(IDS_READY);
    }
}

void canvas_print(HDC dc, RECT r) {
    View v;
    V2 lo, hi;
    if (!bounds(&lo, &hi)) {
        lo = V2(-5, -5);
        hi = V2(5, 5);
    }
    double k = GetDeviceCaps(dc, LOGPIXELSX) / 96.0;
    // room for the labels around the shapes
    double margin = 40 * k;
    v.ox = r.left;
    v.oy = r.top;
    v.w = r.right - r.left;
    v.h = r.bottom - r.top;
    double w = dmax(hi.x - lo.x, 1.0), h = dmax(hi.y - lo.y, 1.0);
    v.s = dmin((v.w - 2 * margin) * 0.85 / w, (v.h - 2 * margin) * 0.85 / h);
    v.cx = (lo.x + hi.x) / 2;
    v.cy = (lo.y + hi.y) / 2;
    Graphics g(dc);
    g.SetPageUnit(UnitPixel);
    g.SetClip(RectF((REAL)r.left, (REAL)r.top, (REAL)(r.right - r.left), (REAL)(r.bottom - r.top)));
    paint_all(g, v, k, true);
}

void canvas_copy_picture() {
    RECT rc;
    GetClientRect(hwnd, &rc);
    HDC dc = GetDC(hwnd);
    HDC mem = CreateCompatibleDC(dc);
    HBITMAP bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
    HGDIOBJ old = SelectObject(mem, bmp);
    {
        Graphics g(mem);
        View v = view;
        paint_all(g, v, 1.0, true);
    }
    SelectObject(mem, old);
    DeleteDC(mem);
    ReleaseDC(hwnd, dc);
    if (OpenClipboard(g_frame)) {
        EmptyClipboard();
        SetClipboardData(CF_BITMAP, bmp);
        CloseClipboard();
    } else {
        DeleteObject(bmp);
    }
}

void add_dimension(GxDim kind, size_t a, size_t b, bool place_auto, V2 label) {
    GxDoc *d = app.doc;
    V2 anchor;
    if (gx_doc_dim_anchor_of(d, kind, a, b, &anchor.x, &anchor.y) != 1) {
        set_status(L"Could not add the dimension: " + last_error());
        return;
    }
    if (place_auto) {
        double off = 40 / view.s;
        switch (kind) {
        case GX_DIM_HDIST: label = V2(anchor.x, anchor.y + off * 1.2); break;
        case GX_DIM_VDIST: label = V2(anchor.x + off * 1.2, anchor.y); break;
        case GX_DIM_X: {
            V2 q = geo::pos(d, a);
            label = V2(anchor.x, q.y >= 0 ? -off * 0.6 : off * 0.6);
            break;
        }
        case GX_DIM_Y: {
            V2 q = geo::pos(d, a);
            label = V2(q.x >= 0 ? -off * 0.9 : off * 0.9, anchor.y);
            break;
        }
        case GX_DIM_RADIUS:
        case GX_DIM_DIAMETER: {
            V2 cc;
            double r;
            if (geo::circle(d, a, &cc, &r)) label = cc + V2((r + off) * 0.7071, (r + off) * 0.7071);
            break;
        }
        case GX_DIM_ANGLE: label = anchor + V2(off * 1.5, off * 0.6); break;
        default: label = anchor + V2(off * 0.5, off); break;
        }
    }
    ptrdiff_t i = gx_doc_add_dim(d, kind, a, b, label.x - anchor.x, label.y - anchor.y);
    if (i < 0) {
        set_status(L"Could not add the dimension: " + last_error());
        std::vector<size_t> ents(1, a);
        if (b) ents.push_back(b);
        app.flash_ents(ents);
        invalidate();
        return;
    }
    app.select_only(Sel(Sel::DIM, (size_t)i));
    doc_changed();
    if (gx_doc_dim_driving(d, (size_t)i) == 1) dlg_dimension(g_frame, (size_t)i, true);
}
