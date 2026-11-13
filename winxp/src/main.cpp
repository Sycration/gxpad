// The main window: menus, toolbars, tool box, status bar, files, printing
// and settings.
#include "common.h"

#pragma comment(lib, "comctl32.lib")
#pragma comment(lib, "gdiplus.lib")
#pragma comment(lib, "comdlg32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "gxgeo.lib")
// the common controls of Windows XP, with visual styles
#pragma comment(linker, "\"/manifestdependency:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' " \
                        "processorArchitecture='x86' publicKeyToken='6595b64144ccf1df' language='*'\"")

App app;
HINSTANCE g_inst;
HWND g_frame;
HFONT g_ui_font;
HIMAGELIST g_icons;

namespace {

const wchar_t *REG_KEY = L"Software\\gxpad";
const int MRU_MAX = 4;

HWND rebar, toolbar, conbar, toolbox_box, toolbox, statusbar, canvas, pane, zoom_box;
HACCEL accel;
int pane_w = 330;
bool show_toolbar = true, show_conbar = true, show_toolbox = true, show_status = true, show_pane = true;
bool splitting;
std::vector<wstr> mru;
const int SPLIT = 5;

// ------------------------------------------------------------ settings

DWORD reg_get(const wchar_t *name, DWORD def) {
    HKEY k;
    DWORD v = def, size = sizeof v, type;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, (wstr(REG_KEY) + L"\\Settings").c_str(), 0, KEY_READ, &k) == ERROR_SUCCESS) {
        if (RegQueryValueExW(k, name, 0, &type, (BYTE *)&v, &size) != ERROR_SUCCESS || type != REG_DWORD) v = def;
        RegCloseKey(k);
    }
    return v;
}

void reg_set(const wchar_t *name, DWORD v) {
    HKEY k;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, (wstr(REG_KEY) + L"\\Settings").c_str(), 0, 0, 0, KEY_WRITE, 0, &k, 0) == ERROR_SUCCESS) {
        RegSetValueExW(k, name, 0, REG_DWORD, (const BYTE *)&v, sizeof v);
        RegCloseKey(k);
    }
}

void load_mru() {
    mru.clear();
    HKEY k;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, (wstr(REG_KEY) + L"\\Recent File List").c_str(), 0, KEY_READ, &k) != ERROR_SUCCESS) return;
    for (int i = 1; i <= MRU_MAX; i++) {
        wchar_t name[16], buf[MAX_PATH];
        DWORD size = sizeof buf, type;
        swprintf(name, 16, L"File%d", i);
        if (RegQueryValueExW(k, name, 0, &type, (BYTE *)buf, &size) == ERROR_SUCCESS && type == REG_SZ) mru.push_back(buf);
    }
    RegCloseKey(k);
}

void save_mru() {
    HKEY k;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, (wstr(REG_KEY) + L"\\Recent File List").c_str(), 0, 0, 0, KEY_WRITE, 0, &k, 0) != ERROR_SUCCESS)
        return;
    for (int i = 1; i <= MRU_MAX; i++) {
        wchar_t name[16];
        swprintf(name, 16, L"File%d", i);
        if (i <= (int)mru.size())
            RegSetValueExW(k, name, 0, REG_SZ, (const BYTE *)mru[i - 1].c_str(), (DWORD)((mru[i - 1].size() + 1) * sizeof(wchar_t)));
        else
            RegDeleteValueW(k, name);
    }
    RegCloseKey(k);
}

void add_mru(const wstr &path) {
    for (size_t i = 0; i < mru.size(); i++)
        if (lstrcmpiW(mru[i].c_str(), path.c_str()) == 0) {
            mru.erase(mru.begin() + i);
            break;
        }
    mru.insert(mru.begin(), path);
    if (mru.size() > (size_t)MRU_MAX) mru.resize(MRU_MAX);
    save_mru();
}

// ------------------------------------------------------------ status and title

wstr file_title() {
    if (app.path.empty()) return load_str(IDS_UNTITLED);
    size_t s = app.path.find_last_of(L"\\/");
    return s == wstr::npos ? app.path : app.path.substr(s + 1);
}

void update_title() {
    wstr t = file_title() + L" - " + load_str(IDS_APP);
    SetWindowTextW(g_frame, t.c_str());
}

void status_part(int part, const wstr &s) { SendMessageW(statusbar, SB_SETTEXTW, part, (LPARAM)s.c_str()); }

void update_status_info() {
    if (!statusbar) return;
    GxDoc *d = app.doc;
    size_t conflicts = gx_doc_conflict_count(d);
    ptrdiff_t dof = gx_doc_dof(d);
    wchar_t buf[128];
    if (conflicts > 0) swprintf(buf, 128, conflicts == 1 ? L"%u conflict" : L"%u conflicts", (unsigned)conflicts);
    else if (dof == 0) swprintf(buf, 128, L"Fully constrained");
    else swprintf(buf, 128, dof == 1 ? L"%d degree of freedom" : L"%d degrees of freedom", (int)dof);
    status_part(2, buf);
    swprintf(buf, 128, L"%d%%", iround(canvas_zoom_percent()));
    status_part(3, buf);
}

void update_coords() {
    V2 m = canvas_mouse();
    status_part(1, m.ok() ? L"X: " + fmt_val(m.x) + L"   Y: " + fmt_val(m.y) : L"");
}

void layout_status() {
    RECT rc;
    GetClientRect(statusbar, &rc);
    int parts[4] = {rc.right - 420, rc.right - 270, rc.right - 70, -1};
    if (parts[0] < 50) parts[0] = 50;
    SendMessage(statusbar, SB_SETPARTS, 4, (LPARAM)parts);
}

// ------------------------------------------------------------ command state

