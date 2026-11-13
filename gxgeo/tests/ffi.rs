//! The C interface, called from Rust.

use gxgeo::ffi::*;
use std::ffi::{CStr, CString, c_int};

fn text(s: *const std::ffi::c_char) -> String {
    unsafe { CStr::from_ptr(s) }.to_str().unwrap().to_string()
}

#[test]
fn derive_through_c_interface() {
    let src = CString::new(include_str!("law_of_cosines.gxp")).unwrap();
    unsafe {
        let doc = gx_doc_from_gxp(src.as_ptr());
        assert!(!doc.is_null());
        let mut dof = usize::MAX;
        assert_eq!(gx_doc_solve(doc, &mut dof), 1);
        assert_eq!(dof, 0);
        assert_eq!(gx_doc_dim_count(doc), 4);
        let name = gx_doc_dim_name(doc, 3);
        assert_eq!(text(name), "θ1");
        gx_string_free(name);
        let f = gx_derive(doc, GX_TARGET_DIM, 3, 0, std::ptr::null());
        assert!(!f.is_null());
        assert_eq!(text((*f).plain), "θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))");
        assert!((*f).exact && (*f).is_angle);
        assert!(((*f).value - gx_doc_dim_value(doc, 3)).abs() < 1e-9);
        gx_formula_free(f);

        // round trip through .gxp text
        let out = gx_doc_to_gxp(doc);
        let again = gx_doc_from_gxp(out);
        gx_string_free(out);
        assert_eq!(gx_doc_dim_count(again), 4);
        gx_doc_free(again);
        gx_doc_free(doc);
    }
}

#[test]
fn errors_are_reported() {
    unsafe {
        let bad = CString::new("not gxp").unwrap();
        assert!(gx_doc_from_gxp(bad.as_ptr()).is_null());
        assert!(!gx_last_error().is_null());
        assert!(gx_doc_from_gxp(std::ptr::null()).is_null());
        assert_eq!(text(gx_last_error()), "null string");

        let doc = gx_doc_new();
        assert!(gx_doc_dim_name(doc, 0).is_null());
        assert!(gx_doc_dim_value(doc, 0).is_nan());
        assert!(gx_derive(doc, GX_TARGET_DIM, 0, 0, std::ptr::null()).is_null());
        assert_eq!(text(gx_last_error()), "gone");
        assert!(gx_derive(doc, 7, 0, 0, std::ptr::null()).is_null());
        assert_eq!(text(gx_last_error()), "bad target kind 7");
        gx_doc_free(doc);

        assert_eq!(gx_doc_solve(std::ptr::null_mut(), std::ptr::null_mut()), -1);
        gx_doc_free(std::ptr::null_mut());
        gx_formula_free(std::ptr::null_mut());
        gx_string_free(std::ptr::null_mut());
    }
}

