// The Constrain and Dimension commands, which work on the selection, and
// deleting and selecting everything.
#include "common.h"

namespace {

struct Parts {
    std::vector<size_t> ents, pts, lines, rounds, conics, curves;
    std::vector<size_t> dims, vars;
};

Parts parts() {
    GxDoc *d = app.doc;
    Parts p;
    for (size_t k = 0; k < app.sel.size(); k++) {
        Sel s = app.sel[k];
        if (s.kind == Sel::DIM) p.dims.push_back(s.i);
        if (s.kind == Sel::VAR) p.vars.push_back(s.i);
        if (s.kind != Sel::ENT) continue;
        size_t e = s.i;
        p.ents.push_back(e);
        if (geo::is_point(d, e)) p.pts.push_back(e);
        if (geo::is_line(d, e)) p.lines.push_back(e);
        if (geo::is_round(d, e)) p.rounds.push_back(e);
        if (geo::is_conic(d, e)) p.conics.push_back(e);
        if (geo::is_curve(d, e)) p.curves.push_back(e);
    }
    return p;
}

bool only_ents() {
    for (size_t k = 0; k < app.sel.size(); k++)
        if (app.sel[k].kind != Sel::ENT) return false;
    return !app.sel.empty();
}

// The constraints a constraint command adds for the selection; empty when
// it does not apply.
struct Con { GxCon kind; size_t a, b; };

std::vector<Con> cons_for(UINT cmd) {
    GxDoc *d = app.doc;
    std::vector<Con> out;
    if (!only_ents()) return out;
    Parts p = parts();
    size_t n = p.ents.size();
    GxCon kind = (GxCon)(cmd - ID_CON_FIRST);
    Con c = {kind, 0, 0};
    switch (cmd) {
    case ID_CON_COINCIDENT:
        if (n == 2 && p.pts.size() == 2) { c.a = p.pts[0]; c.b = p.pts[1]; out.push_back(c); }
        break;
    case ID_CON_ON_CURVE:
        if (n == 2 && p.pts.size() == 1 && p.curves.size() == 1) { c.a = p.pts[0]; c.b = p.curves[0]; out.push_back(c); }
        break;
    case ID_CON_MIDPOINT:
        if (n == 2 && p.pts.size() == 1 && p.lines.size() == 1) { c.a = p.pts[0]; c.b = p.lines[0]; out.push_back(c); }
        break;
    case ID_CON_HORIZONTAL:
    case ID_CON_VERTICAL:
        if (n >= 2 && p.pts.size() == n)
            for (size_t k = 1; k < n; k++) { c.a = p.pts[k - 1]; c.b = p.pts[k]; out.push_back(c); }
        else if (n >= 1 && p.lines.size() == n)
            for (size_t k = 0; k < n; k++) { c.a = p.lines[k]; c.b = 0; out.push_back(c); }
        break;
    case ID_CON_PARALLEL:
        if (n >= 2 && p.lines.size() == n)
            for (size_t k = 1; k < n; k++) { c.a = p.lines[k - 1]; c.b = p.lines[k]; out.push_back(c); }
        break;
    case ID_CON_PERPENDICULAR:
    case ID_CON_COLLINEAR:
        if (n == 2 && p.lines.size() == 2) { c.a = p.lines[0]; c.b = p.lines[1]; out.push_back(c); }
        break;
    case ID_CON_EQUAL:
        if (n >= 2 && (p.lines.size() == n || p.rounds.size() == n))
            for (size_t k = 1; k < n; k++) { c.a = p.ents[k - 1]; c.b = p.ents[k]; out.push_back(c); }
        else if (n == 2 && p.lines.size() + p.rounds.size() == 2) { c.a = p.ents[0]; c.b = p.ents[1]; out.push_back(c); }
        break;
    case ID_CON_CONCENTRIC:
        if (n == 2 && p.rounds.size() + p.conics.size() == 2) { c.a = p.ents[0]; c.b = p.ents[1]; out.push_back(c); }
        else if (n == 2 && p.pts.size() == 1 && p.rounds.size() + p.conics.size() == 1) {
            c.a = p.pts[0];
            c.b = p.rounds.empty() ? p.conics[0] : p.rounds[0];
            out.push_back(c);
        }
        break;
    case ID_CON_TANGENT:
        if (n == 2 && p.curves.size() == 2) { c.a = p.curves[0]; c.b = p.curves[1]; out.push_back(c); }
        break;
    case ID_CON_SYMMETRIC:
        if (n == 3 && p.pts.size() == 2 && p.lines.size() == 1) { c.a = p.pts[0]; c.b = p.pts[1]; out.push_back(c); }
        break;
    }
    for (size_t k = 0; k < out.size(); k++)
        if (cmd != ID_CON_SYMMETRIC && gx_doc_can_constrain(d, out[k].kind, out[k].a, out[k].b) != 1) return std::vector<Con>();
    return out;
}

// The dimension a dimension command adds for the selection.
bool dim_for(UINT cmd, GxDim *kind, size_t *a, size_t *b) {
    GxDoc *d = app.doc;
    if (!only_ents()) return false;
    Parts p = parts();
    size_t n = p.ents.size();
    *b = 0;
    switch (cmd) {
    case ID_DIM_DISTANCE:
        if (n == 2 && p.pts.size() == 2) { *kind = GX_DIM_DISTANCE; *a = p.pts[0]; *b = p.pts[1]; return true; }
        if (n == 2 && p.pts.size() == 1 && p.lines.size() == 1) { *kind = GX_DIM_POINT_LINE; *a = p.pts[0]; *b = p.lines[0]; return true; }
        if (n == 2 && p.lines.size() == 2 && geo::lines_parallel(d, p.lines[0], p.lines[1])) {
            size_t pa, pb;
            geo::line_pts(d, p.lines[1], &pa, &pb);
            *kind = GX_DIM_POINT_LINE;
            *a = pa;
            *b = p.lines[0];
            return true;
        }
        return false;
    case ID_DIM_HDIST:
    case ID_DIM_VDIST:
        if (n == 2 && p.pts.size() == 2) {
            *kind = cmd == ID_DIM_HDIST ? GX_DIM_HDIST : GX_DIM_VDIST;
            *a = p.pts[0];
            *b = p.pts[1];
            return true;
        }
        return false;
    case ID_DIM_LENGTH:
        if (n == 1 && p.lines.size() == 1) { *kind = GX_DIM_LENGTH; *a = p.lines[0]; return true; }
        return false;
    case ID_DIM_RADIUS:
    case ID_DIM_DIAMETER:
        if (n == 1 && p.rounds.size() == 1) { *kind = cmd == ID_DIM_RADIUS ? GX_DIM_RADIUS : GX_DIM_DIAMETER; *a = p.rounds[0]; return true; }
        return false;
    case ID_DIM_ANGLE:
        if (n == 2 && p.lines.size() == 2 && !geo::lines_parallel(d, p.lines[0], p.lines[1])) {
            *kind = GX_DIM_ANGLE;
            *a = p.lines[0];
            *b = p.lines[1];
            return true;
        }
        return false;
    case ID_DIM_X:
    case ID_DIM_Y:
        if (n == 1 && p.pts.size() == 1) { *kind = cmd == ID_DIM_X ? GX_DIM_X : GX_DIM_Y; *a = p.pts[0]; return true; }
        return false;
    case ID_DIM_AXIS_A:
    case ID_DIM_AXIS_B: {
        if (n != 1 || p.conics.size() != 1) return false;
        GxConic k;
        V2 c;
        double av, bv, t;
        if (!geo::conic(d, p.conics[0], &k, &c, &av, &bv, &t)) return false;
        if (cmd == ID_DIM_AXIS_B && k == GX_CONIC_PARABOLA) return false;
        *kind = cmd == ID_DIM_AXIS_A ? GX_DIM_CONIC_A : GX_DIM_CONIC_B;
        *a = p.conics[0];
        return true;
    }
    }
    return false;
}

std::vector<size_t> lockable(bool *all_locked) {
    GxDoc *d = app.doc;
    std::vector<size_t> v;
    *all_locked = true;
    if (!only_ents()) return v;
    Parts p = parts();
    if (p.pts.size() != p.ents.size()) return v;
    for (size_t k = 0; k < p.pts.size(); k++) {
        if (geo::is_fixed(d, p.pts[k])) return std::vector<size_t>();
        if (gx_doc_point_locked(d, p.pts[k]) != 1) *all_locked = false;
        v.push_back(p.pts[k]);
    }
    return v;
}

std::vector<size_t> drawable(bool *all_construction) {
    GxDoc *d = app.doc;
    std::vector<size_t> v;
    *all_construction = true;
    Parts p = parts();
    for (size_t k = 0; k < p.curves.size(); k++) {
        size_t e = p.curves[k];
        if (geo::is_function(d, e) || geo::is_fixed(d, e)) continue;
        if (gx_doc_ent_construction(d, e) != 1) *all_construction = false;
        v.push_back(e);
    }
    return v;
}

bool one_dim(size_t *i) {
    if (app.sel.size() != 1 || app.sel[0].kind != Sel::DIM) return false;
    *i = app.sel[0].i;
    return true;
}

void rejected(const wstr &what, const std::vector<size_t> &ents) {
    wstr e = last_error();
    set_status(what + (e.empty() ? L"." : L": " + e));
    app.flash_ents(ents);
    canvas_refresh();
}

} // namespace

