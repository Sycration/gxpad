// The dialog boxes: properties of dimensions, variables, functions, points
// and shapes, About and Help.
#include "common.h"

namespace {

wstr item_text(HWND dlg, int id) { return window_text(GetDlgItem(dlg, id)); }
void set_item(HWND dlg, int id, const wstr &s) { SetDlgItemTextW(dlg, id, s.c_str()); }
bool checked(HWND dlg, int id) { return IsDlgButtonChecked(dlg, id) == BST_CHECKED; }
void check(HWND dlg, int id, bool on) { CheckDlgButton(dlg, id, on ? BST_CHECKED : BST_UNCHECKED); }

void error_box(HWND dlg, const wstr &what) {
    wstr e = last_error();
    MessageBoxW(dlg, (what + (e.empty() ? L"." : L":\n\n" + e)).c_str(), load_str(IDS_APP).c_str(), MB_ICONEXCLAMATION);
}

// Changes made by one dialog are one undo step.
struct Step {
    ptrdiff_t len;
    Step() : len(gx_doc_undo_len(app.doc)) {}
    void done() {
        if (len >= 0) gx_doc_join_undo(app.doc, (size_t)len);
    }
    // takes back what was done so far
    void undo() {
        while (len >= 0 && gx_doc_undo_len(app.doc) > len && gx_doc_undo(app.doc) == 1) {
        }
    }
};

bool rename(HWND dlg, const wstr &old, int id) {
    wstr nn = item_text(dlg, id);
    if (nn == old) return true;
    if (gx_doc_rename(app.doc, narrow(old).c_str(), narrow(nn).c_str()) == 1) return true;
    error_box(dlg, L"The name \"" + nn + L"\" cannot be used. A name is a letter followed by letters, digits or _, and no other "
                   L"point, dimension or variable may have it");
    SetFocus(GetDlgItem(dlg, id));
    SendDlgItemMessage(dlg, id, EM_SETSEL, 0, -1);
    return false;
}

void center_on_owner(HWND dlg) {
    HWND owner = GetWindow(dlg, GW_OWNER);
    RECT ro, rd;
    GetWindowRect(owner ? owner : GetDesktopWindow(), &ro);
    GetWindowRect(dlg, &rd);
    int w = rd.right - rd.left, h = rd.bottom - rd.top;
    SetWindowPos(dlg, 0, ro.left + (ro.right - ro.left - w) / 2, ro.top + (ro.bottom - ro.top - h) / 2, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

const wchar_t *dim_kind_text(int kind) {
    switch (kind) {
    case GX_DIM_DISTANCE: return L"Distance between two points";
    case GX_DIM_HDIST: return L"Horizontal distance between two points";
    case GX_DIM_VDIST: return L"Vertical distance between two points";
    case GX_DIM_POINT_LINE: return L"Distance from a point to a line";
    case GX_DIM_LENGTH: return L"Length of a line";
    case GX_DIM_RADIUS: return L"Radius of a circle";
    case GX_DIM_DIAMETER: return L"Diameter of a circle";
    case GX_DIM_ANGLE: return L"Angle between two lines";
    case GX_DIM_CONIC_A: return L"First semi-axis of a conic";
    case GX_DIM_CONIC_B: return L"Second semi-axis of a conic";
    case GX_DIM_X: return L"X coordinate of a point";
    case GX_DIM_Y: return L"Y coordinate of a point";
    }
    return L"Dimension";
}

// ------------------------------------------------------------ dimension

struct DimArgs { size_t i; bool fresh; };

void dim_enable(HWND dlg) {
    EnableWindow(GetDlgItem(dlg, IDC_VALUE), checked(dlg, IDC_DRIVING));
}

INT_PTR CALLBACK dim_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    static DimArgs *a;
    GxDoc *d = app.doc;
    switch (msg) {
    case WM_INITDIALOG: {
        a = (DimArgs *)lp;
        size_t i = a->i;
        int kind = gx_doc_dim_kind(d, i);
        bool angle = kind == GX_DIM_ANGLE;
        set_item(dlg, IDC_NAME, geo::dim_name(d, i));
        set_item(dlg, IDC_KIND_TEXT, dim_kind_text(kind));
        wstr expr = take(gx_doc_dim_expr(d, i));
        bool driving = gx_doc_dim_driving(d, i) == 1;
        wstr v = !expr.empty() ? expr : fmt_val(driving ? gx_doc_dim_set_value(d, i) : gx_doc_dim_value(d, i));
        if (angle && expr.empty()) v = fmt_val((driving ? gx_doc_dim_set_value(d, i) : gx_doc_dim_value(d, i)) * 180 / PI);
        set_item(dlg, IDC_VALUE, v);
        check(dlg, driving ? IDC_DRIVING : IDC_MEASURED, true);
        check(dlg, IDC_NUMERIC, gx_doc_dim_numeric(d, i) == 1);
        check(dlg, IDC_SUPP, gx_doc_dim_supp(d, i) == 1);
        ShowWindow(GetDlgItem(dlg, IDC_SUPP), angle ? SW_SHOW : SW_HIDE);
        set_item(dlg, IDC_HINT, angle ? L"Angles are in degrees. The value can also be an expression of variables and other "
                                        L"dimensions, such as 90 - a."
                                      : L"The value can be a number or an expression of variables and other dimensions, "
                                        L"such as 2*d1.");
        if (a->fresh) SetWindowTextW(dlg, L"New Dimension");
        dim_enable(dlg);
        center_on_owner(dlg);
        HWND focus = GetDlgItem(dlg, driving ? IDC_VALUE : IDC_NAME);
        SetFocus(focus);
        SendMessage(focus, EM_SETSEL, 0, -1);
        return FALSE;
    }
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDC_DRIVING:
        case IDC_MEASURED:
            dim_enable(dlg);
            return TRUE;
        case IDOK: {
            size_t i = a->i;
            Step step;
            if (!rename(dlg, geo::dim_name(d, i), IDC_NAME)) { step.undo(); return TRUE; }
            bool was = gx_doc_dim_driving(d, i) == 1;
            wstr expr = take(gx_doc_dim_expr(d, i));
            if (checked(dlg, IDC_DRIVING)) {
                wstr v = item_text(dlg, IDC_VALUE);
                bool changed = !was || v != (expr.empty() ? wstr() : expr);
                // an unchanged number is left alone: it may show fewer digits than it has
                if (changed && expr.empty() && was) {
                    int kind = gx_doc_dim_kind(d, i);
                    double cur = gx_doc_dim_set_value(d, i);
                    wstr shown = fmt_val(kind == GX_DIM_ANGLE ? cur * 180 / PI : cur);
                    changed = v != shown;
                }
                if (changed && gx_doc_set_dim(d, i, narrow(v).c_str()) != 1) {
                    error_box(dlg, L"The dimension cannot have the value \"" + v + L"\"");
                    step.undo();
                    SetFocus(GetDlgItem(dlg, IDC_VALUE));
                    SendDlgItemMessage(dlg, IDC_VALUE, EM_SETSEL, 0, -1);
                    return TRUE;
                }
            } else if (was && gx_doc_set_dim_driving(d, i, false) != 1) {
                error_box(dlg, L"The dimension cannot be made a measurement");
                step.undo();
                return TRUE;
            }
            bool num = checked(dlg, IDC_NUMERIC);
            if (num != (gx_doc_dim_numeric(d, i) == 1)) gx_doc_set_dim_numeric(d, i, num);
            if (gx_doc_dim_kind(d, i) == GX_DIM_ANGLE && checked(dlg, IDC_SUPP) != (gx_doc_dim_supp(d, i) == 1) &&
                gx_doc_toggle_supp(d, i) != 1) {
                error_box(dlg, L"The supplementary angle cannot be measured");
                step.undo();
                return TRUE;
            }
            step.done();
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dlg, IDCANCEL);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

// ------------------------------------------------------------ variable

void var_enable(HWND dlg) {
    bool def = checked(dlg, IDC_DEFINED);
    EnableWindow(GetDlgItem(dlg, IDC_VALUE), !def);
    EnableWindow(GetDlgItem(dlg, IDC_EXPR), def);
}

INT_PTR CALLBACK var_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    static size_t i;
    GxDoc *d = app.doc;
    switch (msg) {
    case WM_INITDIALOG: {
        i = (size_t)lp;
        if (i == NONE) {
            SetWindowTextW(dlg, L"New Variable");
            set_item(dlg, IDC_VALUE, L"1");
            check(dlg, IDC_FREE, true);
        } else {
            wstr expr = take(gx_doc_var_expr(d, i));
            set_item(dlg, IDC_NAME, geo::var_name(d, i));
            set_item(dlg, IDC_VALUE, fmt_val(gx_doc_var_value(d, i)));
            set_item(dlg, IDC_EXPR, expr);
            bool def = !expr.empty();
            check(dlg, def ? IDC_DEFINED : gx_doc_var_free(d, i) == 1 ? IDC_FREE : IDC_SOLVED, true);
            check(dlg, IDC_NUMERIC, gx_doc_var_numeric(d, i) == 1);
            // a variable keeps being a value or being defined
            EnableWindow(GetDlgItem(dlg, IDC_DEFINED), def);
            EnableWindow(GetDlgItem(dlg, IDC_FREE), !def);
            EnableWindow(GetDlgItem(dlg, IDC_SOLVED), !def);
        }
        var_enable(dlg);
        center_on_owner(dlg);
        SetFocus(GetDlgItem(dlg, IDC_NAME));
        return FALSE;
    }
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDC_FREE:
        case IDC_SOLVED:
        case IDC_DEFINED:
            var_enable(dlg);
            return TRUE;
        case IDOK: {
            wstr name = item_text(dlg, IDC_NAME);
            std::string n8 = narrow(name);
            bool def = checked(dlg, IDC_DEFINED);
            wstr vt = item_text(dlg, IDC_VALUE), et = item_text(dlg, IDC_EXPR);
            double v = _wtof(vt.c_str());
            if (!def && (vt.empty() || wcsspn(vt.c_str(), L"0123456789.-+eE ") != vt.size())) {
                MessageBoxW(dlg, L"Type a number for the value.", load_str(IDS_APP).c_str(), MB_ICONEXCLAMATION);
                SetFocus(GetDlgItem(dlg, IDC_VALUE));
                return TRUE;
            }
            Step step;
            size_t j = i;
            if (i == NONE) {
                ptrdiff_t r = def ? gx_doc_add_defvar(d, name.empty() ? 0 : n8.c_str(), narrow(et).c_str())
                                  : gx_doc_add_var(d, name.empty() ? 0 : n8.c_str(), v, checked(dlg, IDC_FREE));
                if (r < 0) {
                    error_box(dlg, L"The variable cannot be added");
                    return TRUE;
                }
                j = (size_t)r;
            } else {
                if (!rename(dlg, geo::var_name(d, i), IDC_NAME)) { step.undo(); return TRUE; }
                if (def) {
                    if (et != take(gx_doc_var_expr(d, i)) && gx_doc_set_var_expr(d, i, narrow(et).c_str()) != 1) {
                        error_box(dlg, L"The expression \"" + et + L"\" cannot be used");
                        step.undo();
                        return TRUE;
                    }
                } else {
                    bool fr = checked(dlg, IDC_FREE);
                    if (fr != (gx_doc_var_free(d, i) == 1) && gx_doc_set_var_free(d, i, fr) != 1) {
                        error_box(dlg, L"The variable cannot be changed");
                        step.undo();
                        return TRUE;
                    }
                    if (fr && vt != fmt_val(gx_doc_var_value(d, i)) && gx_doc_set_var(d, i, v) != 1) {
                        error_box(dlg, L"The variable cannot have that value");
                        step.undo();
                        return TRUE;
                    }
                }
            }
            bool num = checked(dlg, IDC_NUMERIC);
            if (num != (gx_doc_var_numeric(d, j) == 1)) gx_doc_set_var_numeric(d, j, num);
            step.done();
            app.select_only(Sel(Sel::VAR, j));
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dlg, IDCANCEL);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

// ------------------------------------------------------------ function

INT_PTR CALLBACK func_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    static size_t e;
    GxDoc *d = app.doc;
    switch (msg) {
    case WM_INITDIALOG:
        e = (size_t)lp;
        if (e == NONE) {
            SetWindowTextW(dlg, L"New Function");
            set_item(dlg, IDC_EQUATION, L"y = ");
        } else {
            set_item(dlg, IDC_EQUATION, take(gx_doc_curve_src(d, e)));
        }
        center_on_owner(dlg);
        SetFocus(GetDlgItem(dlg, IDC_EQUATION));
        SendDlgItemMessage(dlg, IDC_EQUATION, EM_SETSEL, e == NONE ? 4 : 0, -1);
        return FALSE;
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDOK: {
            wstr src = item_text(dlg, IDC_EQUATION);
            std::string s8 = narrow(src);
            if (gx_doc_check_curve(d, s8.c_str()) != 1) {
                error_box(dlg, L"The equation \"" + src + L"\" cannot be drawn");
                SetFocus(GetDlgItem(dlg, IDC_EQUATION));
                return TRUE;
            }
            if (e == NONE) {
                ptrdiff_t r = gx_doc_add_curve(d, s8.c_str());
                if (r < 0) {
                    error_box(dlg, L"The function cannot be added");
                    return TRUE;
                }
                app.select_only(Sel(Sel::ENT, (size_t)r));
            } else if (src != take(gx_doc_curve_src(d, e)) && gx_doc_set_curve(d, e, s8.c_str()) != 1) {
                error_box(dlg, L"The function cannot be changed to that equation");
                return TRUE;
            }
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dlg, IDCANCEL);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

// ------------------------------------------------------------ point and shape

INT_PTR CALLBACK point_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    static size_t e;
    GxDoc *d = app.doc;
    switch (msg) {
    case WM_INITDIALOG: {
        e = (size_t)lp;
        V2 p = geo::pos(d, e);
        set_item(dlg, IDC_NAME, geo::ent_name(d, e));
        set_item(dlg, IDC_X, fmt_val(p.x));
        set_item(dlg, IDC_Y, fmt_val(p.y));
        check(dlg, IDC_LOCKED, gx_doc_point_locked(d, e) == 1);
        if (geo::is_fixed(d, e)) {
            EnableWindow(GetDlgItem(dlg, IDC_LOCKED), FALSE);
            EnableWindow(GetDlgItem(dlg, IDC_NAME), FALSE);
        }
        center_on_owner(dlg);
        return TRUE;
    }
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDOK: {
            Step step;
            if (!geo::is_fixed(d, e)) {
                if (!rename(dlg, geo::ent_name(d, e), IDC_NAME)) { step.undo(); return TRUE; }
                bool lk = checked(dlg, IDC_LOCKED);
                if (lk != (gx_doc_point_locked(d, e) == 1) && gx_doc_lock_point(d, e, lk) != 1) {
                    error_box(dlg, L"The point cannot be locked");
                    step.undo();
                    return TRUE;
                }
            }
            step.done();
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dlg, IDCANCEL);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

const wchar_t *shape_name(GxDoc *d, size_t e) {
    switch (geo::kind(d, e)) {
    case GX_ENT_LINE: return geo::segment(d, e) ? L"Line segment" : L"Infinite line";
    case GX_ENT_CIRCLE: return L"Circle";
    case GX_ENT_ARC: return L"Arc";
    case GX_ENT_CONIC: {
        GxConic k;
        V2 c;
        double a, b, t;
        if (geo::conic(d, e, &k, &c, &a, &b, &t)) return k == GX_CONIC_ELLIPSE ? L"Ellipse" : k == GX_CONIC_HYPERBOLA ? L"Hyperbola" : L"Parabola";
        return L"Conic";
    }
    }
    return L"Shape";
}

INT_PTR CALLBACK shape_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    static size_t e;
    GxDoc *d = app.doc;
    switch (msg) {
    case WM_INITDIALOG: {
        e = (size_t)lp;
        wstr title = wstr(shape_name(d, e)) + L" Properties";
        SetWindowTextW(dlg, title.c_str());
        set_item(dlg, IDC_KIND_TEXT, shape_name(d, e));
        check(dlg, IDC_CONSTRUCTION, gx_doc_ent_construction(d, e) == 1);
        check(dlg, IDC_INFINITE, gx_doc_ent_infinite(d, e) == 1);
        bool line = geo::is_line(d, e);
        ShowWindow(GetDlgItem(dlg, IDC_INFINITE), line ? SW_SHOW : SW_HIDE);
        if (geo::is_fixed(d, e)) {
            EnableWindow(GetDlgItem(dlg, IDC_CONSTRUCTION), FALSE);
            EnableWindow(GetDlgItem(dlg, IDC_INFINITE), FALSE);
        }
        center_on_owner(dlg);
        return TRUE;
    }
    case WM_COMMAND:
        switch (LOWORD(wp)) {
        case IDOK: {
            Step step;
            bool c = checked(dlg, IDC_CONSTRUCTION);
            if (c != (gx_doc_ent_construction(d, e) == 1)) gx_doc_set_construction(d, e, c);
            bool inf = checked(dlg, IDC_INFINITE);
            if (geo::is_line(d, e) && inf != (gx_doc_ent_infinite(d, e) == 1) && gx_doc_set_infinite(d, e, inf) != 1) {
                error_box(dlg, L"The line cannot be changed");
                step.undo();
                return TRUE;
            }
            step.done();
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dlg, IDCANCEL);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

// ------------------------------------------------------------ about and help

INT_PTR CALLBACK about_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM) {
    switch (msg) {
    case WM_INITDIALOG:
        set_item(dlg, IDC_LIBVERSION, L"Geometry library gxgeo " + widen(gx_version()));
        SendDlgItemMessage(dlg, IDC_ABOUT_ICON, STM_SETICON, (WPARAM)LoadImageW(g_inst, MAKEINTRESOURCEW(IDI_GXPAD), IMAGE_ICON, 32, 32, 0), 0);
        center_on_owner(dlg);
        return TRUE;
    case WM_COMMAND:
        if (LOWORD(wp) == IDOK || LOWORD(wp) == IDCANCEL) {
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

struct Topic { const wchar_t *title, *text; };

const Topic TOPICS[] = {
    {L"Getting started",
     L"gxpad draws geometric constructions and finds formulas for what you measure.\r\n\r\n"
     L"1. Draw shapes with the tools in the tool box at the left, or from the Insert and Construct menus.\r\n"
     L"2. Select shapes and add constraints from the Constrain menu, such as Parallel or Tangent.\r\n"
     L"3. Add dimensions from the Dimension menu or with the Dimension tool, and type their values.\r\n"
     L"4. Select a dimension that measures something. The Formula box at the right shows a formula for it in terms of "
     L"the dimensions you set.\r\n\r\n"
     L"For example, draw a triangle with the Line tool, give its three sides lengths, and measure one of its angles: "
     L"gxpad shows the law of cosines."},
    {L"Drawing shapes",
     L"Pick a tool in the tool box, then click in the drawing. The status bar at the bottom says what to click next.\r\n\r\n"
     L"Lines and circles can also be drawn by dragging, as in Paint. A click on an existing point uses that point; a "
     L"click on a line or curve puts the new point on it. A green square marks the point a click would use.\r\n\r\n"
     L"Press Esc or click the right mouse button to start over, and once more to go back to the Select tool."},
    {L"Selecting and moving",
     L"With the Select tool, click a shape to select it. Hold down Shift or Ctrl to select more than one. Drag across "
     L"empty paper to select everything inside a rectangle.\r\n\r\n"
     L"Drag a point or shape to move it. gxpad keeps the constraints while it moves.\r\n\r\n"
     L"Right-click a shape for the commands that apply to it. Double-click it, or press Alt+Enter, to see its "
     L"properties."},
    {L"Constraints",
     L"A constraint keeps shapes in a relation: two lines parallel, a point on a circle, a line tangent to a curve. "
     L"Select the shapes, then choose a command on the Constrain menu. Commands that do not apply to the selection are "
     L"grayed out.\r\n\r\n"
     L"Small symbols next to the shapes show the constraints. Click a symbol to select the constraint, and press Delete "
     L"to remove it. View > Constraint Symbols hides them.\r\n\r\n"
     L"Shapes that are free to move are blue. Shapes that the constraints and dimensions fix completely are black. "
     L"Red shapes have constraints that cannot all be met."},
    {L"Dimensions",
     L"A dimension measures a length, distance, angle, radius or coordinate.\r\n\r\n"
     L"A driving dimension sets the value: type a number, or an expression of variables and other dimensions, such as "
     L"2*d1. A measured dimension (shown in parentheses) only reports the value.\r\n\r\n"
     L"Select shapes and choose a command on the Dimension menu, or use the Dimension tool: click one or two shapes, then "
     L"click where the label goes. Drag a label to move it. Double-click it to change the value."},
    {L"Formulas",
     L"Select a measured dimension, or a variable, to see its formula in the Formula box. The formula is in terms of the "
     L"driving dimensions and variables, so it holds for any values you give them.\r\n\r\n"
     L"Finding a formula can take a few seconds; a moving bar shows that gxpad is still looking. When the formula is "
     L"approximate or long, Search Longer looks for a better one with ten more seconds each time.\r\n\r\n"
     L"Copy puts the formula on the Clipboard as text, Copy LaTeX as LaTeX for documents. File > Export Formula Picture "
     L"saves it as a PNG picture. Mark a dimension \"Use as a number in formulas\" to have its value appear in other "
     L"formulas instead of its name."},
    {L"Variables and functions",
     L"Variables are names for values that dimensions and functions can use. A free variable has the value you give it. "
     L"A solved variable is found from the constraints. A defined variable is given by an expression.\r\n\r\n"
     L"Functions are curves such as y = sin(x) or x^2 + y^2 = 4. They can use variables and dimensions, and points can "
     L"be constrained onto them. Add both from the Insert menu or with the Add button in the pane at the right."},
    {L"Keyboard shortcuts",
     L"Ctrl+N\tNew\r\nCtrl+O\tOpen\r\nCtrl+S\tSave\r\nCtrl+P\tPrint\r\nCtrl+Z\tUndo\r\nCtrl+Y\tRedo\r\n"
     L"Ctrl+C\tCopy the formula\r\nCtrl+A\tSelect all\r\nDelete\tDelete the selection\r\nAlt+Enter\tProperties\r\n"
     L"Ctrl++\tZoom in\r\nCtrl+-\tZoom out\r\nHome\tFit the drawing in the window\r\nF5\tSearch longer for a formula\r\n"
     L"Esc\tStart the current tool over\r\n\r\nCtrl and the mouse wheel zoom; the wheel alone scrolls. Drag with the "
     L"middle or right mouse button to move the view."},
};

INT_PTR CALLBACK help_proc(HWND dlg, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CTLCOLORSTATIC:
        // the topic reads like a page of help: black on white
        if ((HWND)lp == GetDlgItem(dlg, IDC_TOPIC_TEXT)) {
            SetBkColor((HDC)wp, GetSysColor(COLOR_WINDOW));
            SetTextColor((HDC)wp, GetSysColor(COLOR_WINDOWTEXT));
            return (INT_PTR)GetSysColorBrush(COLOR_WINDOW);
        }
        break;
    case WM_INITDIALOG: {
        HWND lb = GetDlgItem(dlg, IDC_TOPICS);
        for (size_t k = 0; k < sizeof TOPICS / sizeof TOPICS[0]; k++) SendMessageW(lb, LB_ADDSTRING, 0, (LPARAM)TOPICS[k].title);
        SendMessage(lb, LB_SETCURSEL, 0, 0);
        set_item(dlg, IDC_TOPIC_TEXT, TOPICS[0].text);
        HICON ic = (HICON)LoadImageW(g_inst, MAKEINTRESOURCEW(IDI_GXPAD), IMAGE_ICON, 16, 16, 0);
        SendMessage(dlg, WM_SETICON, ICON_SMALL, (LPARAM)ic);
        center_on_owner(dlg);
        return TRUE;
    }
    case WM_COMMAND:
        if (LOWORD(wp) == IDC_TOPICS && HIWORD(wp) == LBN_SELCHANGE) {
            int k = (int)SendDlgItemMessage(dlg, IDC_TOPICS, LB_GETCURSEL, 0, 0);
            if (k >= 0) set_item(dlg, IDC_TOPIC_TEXT, TOPICS[k].text);
            return TRUE;
        }
        if (LOWORD(wp) == IDOK || LOWORD(wp) == IDCANCEL) {
            EndDialog(dlg, IDOK);
            return TRUE;
        }
        break;
    }
    return FALSE;
}

bool run(int id, DLGPROC proc, LPARAM arg, HWND owner) {
    bool ok = DialogBoxParamW(g_inst, MAKEINTRESOURCEW(id), owner, proc, arg) == IDOK;
    if (ok) doc_changed();
    return ok;
}

} // namespace

bool dlg_dimension(HWND owner, size_t i, bool fresh) {
    DimArgs a = {i, fresh};
    return run(IDD_DIMENSION, dim_proc, (LPARAM)&a, owner);
}

bool dlg_variable(HWND owner, size_t i) {
    bool ok = run(IDD_VARIABLE, var_proc, (LPARAM)i, owner);
    if (ok) sel_changed();
    return ok;
}

bool dlg_function(HWND owner, size_t e) {
    bool ok = run(IDD_FUNCTION, func_proc, (LPARAM)e, owner);
    if (ok) sel_changed();
    return ok;
}

bool dlg_point(HWND owner, size_t e) { return run(IDD_POINT, point_proc, (LPARAM)e, owner); }
bool dlg_shape(HWND owner, size_t e) { return run(IDD_SHAPE, shape_proc, (LPARAM)e, owner); }
void dlg_about(HWND owner) { DialogBoxParamW(g_inst, MAKEINTRESOURCEW(IDD_ABOUT), owner, about_proc, 0); }
void dlg_help(HWND owner) { DialogBoxParamW(g_inst, MAKEINTRESOURCEW(IDD_HELP), owner, help_proc, 0); }

bool dlg_properties(HWND owner) {
    GxDoc *d = app.doc;
    if (app.sel.size() != 1) {
        MessageBeep(MB_OK);
        return false;
    }
    Sel s = app.sel[0];
    switch (s.kind) {
    case Sel::DIM: return dlg_dimension(owner, s.i, false);
    case Sel::VAR: return dlg_variable(owner, s.i);
    case Sel::ENT:
        if (geo::is_point(d, s.i)) return dlg_point(owner, s.i);
        if (geo::is_function(d, s.i)) return dlg_function(owner, s.i);
        return dlg_shape(owner, s.i);
    case Sel::CON: {
        wstr e = take(gx_doc_con_error(d, s.i));
        wstr msg = gx_doc_con_conflict(d, s.i) == 1 ? L"This constraint cannot be met together with the others."
                                                    : L"This constraint is met.";
        if (!e.empty()) msg += L"\n\n" + e;
        msg += L"\n\nTo remove it, select it and press Delete.";
        MessageBoxW(owner, msg.c_str(), L"Constraint", MB_ICONINFORMATION);
        return false;
    }
    }
    return false;
}