bool cmd_state(UINT id, bool *checked) {
    *checked = false;
    GxDoc *d = app.doc;
    int t = cmd_tool(id);
    if (t >= 0) {
        *checked = app.tool == (Tool)t;
        return true;
    }
    if ((id >= ID_CON_FIRST && id <= ID_CON_INFINITE) || (id >= ID_DIM_FIRST && id <= ID_DIM_SUPP)) return action_enabled(id, checked);
    switch (id) {
    case ID_FILE_EXPORT: return pane_has_formula();
    case ID_EDIT_UNDO: return gx_doc_undo_len(d) > 0;
    case ID_EDIT_REDO: return app.redo_steps > 0;
    case ID_EDIT_COPY_FORMULA:
    case ID_EDIT_COPY_LATEX:
    case ID_EDIT_COPY_PICTURE: return pane_has_formula();
    case ID_EDIT_DELETE:
        for (size_t k = 0; k < app.sel.size(); k++)
            if (app.sel[k].kind != Sel::ENT || !geo::is_fixed(d, app.sel[k].i)) return true;
        return false;
    case ID_EDIT_PROPERTIES: return app.sel.size() == 1;
    case ID_VIEW_TOOLBAR: *checked = show_toolbar; return true;
    case ID_VIEW_CONBAR: *checked = show_conbar; return true;
    case ID_VIEW_TOOLBOX: *checked = show_toolbox; return true;
    case ID_VIEW_STATUSBAR: *checked = show_status; return true;
    case ID_VIEW_PANE: *checked = show_pane; return true;
    case ID_VIEW_GRID: *checked = app.show_grid; return true;
    case ID_VIEW_SYMBOLS: *checked = app.show_symbols; return true;
    case ID_FORMULA_LONGER: return pane_can_search_longer();
    }
    return true;
}

void update_bar(HWND bar) {
    int n = (int)SendMessage(bar, TB_BUTTONCOUNT, 0, 0);
    for (int i = 0; i < n; i++) {
        TBBUTTON b;
        SendMessage(bar, TB_GETBUTTON, i, (LPARAM)&b);
        if (b.fsStyle & BTNS_SEP) continue;
        bool ck;
        bool en = cmd_state(b.idCommand, &ck);
        SendMessage(bar, TB_ENABLEBUTTON, b.idCommand, MAKELONG(en, 0));
        SendMessage(bar, TB_CHECKBUTTON, b.idCommand, MAKELONG(ck, 0));
    }
}

void update_bars() {
    update_bar(toolbar);
    update_bar(conbar);
    update_bar(toolbox);
}

void init_popup(HMENU m) {
    int n = GetMenuItemCount(m);
    for (int i = 0; i < n; i++) {
        UINT id = GetMenuItemID(m, i);
        if (id == (UINT)-1 || id == 0) continue;
        if (id >= ID_FILE_MRU1 && id <= ID_FILE_MRU4) continue;
        bool ck;
        bool en = cmd_state(id, &ck);
        EnableMenuItem(m, i, MF_BYPOSITION | (en ? MF_ENABLED : MF_GRAYED));
        int t = cmd_tool(id);
        if (t >= 0 && ck) CheckMenuRadioItem(m, i, i, i, MF_BYPOSITION);
        else CheckMenuItem(m, i, MF_BYPOSITION | (ck ? MF_CHECKED : MF_UNCHECKED));
    }
}

// The recent files go between the separators before Exit.
void init_file_menu(HMENU m) {
    for (UINT id = ID_FILE_MRU1; id <= ID_FILE_MRU4; id++) DeleteMenu(m, id, MF_BYCOMMAND);
    int n = GetMenuItemCount(m), exit_at = -1;
    for (int i = 0; i < n; i++)
        if (GetMenuItemID(m, i) == ID_FILE_EXIT) exit_at = i;
    if (exit_at < 1) return;
    int at = exit_at - 1;  // the separator before Exit
    if (mru.empty()) {
        InsertMenuW(m, at, MF_BYPOSITION | MF_STRING | MF_GRAYED, ID_FILE_MRU1, L"Recent File");
        return;
    }
    for (size_t i = 0; i < mru.size(); i++) {
        wstr p = mru[i];
        if (p.size() > 40) p = p.substr(0, 3) + L"..." + p.substr(p.size() - 34);
        wchar_t buf[MAX_PATH + 8];
        swprintf(buf, MAX_PATH + 8, L"&%d %s", (int)i + 1, p.c_str());
        InsertMenuW(m, at + (int)i, MF_BYPOSITION | MF_STRING, ID_FILE_MRU1 + (UINT)i, buf);
    }
}

// ------------------------------------------------------------ files

bool read_file(const wstr &path, std::string *out) {
    HANDLE f = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, 0, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, 0);
    if (f == INVALID_HANDLE_VALUE) return false;
    DWORD size = GetFileSize(f, 0), got = 0;
    out->resize(size);
    bool ok = size == 0 || (ReadFile(f, &(*out)[0], size, &got, 0) && got == size);
    CloseHandle(f);
    return ok;
}

bool write_file(const wstr &path, const std::string &s) {
    HANDLE f = CreateFileW(path.c_str(), GENERIC_WRITE, 0, 0, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0);
    if (f == INVALID_HANDLE_VALUE) return false;
    DWORD put = 0;
    bool ok = s.empty() || (WriteFile(f, s.data(), (DWORD)s.size(), &put, 0) && put == s.size());
    CloseHandle(f);
    return ok;
}

void replace_doc(GxDoc *d, const wstr &path) {
    pane_cancel_jobs();
    GxDoc *old = app.doc;
    app.doc = d;
    gx_doc_free(old);
    app.path = path;
    app.sel.clear();
    app.redo_steps = 0;
    app.undo_seen = 0;
    canvas_cancel();
    gx_doc_solve(d, 0);
    app.saved_gen = gx_doc_generation(d);
    update_title();
    doc_changed();
    canvas_fit();
}

