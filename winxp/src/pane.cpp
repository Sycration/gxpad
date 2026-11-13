// The pane at the right of the drawing: the lists of dimensions, variables and
// functions, and the formula of the selected dimension or variable.
#include "common.h"

using namespace Gdiplus;

bool sets_itself(Sel t);

namespace {

const uint64_t LONGER_MS = 10000;   // more search time per "Search Longer"
const unsigned AUTO_LEVELS = 3;     // longer searches started without asking
const size_t SETTLED_SIZE = 60;     // an exact formula this small is not searched further

enum Tab { TAB_DIMS, TAB_VARS, TAB_FUNCS };

HWND pane, tabs, list, btn_add, btn_props, btn_delete;
HWND box, text, view, btn_copy, btn_latex, btn_longer, progress, state;
HIMAGELIST swatches;
bool filling;

// ------------------------------------------------------------ the formula

struct Formula {
    bool valid;
    Sel target;
    uint64_t gen;
    unsigned level;
    wstr name, plain, err;
    std::string latex;
    std::vector<std::string> rows;  // `name = formula` with u_k, then u_k = ...
    bool exact, is_angle;
    double value, actual;
    size_t size;
    Formula() : valid(false), gen(0), level(0), exact(false), is_angle(false), value(0), actual(0), size(0) {}
    bool settled() const { return !err.empty() || (exact && size <= SETTLED_SIZE); }
    // lower is better: a formula before an error, exact before numeric, then smaller
    bool better_or_equal(const Formula &o) const {
        bool e1 = !err.empty(), e2 = !o.err.empty();
        if (e1 != e2) return !e1;
        if (exact != o.exact) return exact;
        return size <= o.size;
    }
};

struct Job {
    GxDoc *doc;
    GxCancel *cancel;
    Sel target;
    uint64_t gen;
    unsigned level;
    GxFormula *f;
    std::string err;
    HANDLE thread;
};

Formula shown;
Job *job;                       // the derivation running for the selection
unsigned level_for = 0;         // the search level for (level_target, level_gen)
Sel level_target;
uint64_t level_gen = (uint64_t)-1;
std::vector<Bitmap *> images;   // the rows of the shown formula, typeset
double image_pt;                // their size

unsigned __stdcall job_thread(void *arg) {
    Job *j = (Job *)arg;
    j->f = gx_derive(j->doc, j->target.kind == Sel::DIM ? GX_TARGET_DIM : GX_TARGET_VAR, j->target.i, j->level * LONGER_MS,
                     j->cancel);
    if (!j->f) {
        const char *e = gx_last_error();
        j->err = e ? e : "";
    }
    gx_doc_free(j->doc);
    j->doc = 0;
    PostMessage(pane, WM_APP_FORMULA, 0, (LPARAM)j);
    return 0;
}

void free_images() {
    for (size_t k = 0; k < images.size(); k++) delete images[k];
    images.clear();
}

// A bitmap of the PNG bytes, independent of them.
Bitmap *decode_png(const unsigned char *p, size_t n) {
    HGLOBAL mem = GlobalAlloc(GMEM_MOVEABLE, n);
    if (!mem) return 0;
    memcpy(GlobalLock(mem), p, n);
    GlobalUnlock(mem);
    IStream *st = 0;
    if (CreateStreamOnHGlobal(mem, TRUE, &st) != S_OK) {
        GlobalFree(mem);
        return 0;
    }
    Bitmap *src = Bitmap::FromStream(st);
    Bitmap *out = 0;
    if (src && src->GetLastStatus() == Ok) {
        out = new Bitmap(src->GetWidth(), src->GetHeight(), PixelFormat32bppPARGB);
        Graphics g(out);
        g.DrawImage(src, 0, 0, src->GetWidth(), src->GetHeight());
    }
    delete src;
    st->Release();
    return out;
}

std::vector<unsigned char> typeset(const std::string &latex, double pt, double dpi) {
    std::vector<unsigned char> out;
    size_t n = 0;
    uint8_t *p = gx_latex_png(latex.c_str(), pt, dpi, 0x000000, &n);
    if (p) {
        out.assign(p, p + n);
        gx_bytes_free(p, n);
    }
    return out;
}

std::vector<std::string> display_rows() {
    std::vector<std::string> rows = shown.rows;
    if (!rows.empty() && !shown.exact) {
        size_t at = rows[0].find(" = ");
        if (at != std::string::npos) rows[0].replace(at, 3, " \\approx ");
    }
    return rows;
}

int screen_dpi() {
    HDC dc = GetDC(0);
    int d = GetDeviceCaps(dc, LOGPIXELSX);
    ReleaseDC(0, dc);
    return d;
}

// Typesets the rows at the largest size up to 13 pt that fits the view.
void make_images() {
    free_images();
    if (!shown.valid || shown.rows.empty()) return;
    RECT rc;
    GetClientRect(view, &rc);
    double avail = dmax(rc.right - 16.0, 60.0);
    std::vector<std::string> rows = display_rows();
    int dpi = screen_dpi();
    double pt = 13;
    for (int tries = 0; tries < 2; tries++) {
        free_images();
        double widest = 0;
        for (size_t k = 0; k < rows.size(); k++) {
            std::vector<unsigned char> png = typeset(rows[k], k == 0 ? pt : pt * 0.85, dpi);
            Bitmap *b = png.empty() ? 0 : decode_png(&png[0], png.size());
            images.push_back(b);
            if (b) widest = dmax(widest, b->GetWidth());
        }
        if (widest <= avail || pt <= 8.5) break;
        pt = dmax(8.5, pt * avail / widest);
    }
    image_pt = pt;
}

void update_formula_scroll() {
    RECT rc;
    GetClientRect(view, &rc);
    int w = 0, h = 8;
    for (size_t k = 0; k < images.size(); k++)
        if (images[k]) {
            w = (int)dmax(w, images[k]->GetWidth() + 16.0);
            h += images[k]->GetHeight() + 8;
        }
    SCROLLINFO si;
    si.cbSize = sizeof si;
    si.fMask = SIF_RANGE | SIF_PAGE;
    si.nMin = 0;
    si.nMax = w;
    si.nPage = rc.right + 1;
    SetScrollInfo(view, SB_HORZ, &si, TRUE);
    si.nMax = h;
    si.nPage = rc.bottom + 1;
    SetScrollInfo(view, SB_VERT, &si, TRUE);
}

void show_state() {
    bool busy = job != 0;
    SendMessage(progress, PBM_SETMARQUEE, busy, 40);
    ShowWindow(progress, busy ? SW_SHOW : SW_HIDE);
    wstr head, st;
    Sel t;
    bool has_target = app.formula_target(&t);
    if (!has_target) {
        head = L"Select a dimension or variable to see its formula.";
    } else if (sets_itself(t)) {
        GxDoc *d = app.doc;
        if (t.kind == Sel::DIM) {
            head = geo::dim_name(d, t.i) + L" = " + fmt_dim(gx_doc_dim_value(d, t.i), gx_doc_dim_kind(d, t.i) == GX_DIM_ANGLE);
            st = L"A driving dimension sets its value; it has no formula.";
        } else {
            head = geo::var_name(d, t.i) + L" = " + fmt_val(gx_doc_var_value(d, t.i));
            st = L"A free variable has the value you give it.";
        }
    } else if (shown.valid) {
        wstr val = fmt_val(shown.actual);
        if (shown.is_angle) val += L" rad (" + fmt_dim(shown.actual, true) + L")";
        head = shown.name + L" = " + val;
        if (!shown.err.empty()) st = shown.err;
        else if (busy) st = shown.exact ? L"Looking for a simpler formula..." : L"Looking for an exact formula...";
        else if (shown.exact) st = L"Exact formula.";
        else st = L"Approximate: no exact formula was found.";
        if (shown.err.empty() && finite(shown.value) && !(fabs(shown.value - shown.actual) <= 1e-6 * (1 + fabs(shown.actual))))
            st += L" The formula gives " + fmt_val(shown.value) + L".";
    } else {
        head = (t.kind == Sel::DIM ? geo::dim_name(app.doc, t.i) : geo::var_name(app.doc, t.i));
        st = busy ? L"Finding the formula..." : L"";
    }
    SetWindowTextW(text, head.c_str());
    SetWindowTextW(state, st.c_str());
    bool f = shown.valid && shown.err.empty() && !shown.rows.empty();
    EnableWindow(btn_copy, f);
    EnableWindow(btn_latex, f);
    EnableWindow(btn_longer, pane_can_search_longer());
}

void cancel_job() {
    if (job) {
        gx_cancel_set(job->cancel);
        job = 0;  // its result arrives later and is dropped
    }
}

unsigned level_of(Sel t) {
    if (t == level_target && level_gen == gx_doc_generation(app.doc)) return level_for;
    return 0;
}

void start(Sel t, unsigned level) {
    cancel_job();
    Job *j = new Job;
    j->doc = gx_doc_copy(app.doc);
    j->cancel = gx_cancel_new();
    j->target = t;
    j->gen = gx_doc_generation(app.doc);
    j->level = level;
    j->f = 0;
    unsigned id;
    j->thread = (HANDLE)_beginthreadex(0, 0, job_thread, j, 0, &id);
    if (!j->thread) {
        gx_doc_free(j->doc);
        gx_cancel_free(j->cancel);
        delete j;
        return;
    }
    job = j;
}

void take_result(Job *j) {
    Formula f;
    f.valid = true;
    f.target = j->target;
    f.gen = j->gen;
    f.level = j->level;
    if (j->f) {
        GxFormula *g = j->f;
        f.name = widen(g->name);
        f.plain = widen(g->plain);
        f.latex = g->latex;
        f.rows.push_back(g->latex_short);
        for (size_t k = 0; k < g->n_defs; k++) f.rows.push_back(std::string(g->defs[k].name) + " = " + g->defs[k].latex);
        f.exact = g->exact;
        f.is_angle = g->is_angle;
        f.value = g->value;
        f.actual = g->actual;
        f.size = g->size;
    } else {
        f.err = widen(j->err.c_str());
        if (f.err.empty()) f.err = L"No formula was found.";
        f.name = j->target.kind == Sel::DIM ? geo::dim_name(app.doc, j->target.i) : geo::var_name(app.doc, j->target.i);
    }
    bool same = shown.valid && shown.target == f.target && shown.gen == f.gen;
    bool changed = !same || shown.plain != f.plain || shown.exact != f.exact;
    // a tie goes to the longer search
    if (!same || f.better_or_equal(shown)) {
        shown = f;
        make_images();
        update_formula_scroll();
        InvalidateRect(view, 0, TRUE);
    }
    if (changed && !shown.settled() && f.level < AUTO_LEVELS) {
        level_target = f.target;
        level_gen = f.gen;
        level_for = f.level + 1;
        start(f.target, level_for);
    }
}

LRESULT CALLBACK view_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_ERASEBKGND:
        return 1;
    case WM_PAINT: {
        PAINTSTRUCT ps;
        HDC dc = BeginPaint(h, &ps);
        RECT rc;
        GetClientRect(h, &rc);
        HDC mem = CreateCompatibleDC(dc);
        HBITMAP bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
        HGDIOBJ old = SelectObject(mem, bmp);
        FillRect(mem, &rc, (HBRUSH)GetStockObject(WHITE_BRUSH));
        {
            Graphics g(mem);
            int x0 = -GetScrollPos(h, SB_HORZ), y = 8 - GetScrollPos(h, SB_VERT);
            if (shown.valid && !shown.err.empty()) {
                RectF r(8.0f, 8.0f, (REAL)rc.right - 16, (REAL)rc.bottom - 16);
                LOGFONTW lf;
                GetObjectW(g_ui_font, sizeof lf, &lf);
                Font font(mem, &lf);
                SolidBrush br(Color(255, 160, 0, 0));
                g.DrawString(shown.err.c_str(), -1, &font, r, 0, &br);
            }
            for (size_t k = 0; k < images.size(); k++) {
                if (!images[k]) continue;
                g.DrawImage(images[k], x0 + 8, y, images[k]->GetWidth(), images[k]->GetHeight());
                y += images[k]->GetHeight() + 8;
            }
        }
        BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        EndPaint(h, &ps);
        return 0;
    }
    case WM_SIZE:
        if (shown.valid) {
            make_images();
            update_formula_scroll();
            InvalidateRect(h, 0, FALSE);
        }
        return 0;
    case WM_HSCROLL:
    case WM_VSCROLL: {
        int bar = msg == WM_HSCROLL ? SB_HORZ : SB_VERT;
        SCROLLINFO si;
        si.cbSize = sizeof si;
        si.fMask = SIF_ALL;
        GetScrollInfo(h, bar, &si);
        int pos = si.nPos;
        switch (LOWORD(wp)) {
        case SB_LINEUP: pos -= 20; break;
        case SB_LINEDOWN: pos += 20; break;
        case SB_PAGEUP: pos -= (int)si.nPage; break;
        case SB_PAGEDOWN: pos += (int)si.nPage; break;
        case SB_THUMBTRACK: pos = si.nTrackPos; break;
        }
        si.fMask = SIF_POS;
        si.nPos = pos;
        SetScrollInfo(h, bar, &si, TRUE);
        InvalidateRect(h, 0, FALSE);
        return 0;
    }
    case WM_MOUSEWHEEL:
        SendMessage(h, WM_VSCROLL, GET_WHEEL_DELTA_WPARAM(wp) > 0 ? SB_LINEUP : SB_LINEDOWN, 0);
        return 0;
    case WM_CONTEXTMENU: {
        HMENU m = CreatePopupMenu();
        AppendMenuW(m, MF_STRING | (shown.valid && shown.err.empty() ? 0 : MF_GRAYED), ID_EDIT_COPY_FORMULA, L"&Copy Formula");
        AppendMenuW(m, MF_STRING | (shown.valid && shown.err.empty() ? 0 : MF_GRAYED), ID_EDIT_COPY_LATEX, L"Copy as &LaTeX");
        AppendMenuW(m, MF_STRING | (shown.valid && shown.err.empty() ? 0 : MF_GRAYED), ID_EDIT_COPY_PICTURE, L"Copy &Picture");
        AppendMenuW(m, MF_SEPARATOR, 0, 0);
        AppendMenuW(m, MF_STRING | (shown.valid && shown.err.empty() ? 0 : MF_GRAYED), ID_FILE_EXPORT, L"&Save Picture As...");
        TrackPopupMenu(m, TPM_RIGHTBUTTON, GET_X_LPARAM(lp), GET_Y_LPARAM(lp), 0, g_frame, 0);
        DestroyMenu(m);
        return 0;
    }
    }
    return DefWindowProc(h, msg, wp, lp);
}