#[test]
fn build_law_of_cosines_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let o = gx_doc_origin(doc) as usize;
        let b = gx_doc_add_point(doc, 4.0, 0.2) as usize;
        let c = gx_doc_add_point(doc, 1.0, 3.0) as usize;
        let ab = gx_doc_add_line(doc, o, b, false) as usize;
        let bc = gx_doc_add_line(doc, b, c, false) as usize;
        let ca = gx_doc_add_line(doc, c, o, false) as usize;
        assert_eq!(gx_doc_constrain(doc, GX_CON_HORIZONTAL, ab, 0), 1);
        for (side, len) in [(ab, c"5"), (bc, c"4"), (ca, c"3")] {
            let d = gx_doc_add_dim(doc, GX_DIM_LENGTH, side, 0, 0.3, 0.3);
            assert!(d >= 0);
            assert_eq!(gx_doc_dim_driving(doc, d as usize), 1);
            assert_eq!(gx_doc_set_dim(doc, d as usize, len.as_ptr()), 1);
        }
        let ang = gx_doc_add_dim(doc, GX_DIM_ANGLE, ab, ca, 0.3, 0.3) as usize;
        assert_eq!(gx_doc_set_dim_driving(doc, ang, false), 1);
        assert_eq!(gx_doc_solve(doc, std::ptr::null_mut()), 1);

        // the same document as the one the test wrote
        let built: serde_json::Value = serde_json::from_str(&text(gx_doc_to_gxp(doc))).unwrap();
        let file: serde_json::Value = serde_json::from_str(include_str!("law_of_cosines.gxp")).unwrap();
        fn same(a: &serde_json::Value, b: &serde_json::Value) -> bool {
            match (a, b) {
                (serde_json::Value::Number(x), serde_json::Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-9,
                (serde_json::Value::Array(x), serde_json::Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q)),
                (serde_json::Value::Object(x), serde_json::Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
                _ => a == b,
            }
        }
        assert!(same(&built, &file));

        let f = gx_derive(doc, GX_TARGET_DIM, ang, 0, std::ptr::null());
        assert_eq!(text((*f).plain), "θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))");
        gx_formula_free(f);

        // a fourth length is determined already: added as a reference dimension
        let extra = gx_doc_add_dim(doc, GX_DIM_DISTANCE, b, c, 0.3, 0.3) as usize;
        assert_eq!(gx_doc_dim_driving(doc, extra), 0);
        assert_eq!(gx_doc_set_dim_driving(doc, extra, true), 0);
        // a redundant constraint is rejected
        assert_eq!(gx_doc_constrain(doc, GX_CON_HORIZONTAL, ab, 0), 0);

        assert_eq!(gx_doc_rename(doc, c"d1".as_ptr(), c"base".as_ptr()), 1);
        assert_eq!(text(gx_doc_dim_name(doc, 0)), "base");
        assert_eq!(gx_doc_undo(doc), 1);
        assert_eq!(text(gx_doc_dim_name(doc, 0)), "d1");
        gx_doc_free(doc);
    }
}

#[test]
fn construction_errors_are_reported() {
    unsafe {
        let doc = gx_doc_new();
        let o = gx_doc_origin(doc) as usize;
        let x = gx_doc_x_axis(doc) as usize;
        assert_eq!(gx_doc_add_line(doc, o, 999, false), -1);
        assert_eq!(text(gx_last_error()), "no entity 999");
        assert_eq!(gx_doc_add_line(doc, o, x, false), -1);
        assert_eq!(text(gx_last_error()), format!("entity {x} is not a point"));
        assert_eq!(gx_doc_add_circle(doc, o, -1.0), -1);
        assert_eq!(gx_doc_add_point(doc, f64::NAN, 0.0), -1);
        assert_eq!(gx_doc_constrain(doc, 99, o, o), -1);
        assert_eq!(text(gx_last_error()), "unknown constraint kind 99");
        assert_eq!(gx_doc_constrain(doc, GX_CON_PARALLEL, o, x), -1);
        assert_eq!(gx_doc_add_dim(doc, GX_DIM_RADIUS, x, 0, 0.0, 0.0), -1);
        assert_eq!(gx_doc_add_dim(doc, 99, o, 0, 0.0, 0.0), -1);
        assert_eq!(gx_doc_set_dim(doc, 5, c"1".as_ptr()), -1);
        assert_eq!(text(gx_last_error()), "no dimension 5");
        assert_eq!(gx_doc_rename(doc, c"nobody".as_ptr(), c"x2".as_ptr()), -1);
        assert_eq!(gx_doc_redo(doc), 0);
        assert_eq!(gx_doc_add_point(std::ptr::null_mut(), 0.0, 0.0), -1);
        assert_eq!(text(gx_last_error()), "null document");
        gx_doc_free(doc);
    }
}