bool save_as();

bool save() {
    if (app.path.empty()) return save_as();
    char *text = gx_doc_to_gxp(app.doc);
    bool ok = text && write_file(app.path, text);
    gx_string_free(text);
    if (!ok) {
        MessageBoxW(g_frame, (load_str(IDS_SAVE_FAILED) + L"\n\n" + app.path).c_str(), load_str(IDS_APP).c_str(), MB_ICONERROR);
        return false;
    }
    app.saved_gen = gx_doc_generation(app.doc);
    add_mru(app.path);
    update_title();
    return true;
}

wstr filter_of(UINT id) {
    wstr f = load_str(id);
    for (size_t k = 0; k < f.size(); k++)
        if (f[k] == L'|') f[k] = 0;
    return f;
}

bool save_as() {
    wchar_t file[MAX_PATH];
    lstrcpynW(file, app.path.empty() ? L"Untitled.gxp" : app.path.c_str(), MAX_PATH);
    wstr filter = filter_of(IDS_FILTER);
    OPENFILENAMEW of;
    memset(&of, 0, sizeof of);
    of.lStructSize = sizeof of;
    of.hwndOwner = g_frame;
    of.lpstrFilter = filter.c_str();
    of.lpstrFile = file;
    of.nMaxFile = MAX_PATH;
    of.lpstrDefExt = L"gxp";
    of.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
    if (!GetSaveFileNameW(&of)) return false;
    app.path = file;
    return save();
}

// Asks to save changes; false when the user cancels.
bool ask_save() {
    if (!app.dirty()) return true;
    wchar_t buf[MAX_PATH + 128];
    swprintf(buf, MAX_PATH + 128, load_str(IDS_SAVE_CHANGES).c_str(), file_title().c_str());
    switch (MessageBoxW(g_frame, buf, load_str(IDS_APP).c_str(), MB_YESNOCANCEL | MB_ICONEXCLAMATION)) {
    case IDYES: return save();
    case IDNO: return true;
    default: return false;
    }
}

void open_path(const wstr &path) {
    std::string text;
    GxDoc *d = 0;
    if (read_file(path, &text)) d = gx_doc_from_gxp(text.c_str());
    if (!d) {
        wstr e = last_error();
        MessageBoxW(g_frame, (load_str(IDS_OPEN_FAILED) + L"\n\n" + path + (e.empty() ? L"" : L"\n\n" + e)).c_str(),
                    load_str(IDS_APP).c_str(), MB_ICONERROR);
        for (size_t i = 0; i < mru.size(); i++)
            if (mru[i] == path) {
                mru.erase(mru.begin() + i);
                save_mru();
                break;
            }
        return;
    }
    replace_doc(d, path);
    add_mru(path);
}

void open_dialog() {
    if (!ask_save()) return;
    wchar_t file[MAX_PATH] = L"";
    wstr filter = filter_of(IDS_FILTER);
    OPENFILENAMEW of;
    memset(&of, 0, sizeof of);
    of.lStructSize = sizeof of;
    of.hwndOwner = g_frame;
    of.lpstrFilter = filter.c_str();
    of.lpstrFile = file;
    of.nMaxFile = MAX_PATH;
    of.lpstrDefExt = L"gxp";
    of.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
    if (GetOpenFileNameW(&of)) open_path(file);
}

// ------------------------------------------------------------ printing

void print() {
    PRINTDLGW pd;
    memset(&pd, 0, sizeof pd);
    pd.lStructSize = sizeof pd;
    pd.hwndOwner = g_frame;
    pd.Flags = PD_RETURNDC | PD_NOPAGENUMS | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
    if (!PrintDlgW(&pd) || !pd.hDC) return;
    HDC dc = pd.hDC;
    wstr title = file_title();
    DOCINFOW di;
    memset(&di, 0, sizeof di);
    di.cbSize = sizeof di;
    di.lpszDocName = title.c_str();
    SetCursor(LoadCursor(0, IDC_WAIT));
    if (StartDocW(dc, &di) > 0 && StartPage(dc) > 0) {
        int W = GetDeviceCaps(dc, HORZRES), H = GetDeviceCaps(dc, VERTRES), dpi = GetDeviceCaps(dc, LOGPIXELSY);
        int m = dpi / 2;  // half an inch
        HFONT f = CreateFontW(-MulDiv(10, dpi, 72), 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, 0, 0, L"Tahoma");
        HGDIOBJ old = SelectObject(dc, f);
        TextOutW(dc, m, m, title.c_str(), (int)title.size());
        SelectObject(dc, old);
        DeleteObject(f);
        int top = m + MulDiv(24, dpi, 72);
        RECT r = {m, top, W - m, H - m};
        if (pane_has_formula()) r.bottom = H - m - dpi;  // an inch for the formula below
        canvas_print(dc, r);
        if (pane_has_formula()) {
            // the formula at 12 pt, typeset at the printer resolution
            pane_print_formula(dc, m, r.bottom + dpi / 4, W - 2 * m, dpi);
        }
        EndPage(dc);
        EndDoc(dc);
    }
    DeleteDC(dc);
    if (pd.hDevMode) GlobalFree(pd.hDevMode);
    if (pd.hDevNames) GlobalFree(pd.hDevNames);
}

// ------------------------------------------------------------ bars