// ------------------------------------------------------------ the lists

Tab current_tab() { return (Tab)TabCtrl_GetCurSel(tabs); }

void add_column(int i, const wchar_t *name, int width) {
    LVCOLUMNW c;
    c.mask = LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM;
    c.pszText = (LPWSTR)name;
    c.cx = width;
    c.iSubItem = i;
    ListView_InsertColumn(list, i, &c);
}

void set_columns() {
    while (ListView_DeleteColumn(list, 0)) {
    }
    switch (current_tab()) {
    case TAB_DIMS:
        add_column(0, L"Name", 52);
        add_column(1, L"Value", 90);
        add_column(2, L"Type", 70);
        add_column(3, L"In Formulas", 76);
        ListView_SetImageList(list, 0, LVSIL_SMALL);
        break;
    case TAB_VARS:
        add_column(0, L"Name", 52);
        add_column(1, L"Value", 80);
        add_column(2, L"Type", 60);
        add_column(3, L"Definition", 100);
        ListView_SetImageList(list, 0, LVSIL_SMALL);
        break;
    case TAB_FUNCS:
        add_column(0, L"Equation", 280);
        ListView_SetImageList(list, swatches, LVSIL_SMALL);
        break;
    }
}

void add_row(size_t index, const wstr *cells, int n, int image) {
    LVITEMW it;
    memset(&it, 0, sizeof it);
    it.mask = LVIF_TEXT | LVIF_PARAM | (image >= 0 ? LVIF_IMAGE : 0);
    it.iItem = ListView_GetItemCount(list);
    it.pszText = (LPWSTR)cells[0].c_str();
    it.lParam = (LPARAM)index;
    it.iImage = image;
    int row = ListView_InsertItem(list, &it);
    for (int k = 1; k < n; k++) ListView_SetItemText(list, row, k, (LPWSTR)cells[k].c_str());
}