#[test]
fn edit_variables() {
    unsafe {
        let doc = gx_doc_new();
        let a = gx_doc_add_var(doc, std::ptr::null(), 2.0, true);
        assert_eq!(a, 0);
        let n = gx_doc_var_name(doc, 0);
        assert_eq!(text(n), "a");
        gx_string_free(n);
        assert_eq!(gx_doc_var_value(doc, 0), 2.0);
        assert_eq!(gx_doc_var_free(doc, 0), 1);
        assert_eq!(gx_doc_set_var(doc, 0, 5.0), 1);
        assert_eq!(gx_doc_var_value(doc, 0), 5.0);
        assert_eq!(gx_doc_set_var_free(doc, 0, false), 1);
        assert_eq!(gx_doc_var_free(doc, 0), 0);

        let f = gx_doc_add_defvar(doc, std::ptr::null(), c"2a".as_ptr());
        assert_eq!(f, 1);
        let n = gx_doc_var_name(doc, 1);
        assert_eq!(text(n), "f1");
        gx_string_free(n);
        let e = gx_doc_var_expr(doc, 1);
        assert_eq!(text(e), "2a");
        gx_string_free(e);
        assert_eq!(gx_doc_set_var_free(doc, 1, true), -1);
        assert_eq!(gx_doc_set_var(doc, 1, 1.0), -1);
        assert_eq!(gx_doc_set_var_expr(doc, 1, c"a+1".as_ptr()), 1);
        assert_eq!(gx_doc_set_var_expr(doc, 1, c"zz+".as_ptr()), -1);

        // a plain number turns a defined variable into a free value variable
        assert_eq!(gx_doc_set_var_expr(doc, 1, c"7".as_ptr()), 1);
        assert_eq!(gx_doc_var_value(doc, 1), 7.0);
        assert_eq!(gx_doc_var_free(doc, 1), 1);
        let e = gx_doc_var_expr(doc, 1);
        assert_eq!(text(e), "");
        gx_string_free(e);

        assert_eq!(gx_doc_add_var(doc, c"a".as_ptr(), 1.0, true), -1);
        assert_eq!(gx_doc_add_var(doc, c"1x".as_ptr(), 1.0, true), -1);
        assert_eq!(gx_doc_add_defvar(doc, c"g".as_ptr(), c"nope(".as_ptr()), -1);

        assert_eq!(gx_doc_delete_var(doc, 0), 1);
        assert!(gx_doc_var_name(doc, 0).is_null());
        assert!(gx_doc_var_expr(doc, 0).is_null());
        assert!(gx_doc_var_value(doc, 0).is_nan());
        assert_eq!(gx_doc_var_free(doc, 0), -1);
        assert_eq!(gx_doc_delete_var(doc, 0), -1);
        assert_eq!(gx_doc_delete_var(doc, 9), -1);
        gx_doc_free(doc);
    }
}

/// Value of a new dimension of kind `kind` on a and b.
unsafe fn measure(doc: *mut GxDoc, kind: GxDim, a: usize, b: usize) -> f64 {
    let d = unsafe { gx_doc_add_dim(doc, kind, a, b, 0.3, 0.3) };
    assert!(d >= 0, "{}", text(gx_last_error()));
    unsafe { gx_doc_dim_value(doc, d as usize) }
}