HWND make_toolbar(HWND parent, int id, const TBBUTTON *btns, int n, DWORD extra) {
    HWND tb = CreateWindowExW(0, TOOLBARCLASSNAMEW, L"",
                              WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | TBSTYLE_FLAT | TBSTYLE_TOOLTIPS | CCS_NODIVIDER |
                                  CCS_NORESIZE | CCS_NOPARENTALIGN | extra,
                              0, 0, 0, 0, parent, (HMENU)(INT_PTR)id, g_inst, 0);
    SendMessage(tb, TB_BUTTONSTRUCTSIZE, sizeof(TBBUTTON), 0);
    SendMessage(tb, TB_SETIMAGELIST, 0, (LPARAM)g_icons);
    SendMessage(tb, TB_SETEXTENDEDSTYLE, 0, TBSTYLE_EX_HIDECLIPPEDBUTTONS);
    SendMessage(tb, TB_ADDBUTTONS, n, (LPARAM)btns);
    return tb;
}

TBBUTTON btn(int image, int cmd, BYTE style = BTNS_BUTTON, BYTE state = TBSTATE_ENABLED) {
    TBBUTTON b;
    memset(&b, 0, sizeof b);
    b.iBitmap = image;
    b.idCommand = cmd;
    b.fsState = state;
    b.fsStyle = style;
    return b;
}

TBBUTTON sep(int width = 0) {
    TBBUTTON b = btn(width, 0, BTNS_SEP);
    return b;
}

void add_band(HWND bar, bool brk) {
    SIZE sz;
    SendMessage(bar, TB_GETMAXSIZE, 0, (LPARAM)&sz);
    REBARBANDINFOW bi;
    memset(&bi, 0, sizeof bi);
    bi.cbSize = sizeof bi;
    bi.fMask = RBBIM_STYLE | RBBIM_CHILD | RBBIM_CHILDSIZE | RBBIM_SIZE | RBBIM_ID;
    bi.fStyle = RBBS_CHILDEDGE | RBBS_GRIPPERALWAYS | (brk ? RBBS_BREAK : 0);
    bi.hwndChild = bar;
    bi.cxMinChild = 0;
    bi.cyMinChild = sz.cy;
    bi.cx = sz.cx + 20;
    bi.wID = GetDlgCtrlID(bar);
    SendMessage(rebar, RB_INSERTBANDW, (WPARAM)-1, (LPARAM)&bi);
}