Sel row_sel(int row) {
    LVITEMW it;
    memset(&it, 0, sizeof it);
    it.mask = LVIF_PARAM;
    it.iItem = row;
    ListView_GetItem(list, &it);
    switch (current_tab()) {
    case TAB_DIMS: return Sel(Sel::DIM, (size_t)it.lParam);
    case TAB_VARS: return Sel(Sel::VAR, (size_t)it.lParam);
    default: return Sel(Sel::ENT, (size_t)it.lParam);
    }
}

void fill_list() {
    GxDoc *d = app.doc;
    filling = true;
    int top = ListView_GetTopIndex(list);
    SendMessage(list, WM_SETREDRAW, FALSE, 0);
    ListView_DeleteAllItems(list);
    switch (current_tab()) {
    case TAB_DIMS:
        for (size_t i = 0; i < gx_doc_dim_count(d); i++) {
            int kind = gx_doc_dim_kind(d, i);
            if (kind < 0) continue;
            bool driving = gx_doc_dim_driving(d, i) == 1;
            wstr expr = take(gx_doc_dim_expr(d, i));
            wstr cells[4];
            cells[0] = geo::dim_name(d, i);
            cells[1] = !expr.empty() ? expr : fmt_dim(driving ? gx_doc_dim_set_value(d, i) : gx_doc_dim_value(d, i), kind == GX_DIM_ANGLE);
            cells[2] = driving ? L"Driving" : L"Measured";
            cells[3] = gx_doc_dim_numeric(d, i) == 1 ? L"Number" : L"Symbol";
            add_row(i, cells, 4, -1);
        }
        break;
    case TAB_VARS:
        for (size_t i = 0; i < gx_doc_var_count(d); i++) {
            int fr = gx_doc_var_free(d, i);
            if (fr < 0) continue;
            wstr expr = take(gx_doc_var_expr(d, i));
            wstr cells[4];
            cells[0] = geo::var_name(d, i);
            cells[1] = fmt_val(gx_doc_var_value(d, i));
            cells[2] = !expr.empty() ? L"Defined" : fr == 1 ? L"Free" : L"Solved";
            cells[3] = expr;
            add_row(i, cells, 4, -1);
        }
        break;
    case TAB_FUNCS:
        for (size_t e = 0; e < gx_doc_ent_count(d); e++) {
            if (!geo::is_function(d, e)) continue;
            wstr cells[1];
            cells[0] = take(gx_doc_curve_src(d, e));
            add_row(e, cells, 1, (int)(geo::function_index(d, e) % 6));
        }
        break;
    }
    // the selection of the drawing
    int n = ListView_GetItemCount(list);
    for (int r = 0; r < n; r++)
        if (app.selected(row_sel(r))) {
            ListView_SetItemState(list, r, LVIS_SELECTED | LVIS_FOCUSED, LVIS_SELECTED | LVIS_FOCUSED);
            ListView_EnsureVisible(list, r, FALSE);
        }
    if (top > 0 && top < n) ListView_EnsureVisible(list, n - 1, FALSE), ListView_EnsureVisible(list, top, FALSE);
    SendMessage(list, WM_SETREDRAW, TRUE, 0);
    InvalidateRect(list, 0, TRUE);
    filling = false;
    bool any = ListView_GetSelectedCount(list) > 0;
    EnableWindow(btn_add, current_tab() != TAB_DIMS);
    EnableWindow(btn_props, any);
    EnableWindow(btn_delete, any);
}