bool action_enabled(UINT cmd, bool *checked) {
    GxDoc *d = app.doc;
    *checked = false;
    if (cmd >= ID_CON_FIRST && cmd <= ID_CON_LAST) return !cons_for(cmd).empty();
    if (cmd >= ID_DIM_FIRST && cmd <= ID_DIM_LAST) {
        GxDim k;
        size_t a, b;
        return dim_for(cmd, &k, &a, &b);
    }
    size_t i;
    switch (cmd) {
    case ID_CON_LOCK: {
        bool all;
        bool ok = !lockable(&all).empty();
        *checked = ok && all;
        return ok;
    }
    case ID_CON_CONSTRUCTION: {
        bool all;
        bool ok = !drawable(&all).empty();
        *checked = ok && all;
        return ok;
    }
    case ID_CON_INFINITE: {
        Parts p = parts();
        if (!only_ents() || p.ents.size() != 1 || p.lines.size() != 1 || geo::is_fixed(d, p.lines[0])) return false;
        *checked = gx_doc_ent_infinite(d, p.lines[0]) == 1;
        return true;
    }
    case ID_DIM_DRIVING:
        if (!one_dim(&i)) return false;
        *checked = gx_doc_dim_driving(d, i) == 1;
        return true;
    case ID_DIM_NUMERIC:
        if (one_dim(&i)) {
            *checked = gx_doc_dim_numeric(d, i) == 1;
            return true;
        }
        if (app.sel.size() == 1 && app.sel[0].kind == Sel::VAR) {
            *checked = gx_doc_var_numeric(d, app.sel[0].i) == 1;
            return true;
        }
        return false;
    case ID_DIM_SUPP:
        if (!one_dim(&i) || gx_doc_dim_kind(d, i) != GX_DIM_ANGLE) return false;
        *checked = gx_doc_dim_supp(d, i) == 1;
        return true;
    }
    return false;
}