void make_bars() {
    rebar = CreateWindowExW(WS_EX_TOOLWINDOW, REBARCLASSNAMEW, L"",
                            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN | RBS_VARHEIGHT | RBS_BANDBORDERS | CCS_NODIVIDER,
                            0, 0, 0, 0, g_frame, (HMENU)IDC_REBAR, g_inst, 0);
    TBBUTTON std_btns[] = {
        btn(I_NEW, ID_FILE_NEW), btn(I_OPEN, ID_FILE_OPEN), btn(I_SAVE, ID_FILE_SAVE), sep(), btn(I_PRINT, ID_FILE_PRINT), sep(),
        btn(I_UNDO, ID_EDIT_UNDO), btn(I_REDO, ID_EDIT_REDO), sep(), btn(I_DELETE, ID_EDIT_DELETE), btn(I_PROPERTIES, ID_EDIT_PROPERTIES),
        sep(), btn(I_ZOOM_OUT, ID_VIEW_ZOOM_OUT), btn(I_ZOOM_IN, ID_VIEW_ZOOM_IN), sep(70), btn(I_FIT, ID_VIEW_FIT), sep(),
        btn(I_COPY, ID_EDIT_COPY_FORMULA), btn(I_HELP, ID_HELP_TOPICS),
    };
    toolbar = make_toolbar(rebar, IDC_TOOLBAR, std_btns, sizeof std_btns / sizeof std_btns[0], 0);
    // the zoom box sits on the wide separator
    RECT r;
    SendMessage(toolbar, TB_GETITEMRECT, 14, (LPARAM)&r);
    zoom_box = CreateWindowExW(0, L"COMBOBOX", L"", WS_CHILD | WS_VISIBLE | WS_VSCROLL | CBS_DROPDOWN, r.left + 2, r.top, 66, 200, toolbar,
                               (HMENU)IDC_ZOOM, g_inst, 0);
    SendMessage(zoom_box, WM_SETFONT, (WPARAM)g_ui_font, 0);
    const wchar_t *zooms[] = {L"10%", L"25%", L"50%", L"75%", L"100%", L"150%", L"200%", L"400%", L"800%", L"Fit"};
    for (int i = 0; i < 10; i++) SendMessageW(zoom_box, CB_ADDSTRING, 0, (LPARAM)zooms[i]);

    TBBUTTON con_btns[] = {
        btn(I_COINCIDENT, ID_CON_COINCIDENT), btn(I_ON_CURVE, ID_CON_ON_CURVE), btn(I_MIDPOINT_CON, ID_CON_MIDPOINT),
        btn(I_HORIZONTAL, ID_CON_HORIZONTAL), btn(I_VERTICAL, ID_CON_VERTICAL), btn(I_PARALLEL, ID_CON_PARALLEL),
        btn(I_PERPENDICULAR, ID_CON_PERPENDICULAR), btn(I_COLLINEAR, ID_CON_COLLINEAR), btn(I_EQUAL, ID_CON_EQUAL),
        btn(I_TANGENT, ID_CON_TANGENT), btn(I_CONCENTRIC, ID_CON_CONCENTRIC), btn(I_SYMMETRIC, ID_CON_SYMMETRIC), sep(),
        btn(I_LOCK, ID_CON_LOCK, BTNS_CHECK), btn(I_CONSTRUCTION, ID_CON_CONSTRUCTION, BTNS_CHECK), sep(),
        btn(I_DISTANCE, ID_DIM_DISTANCE), btn(I_HDIST, ID_DIM_HDIST), btn(I_VDIST, ID_DIM_VDIST), btn(I_LENGTH, ID_DIM_LENGTH),
        btn(I_RADIUS, ID_DIM_RADIUS), btn(I_DIAMETER, ID_DIM_DIAMETER), btn(I_ANGLE, ID_DIM_ANGLE), btn(I_COORD_X, ID_DIM_X),
        btn(I_COORD_Y, ID_DIM_Y),
    };
    conbar = make_toolbar(rebar, IDC_CONBAR, con_btns, sizeof con_btns / sizeof con_btns[0], 0);
    add_band(toolbar, false);
    add_band(conbar, true);

    // the tool box: two columns of tools, like Paint
    TBBUTTON tools[] = {
        btn(I_SELECT, ID_TOOL_SELECT, BTNS_CHECKGROUP), btn(I_POINT, ID_TOOL_POINT, BTNS_CHECKGROUP),
        btn(I_LINE, ID_TOOL_LINE, BTNS_CHECKGROUP), btn(I_CIRCLE, ID_TOOL_CIRCLE, BTNS_CHECKGROUP),
        btn(I_CIRCLE3, ID_TOOL_CIRCLE3, BTNS_CHECKGROUP), btn(I_ARC, ID_TOOL_ARC, BTNS_CHECKGROUP),
        btn(I_ARC3, ID_TOOL_ARC3, BTNS_CHECKGROUP), btn(I_ELLIPSE, ID_TOOL_ELLIPSE, BTNS_CHECKGROUP),
        btn(I_HYPERBOLA, ID_TOOL_HYPERBOLA, BTNS_CHECKGROUP), btn(I_PARABOLA, ID_TOOL_PARABOLA, BTNS_CHECKGROUP),
        btn(I_ELLIPSE_FOCI, ID_TOOL_ELLIPSE_FOCI, BTNS_CHECKGROUP), btn(I_HYPERBOLA_FOCI, ID_TOOL_HYPERBOLA_FOCI, BTNS_CHECKGROUP),
        btn(I_PARABOLA_FOCUS, ID_TOOL_PARABOLA_FOCUS, BTNS_CHECKGROUP), btn(I_DIMENSION, ID_TOOL_DIMENSION, BTNS_CHECKGROUP),
        btn(I_FUNCTION, ID_INSERT_FUNCTION), btn(I_VARIABLE, ID_INSERT_VARIABLE),
        btn(I_MIDPOINT, ID_TOOL_MIDPOINT, BTNS_CHECKGROUP), btn(I_INTERSECT, ID_TOOL_INTERSECT, BTNS_CHECKGROUP),
        btn(I_PERP_LINE, ID_TOOL_PERP_LINE, BTNS_CHECKGROUP), btn(I_PAR_LINE, ID_TOOL_PAR_LINE, BTNS_CHECKGROUP),
        btn(I_PERP_BISECTOR, ID_TOOL_PERP_BISECTOR, BTNS_CHECKGROUP), btn(I_ANGLE_BISECTOR, ID_TOOL_ANGLE_BISECTOR, BTNS_CHECKGROUP),
        btn(I_TANGENT_LINE, ID_TOOL_TANGENT_LINE, BTNS_CHECKGROUP), btn(I_REFLECT, ID_TOOL_REFLECT, BTNS_CHECKGROUP),
        btn(I_FOCI, ID_TOOL_FOCI, BTNS_CHECKGROUP), btn(I_DIRECTRIX, ID_TOOL_DIRECTRIX, BTNS_CHECKGROUP),
        btn(I_ASYMPTOTES, ID_TOOL_ASYMPTOTES, BTNS_CHECKGROUP),
    };
    int nt = sizeof tools / sizeof tools[0];
    for (int i = 1; i < nt; i += 2) tools[i].fsState |= TBSTATE_WRAP;
    tools[nt - 1].fsState |= TBSTATE_WRAP;
    toolbox_box = CreateWindowExW(0, L"GxpadToolbox", L"", WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN, 0, 0, 10, 10, g_frame, 0, g_inst, 0);
    toolbox = make_toolbar(toolbox_box, IDC_TOOLBOX, tools, nt, TBSTYLE_WRAPABLE);
    SendMessage(toolbox, TB_CHECKBUTTON, ID_TOOL_SELECT, TRUE);

    statusbar = CreateWindowExW(0, STATUSCLASSNAMEW, L"", WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP, 0, 0, 0, 0, g_frame, (HMENU)IDC_STATUS, g_inst, 0);
}

int toolbox_width() {
    DWORD s = (DWORD)SendMessage(toolbox, TB_GETBUTTONSIZE, 0, 0);
    return 2 * LOWORD(s) + 8;
}

void layout() {
    RECT rc;
    GetClientRect(g_frame, &rc);
    int top = 0, bottom = rc.bottom;
    ShowWindow(rebar, show_toolbar || show_conbar ? SW_SHOW : SW_HIDE);
    SendMessage(rebar, RB_SHOWBAND, 0, show_toolbar);
    SendMessage(rebar, RB_SHOWBAND, 1, show_conbar);
    if (show_toolbar || show_conbar) {
        SendMessage(rebar, WM_SIZE, 0, 0);
        RECT rr;
        GetWindowRect(rebar, &rr);
        top = rr.bottom - rr.top;
    }
    ShowWindow(statusbar, show_status ? SW_SHOW : SW_HIDE);
    if (show_status) {
        SendMessage(statusbar, WM_SIZE, 0, 0);
        RECT sr;
        GetWindowRect(statusbar, &sr);
        bottom -= sr.bottom - sr.top;
        layout_status();
    }
    int left = 0;
    ShowWindow(toolbox_box, show_toolbox ? SW_SHOW : SW_HIDE);
    if (show_toolbox) {
        int w = toolbox_width();
        MoveWindow(toolbox_box, 0, top, w, bottom - top, TRUE);
        left = w;
    }
    int pw = show_pane ? pane_w : 0;
    if (pw > rc.right - left - 150) pw = rc.right - left - 150 > 200 ? rc.right - left - 150 : 200;
    ShowWindow(pane, show_pane ? SW_SHOW : SW_HIDE);
    int cw = rc.right - left - (show_pane ? pw + SPLIT : 0);
    MoveWindow(canvas, left, top, cw > 10 ? cw : 10, bottom - top, TRUE);
    if (show_pane) MoveWindow(pane, rc.right - pw, top, pw, bottom - top, TRUE);
}