void make_swatches() {
    swatches = ImageList_Create(16, 16, ILC_COLOR32, 6, 0);
    HDC sdc = GetDC(0);
    for (int k = 0; k < 6; k++) {
        BITMAPINFO bi;
        memset(&bi, 0, sizeof bi);
        bi.bmiHeader.biSize = sizeof bi.bmiHeader;
        bi.bmiHeader.biWidth = 16;
        bi.bmiHeader.biHeight = -16;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        void *bits;
        HBITMAP b = CreateDIBSection(sdc, &bi, DIB_RGB_COLORS, &bits, 0, 0);
        DWORD *px = (DWORD *)bits;
        const Color &c = col::functions[k];
        for (int y = 0; y < 16; y++)
            for (int x = 0; x < 16; x++) {
                bool in = x >= 3 && x < 13 && y >= 3 && y < 13;
                bool edge = in && (x == 3 || x == 12 || y == 3 || y == 12);
                DWORD v = 0;
                if (edge) v = 0xFF404040;
                else if (in) v = 0xFF000000 | (c.GetR() << 16) | (c.GetG() << 8) | c.GetB();
                px[y * 16 + x] = v;
            }
        ImageList_Add(swatches, b, 0);
        DeleteObject(b);
    }
    ReleaseDC(0, sdc);
}