#[test]
fn constructions_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let (o, xa, ya) = (gx_doc_origin(doc) as usize, gx_doc_x_axis(doc) as usize, gx_doc_y_axis(doc) as usize);

        // midpoint of two points
        let p = gx_doc_add_point(doc, 4.0, 2.0) as usize;
        let m = gx_doc_add_midpoint(doc, o, p);
        assert!(m >= 0);
        assert!((measure(doc, GX_DIM_DISTANCE, o, m as usize) - 5f64.sqrt()).abs() < 1e-9);
        assert_eq!(gx_doc_add_midpoint(doc, o, o), -1);

        // intersection of a circle and an axis, on the side of the hint
        let c = gx_doc_add_circle(doc, o, 1.2) as usize;
        let x = gx_doc_add_intersection_near(doc, c, xa, 0.9, 0.1);
        assert!(x >= 0);
        assert!((measure(doc, GX_DIM_HDIST, o, x as usize) - 1.2).abs() < 1e-6);
        let y = gx_doc_add_intersection(doc, ya, xa, 0.1, 0.1);
        assert!(y >= 0);
        assert_eq!(gx_doc_add_intersection_near(doc, xa, xa, 0.0, 0.0), -1);

        // perpendicular line through a point
        let q = gx_doc_add_point(doc, 1.0, 3.0) as usize;
        let l = gx_doc_add_line(doc, o, p, false) as usize;
        let pl = gx_doc_add_perp_line(doc, l, q);
        assert!(pl >= 0);
        assert!((measure(doc, GX_DIM_ANGLE, l, pl as usize).abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
        let mut mid = usize::MAX;
        let pb = gx_doc_add_perp_bisector(doc, o, q, &mut mid);
        assert!(pb >= 0 && mid != usize::MAX);

        // circle through three points
        let (a, b, e) = (gx_doc_add_point(doc, 5.0, 0.0), gx_doc_add_point(doc, 0.0, 5.0), gx_doc_add_point(doc, -5.0, 0.0));
        let mut ctr = usize::MAX;
        let ci = gx_doc_add_circle_through(doc, a as usize, b as usize, e as usize, &mut ctr);
        assert!(ci >= 0 && ctr != usize::MAX);
        assert!((measure(doc, GX_DIM_RADIUS, ci as usize, 0) - 5.0).abs() < 1e-6);
        let k = gx_doc_add_point(doc, 9.0, 0.0) as usize;
        assert_eq!(gx_doc_add_circle_through(doc, o, xa, k, std::ptr::null_mut()), -1);

        // ellipse by foci, its foci and a hyperbola's asymptotes
        let (f1, f2, pe) = (gx_doc_add_point(doc, -3.0, 10.0), gx_doc_add_point(doc, 3.0, 10.0), gx_doc_add_point(doc, 0.0, 14.0));
        let el = gx_doc_add_conic_by_foci(doc, GX_CONIC_ELLIPSE, f1 as usize, f2 as usize, pe as usize);
        assert!(el >= 0);
        assert!((measure(doc, GX_DIM_CONIC_A, el as usize, 0) - 5.0).abs() < 1e-6);
        let mut foci = [0usize; 2];
        assert_eq!(gx_doc_add_foci(doc, el as usize, foci.as_mut_ptr(), 2), 2);
        let hc = gx_doc_add_point(doc, 0.0, -10.0) as usize;
        let hy = gx_doc_add_conic(doc, GX_CONIC_HYPERBOLA, hc, 2.0, 1.0, 0.0);
        assert!(hy >= 0);
        let mut asy = [0usize; 2];
        assert_eq!(gx_doc_add_asymptotes(doc, hy as usize, asy.as_mut_ptr(), 2), 2);
        assert_eq!(gx_doc_add_asymptotes(doc, el as usize, asy.as_mut_ptr(), 2), -1);

        // curves
        let cv = gx_doc_add_curve(doc, c"y = x^2".as_ptr());
        assert!(cv >= 0);
        assert_eq!(gx_doc_add_curve(doc, c"y = (".as_ptr()), -1);
        assert_eq!(gx_doc_set_curve(doc, cv as usize, c"y = 2x".as_ptr()), 1);
        assert_eq!(gx_doc_set_curve(doc, c as usize, c"y = 2x".as_ptr()), -1);
        let pc = gx_doc_add_point_on(doc, cv as usize, 1.0, 2.0);
        assert!(pc >= 0);
        assert_eq!(gx_doc_undo(doc), 1);
        gx_doc_free(doc);
    }
}