LRESULT CALLBACK toolbox_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_SIZE: {
        DWORD s = (DWORD)SendMessage(toolbox, TB_GETBUTTONSIZE, 0, 0);
        int rows = 0, n = (int)SendMessage(toolbox, TB_BUTTONCOUNT, 0, 0);
        rows = (n + 1) / 2;
        MoveWindow(toolbox, 3, 4, 2 * LOWORD(s) + 2, rows * HIWORD(s) + 4, TRUE);
        return 0;
    }
    case WM_COMMAND:
        return SendMessage(g_frame, msg, wp, lp);
    case WM_NOTIFY:
        return SendMessage(g_frame, msg, wp, lp);
    }
    return DefWindowProc(h, msg, wp, lp);
}

// ------------------------------------------------------------ commands

void zoom_from_box() {
    wstr t = window_text(zoom_box);
    if (lstrcmpiW(t.c_str(), L"Fit") == 0) {
        canvas_fit();
    } else {
        double v = _wtof(t.c_str());
        if (v > 0) canvas_set_zoom(v);
    }
    SetFocus(canvas);
}

void show_zoom() {
    wchar_t buf[16];
    swprintf(buf, 16, L"%d%%", iround(canvas_zoom_percent()));
    if (GetFocus() != zoom_box && GetParent(GetFocus()) != zoom_box) SetWindowTextW(zoom_box, buf);
    update_status_info();
}

void command(UINT id) {
    int t = cmd_tool(id);
    if (t >= 0) {
        set_tool((Tool)t);
        return;
    }
    if ((id >= ID_CON_FIRST && id <= ID_CON_INFINITE) || (id >= ID_DIM_FIRST && id <= ID_DIM_SUPP)) {
        action_run(id);
        return;
    }
    if (id >= ID_FILE_MRU1 && id <= ID_FILE_MRU4) {
        size_t i = id - ID_FILE_MRU1;
        if (i < mru.size() && ask_save()) {
            wstr p = mru[i];
            open_path(p);
        }
        return;
    }
    switch (id) {
    case ID_FILE_NEW:
        if (ask_save()) replace_doc(gx_doc_new(), L"");
        break;
    case ID_FILE_OPEN: open_dialog(); break;
    case ID_FILE_SAVE: save(); break;
    case ID_FILE_SAVE_AS: save_as(); break;
    case ID_FILE_EXPORT: pane_save_picture(); break;
    case ID_FILE_PRINT: print(); break;
    case ID_FILE_EXIT: SendMessage(g_frame, WM_CLOSE, 0, 0); break;
    case ID_EDIT_UNDO:
        canvas_cancel();
        if (gx_doc_undo(app.doc) == 1) {
            app.redo_steps++;
            app.undo_seen = (size_t)gx_doc_undo_len(app.doc);
            doc_changed();
        }
        break;
    case ID_EDIT_REDO:
        canvas_cancel();
        if (app.redo_steps > 0 && gx_doc_redo(app.doc) == 1) {
            app.redo_steps--;
            app.undo_seen = (size_t)gx_doc_undo_len(app.doc);
            doc_changed();
        }
        break;
    case ID_EDIT_COPY_FORMULA: pane_copy_formula(false); break;
    case ID_EDIT_COPY_LATEX: pane_copy_formula(true); break;
    case ID_EDIT_COPY_PICTURE: pane_copy_picture(); break;
    case ID_EDIT_COPY_DRAWING: canvas_copy_picture(); set_status(L"The drawing was copied."); break;
    case ID_EDIT_DELETE: delete_selection(); break;
    case ID_EDIT_SELECT_ALL: select_all(); break;
    case ID_EDIT_PROPERTIES: dlg_properties(g_frame); break;
    case ID_VIEW_TOOLBAR: show_toolbar = !show_toolbar; layout(); break;
    case ID_VIEW_CONBAR: show_conbar = !show_conbar; layout(); break;
    case ID_VIEW_TOOLBOX: show_toolbox = !show_toolbox; layout(); break;
    case ID_VIEW_STATUSBAR: show_status = !show_status; layout(); break;
    case ID_VIEW_PANE: show_pane = !show_pane; layout(); break;
    case ID_VIEW_ZOOM_IN: canvas_zoom(1.25); break;
    case ID_VIEW_ZOOM_OUT: canvas_zoom(0.8); break;
    case ID_VIEW_FIT: canvas_fit(); break;
    case ID_VIEW_GRID: app.show_grid = !app.show_grid; canvas_refresh(); break;
    case ID_VIEW_SYMBOLS: app.show_symbols = !app.show_symbols; canvas_refresh(); break;
    case ID_INSERT_FUNCTION: dlg_function(g_frame, NONE); break;
    case ID_INSERT_VARIABLE: dlg_variable(g_frame, NONE); break;
    case ID_FORMULA_LONGER: pane_search_longer(); break;
    case ID_HELP_TOPICS: dlg_help(g_frame); break;
    case ID_HELP_ABOUT: dlg_about(g_frame); break;
    }
    update_bars();
}