void list_props() {
    if (ListView_GetSelectedCount(list) == 0) return;
    dlg_properties(g_frame);
}

LRESULT CALLBACK pane_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_SIZE:
        pane_layout();
        return 0;
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDC_PANE_ADD:
            if (current_tab() == TAB_VARS) dlg_variable(g_frame, NONE);
            else if (current_tab() == TAB_FUNCS) dlg_function(g_frame, NONE);
            return 0;
        case IDC_PANE_PROPS: list_props(); return 0;
        case IDC_PANE_DELETE: delete_selection(); return 0;
        case IDC_FORMULA_COPY: pane_copy_formula(false); return 0;
        case IDC_FORMULA_LATEX: pane_copy_formula(true); return 0;
        case IDC_FORMULA_LONGER: pane_search_longer(); return 0;
        }
        break;
    case WM_NOTIFY: {
        NMHDR *n = (NMHDR *)lp;
        if (n->hwndFrom == tabs && n->code == TCN_SELCHANGE) {
            set_columns();
            fill_list();
            return 0;
        }
        if (n->hwndFrom == list) {
            if (n->code == LVN_ITEMCHANGED && !filling) {
                NMLISTVIEW *v = (NMLISTVIEW *)lp;
                if ((v->uChanged & LVIF_STATE) && (v->uNewState & LVIS_SELECTED) && !(v->uOldState & LVIS_SELECTED)) {
                    app.select_only(row_sel(v->iItem));
                    sel_changed();
                }
                bool any = ListView_GetSelectedCount(list) > 0;
                EnableWindow(btn_props, any);
                EnableWindow(btn_delete, any);
            } else if (n->code == NM_DBLCLK || n->code == NM_RETURN) {
                list_props();
            } else if (n->code == NM_RCLICK) {
                NMITEMACTIVATE *a = (NMITEMACTIVATE *)lp;
                POINT pt = a->ptAction;
                ClientToScreen(list, &pt);
                HMENU menu = LoadMenuW(g_inst, MAKEINTRESOURCEW(IDR_CONTEXT_LIST));
                HMENU popup = GetSubMenu(menu, current_tab());
                if (a->iItem >= 0) SetMenuDefaultItem(popup, ID_EDIT_PROPERTIES, FALSE);
                TrackPopupMenu(popup, TPM_RIGHTBUTTON, pt.x, pt.y, 0, g_frame, 0);
                DestroyMenu(menu);
            } else if (n->code == LVN_KEYDOWN && ((NMLVKEYDOWN *)lp)->wVKey == VK_DELETE) {
                delete_selection();
            }
            return 0;
        }
        break;
    }
    case WM_APP_FORMULA: {
        Job *j = (Job *)lp;
        CloseHandle(j->thread);
        if (j == job) {
            job = 0;
            take_result(j);
        }
        gx_cancel_free(j->cancel);
        gx_formula_free(j->f);
        delete j;
        show_state();
        return 0;
    }
    case WM_CTLCOLORSTATIC:
        if ((HWND)lp == state && shown.valid && !shown.err.empty()) {
            SetTextColor((HDC)wp, RGB(160, 0, 0));
            SetBkMode((HDC)wp, TRANSPARENT);
            return (LRESULT)GetSysColorBrush(COLOR_BTNFACE);
        }
        break;
    }
    return DefWindowProc(h, msg, wp, lp);
}