void action_run(UINT cmd) {
    GxDoc *d = app.doc;
    bool checked;
    if (!action_enabled(cmd, &checked)) return;
    if (cmd >= ID_CON_FIRST && cmd <= ID_CON_LAST) {
        std::vector<Con> cs = cons_for(cmd);
        std::vector<size_t> ents = app.sel_ents();
        int r;
        if (cmd == ID_CON_SYMMETRIC) {
            Parts p = parts();
            r = gx_doc_constrain_symmetric(d, p.pts[0], p.pts[1], p.lines[0]);
        } else if (cs.size() == 1) {
            V2 m = canvas_mouse();
            if (!m.ok()) m = geo::center(d, cs[0].a);
            if (!m.ok()) m = V2(0, 0);
            r = gx_doc_constrain_near(d, cs[0].kind, cs[0].a, cs[0].b, m.x, m.y);
        } else {
            std::vector<GxCon> kinds;
            std::vector<size_t> as, bs;
            for (size_t k = 0; k < cs.size(); k++) {
                kinds.push_back(cs[k].kind);
                as.push_back(cs[k].a);
                bs.push_back(cs[k].b);
            }
            r = gx_doc_add_constraints(d, &kinds[0], &as[0], &bs[0], cs.size());
        }
        if (r == 1) doc_changed();
        else rejected(L"The constraint was not added", ents);
        return;
    }
    if (cmd >= ID_DIM_FIRST && cmd <= ID_DIM_LAST) {
        GxDim k;
        size_t a, b;
        if (dim_for(cmd, &k, &a, &b)) add_dimension(k, a, b, true, V2());
        return;
    }
    size_t i;
    int r = 1;
    switch (cmd) {
    case ID_CON_LOCK: {
        bool all;
        std::vector<size_t> pts = lockable(&all);
        r = gx_doc_lock_points(d, &pts[0], pts.size(), !all);
        break;
    }
    case ID_CON_CONSTRUCTION: {
        bool all;
        std::vector<size_t> es = drawable(&all);
        r = gx_doc_set_construction_all(d, &es[0], es.size(), !all);
        break;
    }
    case ID_CON_INFINITE: {
        std::vector<size_t> ls = app.sel_ents();
        r = gx_doc_toggle_infinite(d, &ls[0], ls.size());
        break;
    }
    case ID_DIM_DRIVING:
        if (one_dim(&i)) r = gx_doc_set_dim_driving(d, i, !checked);
        break;
    case ID_DIM_NUMERIC:
        if (one_dim(&i)) r = gx_doc_set_dim_numeric(d, i, !checked);
        else r = gx_doc_set_var_numeric(d, app.sel[0].i, !checked);
        break;
    case ID_DIM_SUPP:
        if (one_dim(&i)) r = gx_doc_toggle_supp(d, i);
        break;
    }
    if (r == 1) {
        doc_changed();
    } else {
        std::vector<size_t> ents = app.sel_ents();
        if (one_dim(&i)) ents = geo::dim_ents(d, i);
        rejected(L"The change was not made", ents);
    }
}

