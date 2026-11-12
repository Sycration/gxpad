//! The example constructions the generated tests save in `test_examples/`:
//! they load and solve, and the formulas known to be short stay short.

mod testkit;

use gxgeo::derive;
use gxgeo::expr::bx::{plain, to_bx};
use gxgeo::formula::{Target, sym_name, target_expr};
use gxgeo::sketch::Sketch;
use testkit::EXAMPLES_DIR;

/// The example file at `path`, opened and solved.
fn open(path: &std::path::Path) -> Sketch {
    let text = std::fs::read_to_string(path).unwrap();
    let doc = gxgeo::gxp::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut sk = Sketch::from_doc(doc);
    sk.solve_full();
    sk
}

/// Every example file written by earlier test runs opens and solves without
/// errors.
#[test]
fn example_files_load() {
    let Ok(groups) = std::fs::read_dir(EXAMPLES_DIR) else { return };
    for group in groups.flatten() {
        let Ok(files) = std::fs::read_dir(group.path()) else { continue };
        for f in files.flatten() {
            let path = f.path();
            if path.extension().is_none_or(|e| e != "gxp") {
                continue;
            }
            let sk = open(&path);
            assert!(sk.errors.is_empty(), "{}: errors {:?}", path.display(), sk.errors);
        }
    }
}

/// Formula lengths. Each entry: an example file written by the generated
/// tests, and the most characters any of its reference dimensions' formulas
/// may have.
#[test]
fn known_short_formulas_stay_short() {
    let cases: &[(&str, usize)] = &[
        ("gen_angles/polar_walk_large_turning", 260),
        ("gen_angles/quadrilateral_angle_sum", 30),
        ("gen_angles/regular_pentagon_turning_angles", 130),
        ("gen_circle_tangency/wedge_chain_of_three", 90),
        ("gen_circle_tangency/descartes_inner_circle", 180),
        ("gen_constructions/napoleon_triangle", 150),
        ("gen_constructions/two_reflections_rotation_distance", 30),
        ("gen_constructions/bisector_reflection_on_side", 20),
        ("gen_invariants/ptolemy_diagonals", 80),
        ("gen_linkages/regular_pentagon_closing_side", 40),
        ("gen_linkages/hinged_triangles_median", 40),
        ("gen_triangle_centers/incenter_distance_to_sides_floating", 90),
        ("gen_variables/pentagon_golden_ratio", 120),
    ];
    derive::set_time_scale(1.0);
    for (file, limit) in cases {
        let path = std::path::Path::new(EXAMPLES_DIR).join(format!("{file}.gxp"));
        if !path.exists() {
            eprintln!("skipped (no example file yet): {file}");
            continue;
        }
        let sk = open(&path);
        let names = sk.doc.param_names();
        for i in 0..sk.doc.dims.len() {
            let Some(d) = sk.doc.dims[i].as_ref() else { continue };
            if d.driving {
                continue;
            }
            let (name, target, _) = target_expr(&sk.doc, Target::Dim(i)).expect("bad target");
            let der = derive::derive(&sk.doc, &target).unwrap();
            let text = plain(&to_bx(&der.expr, &|s| sym_name(&names, s)));
            assert!(der.exact, "{file}: {name} is not exact");
            assert!(text.len() <= *limit, "{file}: {name} has {} characters (limit {limit}): {text}", text.len());
            // the same derivation again gives the same formula
            let again = derive::derive(&sk.doc, &target).unwrap();
            assert_eq!(text, plain(&to_bx(&again.expr, &|s| sym_name(&names, s))), "{file}: {name} differs between two derivations");
        }
    }
}

/// Derives every reference dimension of the construction file named by
/// GXGEO_DERIVE_FILE, for debugging a formula that was not found
/// (GXGEO_DERIVE_LEVEL: extra search in steps of 10 s, default 0).
#[test]
#[ignore]
fn derive_file() {
    let Ok(path) = std::env::var("GXGEO_DERIVE_FILE") else { return };
    let sk = open(std::path::Path::new(&path));
    println!("dof {}, conflicts {:?}, errors {:?}", sk.dof, sk.conflicts, sk.errors);
    let level: u64 = std::env::var("GXGEO_DERIVE_LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    for i in 0..sk.doc.dims.len() {
        let Some(d) = sk.doc.dims[i].as_ref() else { continue };
        if d.driving {
            continue;
        }
        let (name, target, _) = target_expr(&sk.doc, Target::Dim(i)).expect("bad target");
        let t0 = std::time::Instant::now();
        match derive::derive_harder(&sk.doc, &target, level * 10_000) {
            Ok(der) => println!(
                "{name} = {}   [exact {}, value {}, measured {}, {:?}]",
                plain(&to_bx(&der.expr, &|s| format!("s{s}"))),
                der.exact,
                der.value,
                der.actual,
                t0.elapsed()
            ),
            Err(e) => println!("{name}: error {e}"),
        }
    }
}