#[test]
fn read_edit_and_delete_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let o = gx_doc_origin(doc) as usize;
        let b = gx_doc_add_point(doc, 4.0, 0.2) as usize;
        let c = gx_doc_add_point(doc, 1.0, 3.0) as usize;
        let ab = gx_doc_add_line(doc, o, b, false) as usize;
        let circ = gx_doc_add_circle(doc, o, 2.0) as usize;

        // entity kinds and count
        assert!(gx_doc_ent_count(doc) >= 7);
        assert_eq!(gx_doc_ent_kind(doc, o), GX_ENT_POINT as c_int);
        assert_eq!(gx_doc_ent_kind(doc, ab), GX_ENT_LINE as c_int);
        assert_eq!(gx_doc_ent_kind(doc, circ), GX_ENT_CIRCLE as c_int);
        assert_eq!(gx_doc_ent_kind(doc, 9999), -1);
        assert_eq!(text(gx_last_error()), "no entity 9999");
        let n = gx_doc_ent_name(doc, b);
        assert_eq!(text(n), "A");
        gx_string_free(n);

        // point coordinates
        let (mut x, mut y) = (0.0, 0.0);
        assert_eq!(gx_doc_point_pos(doc, b, &mut x, &mut y), 1);
        assert!((x - 4.0).abs() < 1e-9 && (y - 0.2).abs() < 1e-9);
        assert_eq!(gx_doc_point_pos(doc, ab, &mut x, &mut y), -1);
        assert_eq!(gx_doc_point_locked(doc, o), 1);
        assert_eq!(gx_doc_point_locked(doc, b), 0);

        // line endpoints
        let (mut a, mut b2, mut inf) = (usize::MAX, usize::MAX, true);
        assert_eq!(gx_doc_line_pts(doc, ab, &mut a, &mut b2, &mut inf), 1);
        assert_eq!((a, b2), (o, b));
        assert!(!inf);
        assert_eq!(gx_doc_ent_infinite(doc, ab), 0);

        // circle center and radius
        let (mut cc, mut r) = (usize::MAX, 0.0);
        assert_eq!(gx_doc_circle_vals(doc, circ, &mut cc, &mut r), 1);
        assert_eq!(cc, o);
        assert!((r - 2.0).abs() < 1e-9);

        // constraints
        assert_eq!(gx_doc_constrain(doc, GX_CON_HORIZONTAL, ab, 0), 1);
        let con = gx_doc_con_count(doc) - 1;
        assert_eq!(gx_doc_con_kind(doc, con), GX_CON_HORIZONTAL as c_int);
        let mut ce = [usize::MAX; 4];
        assert_eq!(gx_doc_con_ents(doc, con, ce.as_mut_ptr(), 4), 2);
        assert_eq!(ce[0], o);

        // dimension kind
        let d = gx_doc_add_dim(doc, GX_DIM_LENGTH, ab, 0, 0.3, 0.3) as usize;
        assert_eq!(gx_doc_dim_kind(doc, d), GX_DIM_LENGTH as c_int);
        assert_eq!(gx_doc_dim_kind(doc, 9999), -1);

        // construction and infinite flags
        assert_eq!(gx_doc_ent_construction(doc, ab), 0);
        assert_eq!(gx_doc_set_construction(doc, ab, true), 1);
        assert_eq!(gx_doc_ent_construction(doc, ab), 1);
        assert_eq!(gx_doc_set_construction(doc, b, true), -1);
        assert_eq!(gx_doc_set_infinite(doc, ab, true), 1);
        assert_eq!(gx_doc_ent_infinite(doc, ab), 1);

        // polylines of the circle: sizes first, then the points
        let win = GxWindow { x0: -10.0, y0: -10.0, x1: 10.0, y1: 10.0, px: 0.02 };
        let mut npl = usize::MAX;
        let np = gx_doc_ent_samples(doc, circ, win, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, &mut npl);
        assert!(np > 0);
        assert_eq!(npl, 1);
        let mut pts = vec![0.0; 2 * np as usize];
        let mut starts = [usize::MAX; 1];
        assert_eq!(gx_doc_ent_samples(doc, circ, win, pts.as_mut_ptr(), np as usize, starts.as_mut_ptr(), 1, &mut npl), np);
        assert_eq!(starts[0], 0);
        assert!(pts.chunks(2).all(|p| (p[0].hypot(p[1]) - 2.0).abs() < 1e-9));
        // a buffer too small takes what fits and the total still comes back
        let mut few = [0.0; 4];
        assert_eq!(gx_doc_ent_samples(doc, circ, win, few.as_mut_ptr(), 2, std::ptr::null_mut(), 0, std::ptr::null_mut()), np);
        // a point is one point, a segment its two endpoints
        let mut one = [0.0; 2];
        assert_eq!(gx_doc_ent_samples(doc, b, win, one.as_mut_ptr(), 1, std::ptr::null_mut(), 0, std::ptr::null_mut()), 1);
        assert!((one[0] - 4.0).abs() < 1e-9);
        assert_eq!(gx_doc_ent_samples(doc, ab, win, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, std::ptr::null_mut()), 2);
        let bad = GxWindow { x0: 1.0, y0: 0.0, x1: 0.0, y1: 1.0, px: 0.1 };
        assert_eq!(gx_doc_ent_samples(doc, circ, bad, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, std::ptr::null_mut()), -1);

        // deleting a dimension, then an entity (cascades the line and its
        // horizontal constraint), then the constraint is already gone
        assert_eq!(gx_doc_delete_dim(doc, d), 1);
        assert_eq!(gx_doc_dim_kind(doc, d), -1);
        assert_eq!(gx_doc_delete_dim(doc, d), -1);
        assert_eq!(gx_doc_delete_ent(doc, b), 1);
        assert_eq!(gx_doc_ent_kind(doc, b), -1);
        assert_eq!(gx_doc_con_kind(doc, con), -1);
        assert_eq!(gx_doc_delete_ent(doc, b), -1);

        // locking and dragging a free point
        assert_eq!(gx_doc_lock_point(doc, c, true), 1);
        assert_eq!(gx_doc_point_locked(doc, c), 1);
        assert_eq!(gx_doc_lock_point(doc, c, false), 1);
        assert_eq!(gx_doc_point_locked(doc, c), 0);
        let q = gx_doc_add_point(doc, 0.0, 0.0) as usize;
        assert_eq!(gx_doc_drag_point(doc, q, 3.0, 4.0), 1);
        let (mut qx, mut qy) = (0.0, 0.0);
        gx_doc_point_pos(doc, q, &mut qx, &mut qy);
        assert!((qx - 3.0).abs() < 1e-6 && (qy - 4.0).abs() < 1e-6);
        assert_eq!(gx_doc_drag_point(doc, ab, 0.0, 0.0), -1);

        gx_doc_free(doc);
    }
}

