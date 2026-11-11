//! The C interface, called from Rust.

use gxgeo::ffi::*;
use std::ffi::{CStr, CString};

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
        let f = gx_derive(doc, GX_TARGET_DIM, 3, 0);
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
        assert!(gx_derive(doc, GX_TARGET_DIM, 0, 0).is_null());
        assert_eq!(text(gx_last_error()), "gone");
        assert!(gx_derive(doc, 7, 0, 0).is_null());
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

        let f = gx_derive(doc, GX_TARGET_DIM, ang, 0);
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