void save_settings() {
    WINDOWPLACEMENT wp = {sizeof wp};
    GetWindowPlacement(g_frame, &wp);
    reg_set(L"Left", wp.rcNormalPosition.left);
    reg_set(L"Top", wp.rcNormalPosition.top);
    reg_set(L"Right", wp.rcNormalPosition.right);
    reg_set(L"Bottom", wp.rcNormalPosition.bottom);
    reg_set(L"Maximized", wp.showCmd == SW_SHOWMAXIMIZED);
    reg_set(L"PaneWidth", pane_w);
    reg_set(L"Toolbar", show_toolbar);
    reg_set(L"ConstraintBar", show_conbar);
    reg_set(L"ToolBox", show_toolbox);
    reg_set(L"StatusBar", show_status);
    reg_set(L"Pane", show_pane);
    reg_set(L"Grid", app.show_grid);
    reg_set(L"Symbols", app.show_symbols);
}

void load_settings() {
    pane_w = (int)reg_get(L"PaneWidth", 330);
    show_toolbar = reg_get(L"Toolbar", 1) != 0;
    show_conbar = reg_get(L"ConstraintBar", 1) != 0;
    show_toolbox = reg_get(L"ToolBox", 1) != 0;
    show_status = reg_get(L"StatusBar", 1) != 0;
    show_pane = reg_get(L"Pane", 1) != 0;
    app.show_grid = reg_get(L"Grid", 1) != 0;
    app.show_symbols = reg_get(L"Symbols", 1) != 0;
}

bool in_splitter(int x) {
    if (!show_pane) return false;
    RECT rc;
    GetClientRect(g_frame, &rc);
    int px = rc.right - pane_w;
    return x >= px - SPLIT && x < px;
}

LRESULT CALLBACK frame_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CREATE:
        g_frame = h;
        make_bars();
        canvas = canvas_create(h);
        pane = pane_create(h);
        DragAcceptFiles(h, TRUE);
        return 0;
    case WM_SIZE:
        layout();
        return 0;
    case WM_SETFOCUS:
        SetFocus(canvas);
        return 0;
    case WM_COMMAND:
        if (LOWORD(wp) == IDC_CANVAS && lp == 0) {
            if (HIWORD(wp) == 1) show_zoom();
            else if (HIWORD(wp) == 2) update_coords();
            return 0;
        }
        if (LOWORD(wp) == IDC_ZOOM) {
            if (HIWORD(wp) == CBN_SELENDOK) {
                int i = (int)SendMessage(zoom_box, CB_GETCURSEL, 0, 0);
                wchar_t buf[32];
                SendMessageW(zoom_box, CB_GETLBTEXT, i, (LPARAM)buf);
                SetWindowTextW(zoom_box, buf);
                zoom_from_box();
            }
            return 0;
        }
        command(LOWORD(wp));
        return 0;
    case WM_NOTIFY: {
        NMHDR *n = (NMHDR *)lp;
        if (n->code == TTN_GETDISPINFOW) {
            NMTTDISPINFOW *di = (NMTTDISPINFOW *)lp;
            static wstr tip;
            wstr s = load_str((UINT)n->idFrom);
            size_t nl = s.find(L'\n');
            tip = nl == wstr::npos ? s : s.substr(nl + 1);
            di->lpszText = (LPWSTR)tip.c_str();
            return 0;
        }
        if (n->hwndFrom == rebar && n->code == RBN_HEIGHTCHANGE) {
            layout();
            return 0;
        }
        break;
    }
    case WM_INITMENUPOPUP: {
        HMENU m = (HMENU)wp;
        if (GetMenuItemCount(m) > 0 && (GetMenuState(m, ID_FILE_EXIT, MF_BYCOMMAND) != (UINT)-1)) init_file_menu(m);
        init_popup(m);
        return 0;
    }
    case WM_MENUSELECT: {
        UINT id = LOWORD(wp), flags = HIWORD(wp);
        if (flags == 0xFFFF && lp == 0) break;
        wstr s;
        if (!(flags & MF_POPUP) && !(flags & MF_SEPARATOR)) {
            s = load_str(id >= ID_FILE_MRU1 && id <= ID_FILE_MRU4 ? ID_FILE_MRU1 : id);
            size_t nl = s.find(L'\n');
            if (nl != wstr::npos) s = s.substr(0, nl);
        }
        SendMessage(statusbar, SB_SIMPLE, TRUE, 0);
        SendMessageW(statusbar, SB_SETTEXTW, SB_SIMPLEID, (LPARAM)s.c_str());
        return 0;
    }
    case WM_EXITMENULOOP:
        SendMessage(statusbar, SB_SIMPLE, FALSE, 0);
        return 0;
    case WM_SETCURSOR:
        if ((HWND)wp == h && LOWORD(lp) == HTCLIENT) {
            POINT pt;
            GetCursorPos(&pt);
            ScreenToClient(h, &pt);
            if (in_splitter(pt.x)) {
                SetCursor(LoadCursor(0, IDC_SIZEWE));
                return TRUE;
            }
        }
        break;
    case WM_LBUTTONDOWN:
        if (in_splitter(GET_X_LPARAM(lp))) {
            splitting = true;
            SetCapture(h);
        }
        return 0;
    case WM_MOUSEMOVE:
        if (splitting) {
            RECT rc;
            GetClientRect(h, &rc);
            pane_w = (int)dclamp(rc.right - GET_X_LPARAM(lp) - SPLIT / 2, 220, dmax(rc.right - 300, 220));
            layout();
        }
        return 0;
    case WM_LBUTTONUP:
        if (splitting) {
            splitting = false;
            ReleaseCapture();
        }
        return 0;
    case WM_DROPFILES: {
        wchar_t file[MAX_PATH];
        if (DragQueryFileW((HDROP)wp, 0, file, MAX_PATH) && ask_save()) open_path(file);
        DragFinish((HDROP)wp);
        return 0;
    }
    case WM_CLOSE:
        if (ask_save()) DestroyWindow(h);
        return 0;
    case WM_DESTROY:
        save_settings();
        pane_cancel_jobs();
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProc(h, msg, wp, lp);
}