#[test]
fn drag_circle_and_conic_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let o = gx_doc_origin(doc) as usize;
        let circ = gx_doc_add_circle(doc, o, 1.0) as usize;
        // drag the circle edge out to radius 3
        assert_eq!(gx_doc_drag_circle(doc, circ, 3.0, 0.0), 1);
        let (mut cc, mut r) = (usize::MAX, 0.0);
        gx_doc_circle_vals(doc, circ, &mut cc, &mut r);
        assert!((r - 3.0).abs() < 1e-6);

        // a free ellipse: drag its first handle to (4, 3) -> a = 5, angle = atan2(3,4)
        let el = gx_doc_add_conic(doc, GX_CONIC_ELLIPSE, o, 1.0, 1.0, 0.0) as usize;
        assert_eq!(gx_doc_drag_conic_handle(doc, el, 0, 4.0, 3.0), 1);
        let (mut kind, mut c, mut a, mut b, mut t) = (GX_CONIC_ELLIPSE, usize::MAX, 0.0, 0.0, 0.0);
        gx_doc_conic_vals(doc, el, &mut kind, &mut c, &mut a, &mut b, &mut t);
        assert!((a - 5.0).abs() < 1e-6, "a = {a}");
        assert!((t - 3f64.atan2(4.0)).abs() < 1e-6, "t = {t}");
        // drag the second handle: b becomes the perpendicular distance in the
        // rotated frame, |(-dx·sin t + dy·cos t)| = 2·cos(atan2(3,4)) = 1.6
        assert_eq!(gx_doc_drag_conic_handle(doc, el, 1, 0.0, 2.0), 1);
        gx_doc_conic_vals(doc, el, &mut kind, &mut c, &mut a, &mut b, &mut t);
        assert!((b - 1.6).abs() < 1e-6, "b = {b}");

        gx_doc_free(doc);
    }
}

#[test]
fn numeric_flags_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        // variable
        let a = gx_doc_add_var(doc, c"a".as_ptr(), 2.0, true) as usize;
        assert_eq!(gx_doc_var_numeric(doc, a), 0);
        assert_eq!(gx_doc_set_var_numeric(doc, a, true), 1);
        assert_eq!(gx_doc_var_numeric(doc, a), 1);
        // frozen value stays put
        assert_eq!(gx_doc_var_value(doc, a), 2.0);
        assert_eq!(gx_doc_set_var_numeric(doc, a, false), 1);
        assert_eq!(gx_doc_var_numeric(doc, a), 0);

        // dimension
        let o = gx_doc_origin(doc) as usize;
        let p = gx_doc_add_point(doc, 3.0, 0.0) as usize;
        let l = gx_doc_add_line(doc, o, p, false) as usize;
        let d = gx_doc_add_dim(doc, GX_DIM_LENGTH, l, 0, 0.3, 0.3) as usize;
        assert_eq!(gx_doc_set_dim(doc, d, c"3".as_ptr()), 1);
        assert_eq!(gx_doc_dim_numeric(doc, d), 0);
        assert_eq!(gx_doc_set_dim_numeric(doc, d, true), 1);
        assert_eq!(gx_doc_dim_numeric(doc, d), 1);

        // round trip through .gxp preserves the flag
        let out = gx_doc_to_gxp(doc);
        let again = gx_doc_from_gxp(out);
        gx_string_free(out);
        assert_eq!(gx_doc_dim_numeric(again, d), 1);
        gx_doc_free(again);

        // errors
        assert_eq!(gx_doc_var_numeric(doc, 99), -1);
        assert_eq!(gx_doc_dim_numeric(doc, 99), -1);
        gx_doc_free(doc);
    }
}