void delete_selection() {
    GxDoc *d = app.doc;
    std::vector<size_t> ents, cons, dims, vars;
    for (size_t k = 0; k < app.sel.size(); k++) {
        Sel s = app.sel[k];
        switch (s.kind) {
        case Sel::ENT: if (!geo::is_fixed(d, s.i)) ents.push_back(s.i); break;
        case Sel::CON: cons.push_back(s.i); break;
        case Sel::DIM: dims.push_back(s.i); break;
        case Sel::VAR: vars.push_back(s.i); break;
        }
    }
    if (ents.empty() && cons.empty() && dims.empty() && vars.empty()) return;
    size_t z = 0;
    int r = gx_doc_delete(d, ents.empty() ? &z : &ents[0], ents.size(), cons.empty() ? &z : &cons[0], cons.size(),
                          dims.empty() ? &z : &dims[0], dims.size(), vars.empty() ? &z : &vars[0], vars.size());
    if (r == 1) {
        app.sel.clear();
        doc_changed();
        sel_changed();
    } else {
        rejected(L"Could not delete the selection", ents);
    }
}

void select_all() {
    GxDoc *d = app.doc;
    app.sel.clear();
    for (size_t e = 0; e < gx_doc_ent_count(d); e++)
        if (geo::live(d, e) && !geo::hidden(d, e) && !geo::is_fixed(d, e)) app.sel.push_back(Sel(Sel::ENT, e));
    sel_changed();
}