bool edit_has_focus() {
    HWND f = GetFocus();
    if (!f) return false;
    wchar_t cls[32];
    GetClassNameW(f, cls, 32);
    return lstrcmpiW(cls, L"Edit") == 0;
}

} // namespace

// ------------------------------------------------------------ shared

void set_status(const wstr &text) {
    if (statusbar) status_part(0, text);
}

void doc_changed() {
    GxDoc *d = app.doc;
    app.prune_selection();
    // a new undo step means redo has nothing left to take back
    ptrdiff_t len = gx_doc_undo_len(d);
    if (len >= 0 && (size_t)len > app.undo_seen) app.redo_steps = 0;
    if (len >= 0) app.undo_seen = (size_t)len;
    canvas_refresh();
    pane_refresh();
    update_bars();
    update_status_info();
}

void sel_changed() {
    canvas_refresh();
    pane_refresh();
    update_bars();
}

void set_tool(Tool t) {
    app.tool = t;
    canvas_cancel();
    SendMessage(toolbox, TB_CHECKBUTTON, tool_cmd(t), TRUE);
    update_bars();
    set_status(canvas_prompt());
}

int WINAPI wWinMain(HINSTANCE inst, HINSTANCE, LPWSTR, int show) {
    g_inst = inst;
    Gdiplus::GdiplusStartupInput gsi;
    ULONG_PTR gtoken;
    Gdiplus::GdiplusStartup(&gtoken, &gsi, 0);
    INITCOMMONCONTROLSEX icc = {sizeof icc, ICC_BAR_CLASSES | ICC_COOL_CLASSES | ICC_LISTVIEW_CLASSES | ICC_TAB_CLASSES |
                                                ICC_PROGRESS_CLASS | ICC_WIN95_CLASSES | ICC_STANDARD_CLASSES};
    InitCommonControlsEx(&icc);
    OleInitialize(0);

    NONCLIENTMETRICSW ncm;
    ncm.cbSize = sizeof ncm;
    SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof ncm, &ncm, 0);
    g_ui_font = CreateFontIndirectW(&ncm.lfMessageFont);

    g_icons = ImageList_Create(16, 16, ILC_COLOR32, I_COUNT, 0);
    HBITMAP strip = (HBITMAP)LoadImageW(inst, MAKEINTRESOURCEW(IDB_TOOLBAR), IMAGE_BITMAP, 0, 0, LR_CREATEDIBSECTION);
    ImageList_Add(g_icons, strip, 0);
    DeleteObject(strip);

    app.doc = gx_doc_new();
    app.saved_gen = gx_doc_generation(app.doc);
    load_settings();
    load_mru();

    WNDCLASSEXW wc = {sizeof wc};
    wc.lpfnWndProc = frame_proc;
    wc.hInstance = inst;
    wc.hIcon = LoadIconW(inst, MAKEINTRESOURCEW(IDI_GXPAD));
    wc.hIconSm = (HICON)LoadImageW(inst, MAKEINTRESOURCEW(IDI_GXPAD), IMAGE_ICON, 16, 16, 0);
    wc.hCursor = LoadCursor(0, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
    wc.lpszMenuName = MAKEINTRESOURCEW(IDR_MENU);
    wc.lpszClassName = L"GxpadFrame";
    RegisterClassExW(&wc);
    WNDCLASSEXW tc = {sizeof tc};
    tc.lpfnWndProc = toolbox_proc;
    tc.hInstance = inst;
    tc.hCursor = LoadCursor(0, IDC_ARROW);
    tc.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
    tc.lpszClassName = L"GxpadToolbox";
    RegisterClassExW(&tc);

    int x = (int)reg_get(L"Left", CW_USEDEFAULT), y = (int)reg_get(L"Top", CW_USEDEFAULT);
    int r = (int)reg_get(L"Right", 0), b = (int)reg_get(L"Bottom", 0);
    int w = r > x && x != (int)CW_USEDEFAULT ? r - x : 1000, hh = b > y && y != (int)CW_USEDEFAULT ? b - y : 700;
    HWND frame = CreateWindowExW(0, L"GxpadFrame", L"gxpad", WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN, x, y, w, hh, 0, 0, inst, 0);
    accel = LoadAcceleratorsW(inst, MAKEINTRESOURCEW(IDR_ACCEL));
    update_title();
    ShowWindow(frame, reg_get(L"Maximized", 0) ? SW_SHOWMAXIMIZED : show);
    UpdateWindow(frame);
    doc_changed();
    set_status(load_str(IDS_READY));
    show_zoom();

    // a file named on the command line, and a dimension in it to select
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    if (argv && argc > 1) {
        open_path(argv[1]);
        if (argc > 2) {
            for (size_t i = 0; i < gx_doc_dim_count(app.doc); i++)
                if (geo::dim_name(app.doc, i) == argv[2]) {
                    app.select_only(Sel(Sel::DIM, i));
                    sel_changed();
                }
        }
    }
    if (argv) LocalFree(argv);

    MSG m;
    while (GetMessageW(&m, 0, 0, 0) > 0) {
        if (m.message == WM_KEYDOWN && m.hwnd == zoom_box && m.wParam == VK_RETURN) {
            zoom_from_box();
            continue;
        }
        HWND zoom_edit = GetWindow(zoom_box, GW_CHILD);
        if (m.message == WM_KEYDOWN && m.hwnd == zoom_edit && m.wParam == VK_RETURN) {
            zoom_from_box();
            continue;
        }
        if (!edit_has_focus() && TranslateAcceleratorW(frame, accel, &m)) continue;
        if (IsDialogMessageW(pane, &m)) continue;
        TranslateMessage(&m);
        DispatchMessageW(&m);
    }
    gx_doc_free(app.doc);
    Gdiplus::GdiplusShutdown(gtoken);
    OleUninitialize();
    return 0;
}