#[test]
fn dimension_label_offset_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let o = gx_doc_origin(doc) as usize;
        let p = gx_doc_add_point(doc, 4.0, 2.0) as usize;
        let d = gx_doc_add_dim(doc, GX_DIM_DISTANCE, o, p, 0.3, 0.3) as usize;
        let (mut x, mut y) = (0.0, 0.0);
        assert_eq!(gx_doc_dim_anchor(doc, d, &mut x, &mut y), 1);
        assert_eq!((x, y), (2.0, 1.0));
        assert_eq!(gx_doc_set_dim_offset(doc, d, -1.0, 0.5), 1);
        assert_eq!(gx_doc_dim_offset(doc, d, &mut x, &mut y), 1);
        assert_eq!((x, y), (-1.0, 0.5));
        assert_eq!(gx_doc_dim_offset(doc, d, std::ptr::null_mut(), std::ptr::null_mut()), 1);
        assert_eq!(gx_doc_set_dim_offset(doc, d, f64::INFINITY, 0.0), -1);
        assert_eq!(gx_doc_set_dim_offset(doc, 9, 0.0, 0.0), -1);
        assert_eq!(gx_doc_dim_offset(doc, 9, &mut x, &mut y), -1);
        assert_eq!(gx_doc_dim_anchor(doc, 9, &mut x, &mut y), -1);
        gx_doc_free(doc);
    }
}

#[test]
fn undo_steps_join_through_c_interface() {
    unsafe {
        let doc = gx_doc_new();
        let len = gx_doc_undo_len(doc);
        assert!(len >= 0);
        let a = gx_doc_add_point(doc, 0.0, 0.0);
        let b = gx_doc_add_point(doc, 1.0, 1.0);
        let l = gx_doc_add_line(doc, a as usize, b as usize, false);
        assert!(l >= 0);
        assert_eq!(gx_doc_join_undo(doc, len as usize), 1);
        assert_eq!(gx_doc_undo_len(doc), len + 1);
        assert_eq!(gx_doc_undo(doc), 1);
        assert_eq!(gx_doc_ent_kind(doc, a as usize), -1);
        assert_eq!(gx_doc_join_undo(doc, len as usize), 0);
        assert_eq!(gx_doc_undo_len(std::ptr::null()), -1);
        // two free points: four degrees of freedom
        assert_eq!(gx_doc_redo(doc), 1);
        gx_doc_solve(doc, std::ptr::null_mut());
        assert_eq!(gx_doc_dof(doc), 4);
        assert_eq!(gx_doc_dof(std::ptr::null()), -1);
        gx_doc_free(doc);
    }
}

#[test]
fn latex_png_through_c_interface() {
    unsafe {
        let latex = CString::new(r"x = \sqrt{a^{2} + b^{2}}").unwrap();
        let mut len = 0usize;
        let p = gx_latex_png(latex.as_ptr(), 12.0, 96.0, 0x204080, &mut len);
        assert!(!p.is_null());
        let png = std::slice::from_raw_parts(p, len);
        assert_eq!(&png[..4], b"\x89PNG");
        gx_bytes_free(p, len);

        let bad = CString::new(r"\frac{1}{").unwrap();
        assert!(gx_latex_png(bad.as_ptr(), 12.0, 96.0, 0, &mut len).is_null());
        assert!(!gx_last_error().is_null());
        assert!(gx_latex_png(latex.as_ptr(), -1.0, 96.0, 0, &mut len).is_null());
        assert!(gx_latex_png(latex.as_ptr(), 12.0, 96.0, 0, std::ptr::null_mut()).is_null());
        assert_eq!(text(gx_last_error()), "null length");
        gx_bytes_free(std::ptr::null_mut(), 0);
    }
}