HWND child(const wchar_t *cls, const wchar_t *txt, DWORD style, DWORD ex, int id) {
    HWND w = CreateWindowExW(ex, cls, txt, WS_CHILD | WS_VISIBLE | style, 0, 0, 10, 10, pane, (HMENU)(INT_PTR)id, g_inst, 0);
    SendMessage(w, WM_SETFONT, (WPARAM)g_ui_font, 0);
    return w;
}

} // namespace

// ------------------------------------------------------------ interface

HWND pane_create(HWND parent) {
    WNDCLASSEXW wc = {sizeof wc};
    wc.lpfnWndProc = pane_proc;
    wc.hInstance = g_inst;
    wc.hCursor = LoadCursor(0, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
    wc.lpszClassName = L"GxpadPane";
    RegisterClassExW(&wc);
    WNDCLASSEXW vc = {sizeof vc};
    vc.lpfnWndProc = view_proc;
    vc.hInstance = g_inst;
    vc.hCursor = LoadCursor(0, IDC_ARROW);
    vc.lpszClassName = L"GxpadFormula";
    RegisterClassExW(&vc);
    make_swatches();

    // no WS_CLIPCHILDREN: the pane erases the inside of the group box, which a
    // group box leaves to its parent
    pane = CreateWindowExW(WS_EX_CONTROLPARENT, L"GxpadPane", L"", WS_CHILD | WS_VISIBLE, 0, 0, 300, 400, parent,
                           (HMENU)IDC_PANE, g_inst, 0);
    tabs = child(WC_TABCONTROLW, L"", WS_CLIPSIBLINGS | WS_TABSTOP, 0, IDC_TABS);
    const wchar_t *names[3] = {L"Dimensions", L"Variables", L"Functions"};
    for (int k = 0; k < 3; k++) {
        TCITEMW ti;
        ti.mask = TCIF_TEXT;
        ti.pszText = (LPWSTR)names[k];
        TabCtrl_InsertItem(tabs, k, &ti);
    }
    list = child(WC_LISTVIEWW, L"", LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS | WS_TABSTOP, WS_EX_CLIENTEDGE, IDC_LIST);
    ListView_SetExtendedListViewStyle(list, LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | LVS_EX_DOUBLEBUFFER);
    SetWindowPos(list, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    btn_add = child(L"BUTTON", L"&Add...", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_PANE_ADD);
    btn_props = child(L"BUTTON", L"P&roperties", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_PANE_PROPS);
    btn_delete = child(L"BUTTON", L"&Delete", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_PANE_DELETE);
    box = child(L"BUTTON", L"Formula", BS_GROUPBOX | WS_CLIPSIBLINGS, 0, IDC_FORMULA_BOX);
    text = child(L"STATIC", L"", SS_LEFT | SS_ENDELLIPSIS | SS_NOPREFIX, 0, IDC_FORMULA_TEXT);
    view = child(L"GxpadFormula", L"", WS_HSCROLL | WS_VSCROLL, WS_EX_CLIENTEDGE, IDC_FORMULA_VIEW);
    btn_copy = child(L"BUTTON", L"&Copy", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_FORMULA_COPY);
    btn_latex = child(L"BUTTON", L"Copy &LaTeX", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_FORMULA_LATEX);
    btn_longer = child(L"BUTTON", L"&Search Longer", BS_PUSHBUTTON | WS_TABSTOP, 0, IDC_FORMULA_LONGER);
    progress = child(PROGRESS_CLASSW, L"", PBS_MARQUEE, 0, IDC_FORMULA_PROGRESS);
    state = child(L"STATIC", L"", SS_LEFT | SS_NOPREFIX, 0, IDC_FORMULA_STATE);
    ShowWindow(progress, SW_HIDE);
    // the group box goes below the controls inside it
    SetWindowPos(box, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    set_columns();
    fill_list();
    show_state();
    return pane;
}

void pane_layout() {
    RECT rc;
    GetClientRect(pane, &rc);
    int W = rc.right, H = rc.bottom, m = 6, bh = 23, gap = 4;
    int list_h = (int)((H - 3 * m) * 0.48);
    // the tabs with the list on their page, the buttons below them
    int tabs_h = list_h - bh - gap;
    MoveWindow(tabs, m, m, W - 2 * m, tabs_h, TRUE);
    RECT page = {m, m, W - m, m + tabs_h};
    TabCtrl_AdjustRect(tabs, FALSE, &page);
    MoveWindow(list, page.left + 2, page.top + 2, page.right - page.left - 4, page.bottom - page.top - 4, TRUE);
    int by = m + tabs_h + gap, bw = 74;
    MoveWindow(btn_add, m, by, bw, bh, TRUE);
    MoveWindow(btn_props, m + bw + gap, by, bw, bh, TRUE);
    MoveWindow(btn_delete, m + 2 * (bw + gap), by, bw, bh, TRUE);
    // the formula
    int top = by + bh + m + 2;
    MoveWindow(box, m, top, W - 2 * m, H - top - m, TRUE);
    int ix = 2 * m + 2, iw = W - 4 * m - 4, y = top + 18;
    MoveWindow(text, ix, y, iw, 16, TRUE);
    y += 20;
    int bottom = H - m - 8;
    int sy = bottom - 16, rowy = sy - gap - bh;
    MoveWindow(view, ix, y, iw, rowy - gap - y > 20 ? rowy - gap - y : 20, TRUE);
    int b1 = 60, b2 = 84, b3 = 96;
    MoveWindow(btn_copy, ix, rowy, b1, bh, TRUE);
    MoveWindow(btn_latex, ix + b1 + gap, rowy, b2, bh, TRUE);
    MoveWindow(btn_longer, ix + b1 + b2 + 2 * gap, rowy, b3, bh, TRUE);
    // the moving bar shows a running search next to the buttons
    int px = ix + b1 + b2 + b3 + 3 * gap + 2;
    MoveWindow(progress, px, rowy + 5, ix + iw - px > 90 ? 90 : ix + iw - px, 13, TRUE);
    MoveWindow(state, ix, sy, iw, 16, TRUE);
}

void pane_refresh() {
    if (!pane) return;
    fill_list();
    pane_formula_update();
}

// A driving dimension with a plain value, or a free variable, sets its own
// value: its formula would only repeat its name.
bool sets_itself(Sel t) {
    GxDoc *d = app.doc;
    if (t.kind == Sel::DIM) return gx_doc_dim_driving(d, t.i) == 1 && take(gx_doc_dim_expr(d, t.i)).empty();
    return gx_doc_var_free(d, t.i) == 1 && take(gx_doc_var_expr(d, t.i)).empty();
}

void pane_formula_update() {
    if (!pane) return;
    Sel t;
    if (!app.formula_target(&t) || sets_itself(t)) {
        cancel_job();
        shown = Formula();
        free_images();
        update_formula_scroll();
        InvalidateRect(view, 0, TRUE);
        show_state();
        return;
    }
    uint64_t gen = gx_doc_generation(app.doc);
    unsigned level = level_of(t);
    bool same_shown = shown.valid && shown.target == t && shown.gen == gen;
    bool running = job && job->target == t && job->gen == gen && job->level == level;
    if (!running && !(same_shown && shown.level >= level)) {
        if (shown.valid && shown.target != t) {
            shown = Formula();
            free_images();
            update_formula_scroll();
            InvalidateRect(view, 0, TRUE);
        }
        start(t, level);
    }
    show_state();
}

bool pane_can_search_longer() { return shown.valid && !shown.settled() && job == 0; }

void pane_search_longer() {
    if (!pane_can_search_longer()) return;
    level_target = shown.target;
    level_gen = gx_doc_generation(app.doc);
    level_for = level_of(shown.target) + 1;
    if (level_for <= shown.level) level_for = shown.level + 1;
    start(shown.target, level_for);
    show_state();
}

bool pane_has_formula() { return shown.valid && shown.err.empty() && !shown.rows.empty(); }

static void clip_text(const wstr &s) {
    if (!OpenClipboard(g_frame)) return;
    EmptyClipboard();
    HGLOBAL mem = GlobalAlloc(GMEM_MOVEABLE, (s.size() + 1) * sizeof(wchar_t));
    if (mem) {
        memcpy(GlobalLock(mem), s.c_str(), (s.size() + 1) * sizeof(wchar_t));
        GlobalUnlock(mem);
        SetClipboardData(CF_UNICODETEXT, mem);
    }
    CloseClipboard();
}

void pane_copy_formula(bool latex) {
    if (!pane_has_formula()) return;
    clip_text(latex ? widen(shown.latex.c_str()) : shown.plain);
    set_status(latex ? L"The formula was copied as LaTeX." : L"The formula was copied.");
}

void pane_copy_picture() {
    if (!pane_has_formula()) return;
    std::vector<unsigned char> png = typeset(shown.latex, 14, screen_dpi());
    Bitmap *b = png.empty() ? 0 : decode_png(&png[0], png.size());
    if (!b) return;
    // on white, for programs that ignore transparency
    int w = b->GetWidth() + 8, h = b->GetHeight() + 8;
    HDC sdc = GetDC(0);
    HDC mem = CreateCompatibleDC(sdc);
    HBITMAP bmp = CreateCompatibleBitmap(sdc, w, h);
    HGDIOBJ old = SelectObject(mem, bmp);
    RECT r = {0, 0, w, h};
    FillRect(mem, &r, (HBRUSH)GetStockObject(WHITE_BRUSH));
    {
        Graphics g(mem);
        g.DrawImage(b, 4, 4, b->GetWidth(), b->GetHeight());
    }
    SelectObject(mem, old);
    DeleteDC(mem);
    ReleaseDC(0, sdc);
    delete b;
    if (OpenClipboard(g_frame)) {
        EmptyClipboard();
        SetClipboardData(CF_BITMAP, bmp);
        CloseClipboard();
        set_status(L"The formula picture was copied.");
    } else {
        DeleteObject(bmp);
    }
}

void pane_save_picture() {
    if (!pane_has_formula()) return;
    wchar_t file[MAX_PATH];
    wstr name = shown.name;
    for (size_t k = 0; k < name.size(); k++)
        if (wcschr(L"\\/:*?\"<>|", name[k])) name[k] = L'_';
    lstrcpynW(file, (name + L".png").c_str(), MAX_PATH);
    wstr filter = load_str(IDS_PNG_FILTER);
    for (size_t k = 0; k < filter.size(); k++)
        if (filter[k] == L'|') filter[k] = 0;
    OPENFILENAMEW of;
    memset(&of, 0, sizeof of);
    of.lStructSize = sizeof of;
    of.hwndOwner = g_frame;
    of.lpstrFilter = filter.c_str();
    of.lpstrFile = file;
    of.nMaxFile = MAX_PATH;
    of.lpstrDefExt = L"png";
    of.lpstrTitle = L"Save Formula Picture";
    of.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
    if (!GetSaveFileNameW(&of)) return;
    // 12 pt at 300 dpi: sharp enough to print
    std::vector<unsigned char> png = typeset(shown.latex, 12, 300);
    HANDLE f = CreateFileW(file, GENERIC_WRITE, 0, 0, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0);
    DWORD written = 0;
    bool ok = f != INVALID_HANDLE_VALUE && !png.empty() && WriteFile(f, &png[0], (DWORD)png.size(), &written, 0) &&
              written == png.size();
    if (f != INVALID_HANDLE_VALUE) CloseHandle(f);
    if (!ok) MessageBoxW(g_frame, L"The picture could not be saved.", load_str(IDS_APP).c_str(), MB_ICONERROR);
}

void pane_cancel_jobs() {
    if (job) {
        HANDLE t = job->thread;
        gx_cancel_set(job->cancel);
        WaitForSingleObject(t, 3000);
        job = 0;
    }
}

void pane_print_formula(HDC dc, int x, int y, int w, int dpi) {
    if (!pane_has_formula()) return;
    std::vector<unsigned char> png = typeset(shown.latex, 12, dpi);
    Bitmap *b = png.empty() ? 0 : decode_png(&png[0], png.size());
    if (!b) return;
    double k = dmin(1.0, (double)w / b->GetWidth());
    Graphics g(dc);
    g.SetPageUnit(UnitPixel);
    g.DrawImage(b, x, y, (INT)(b->GetWidth() * k), (INT)(b->GetHeight() * k));
    delete b;
}
