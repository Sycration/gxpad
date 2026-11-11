# Symbolic derivation in gxpad: shortcomings

This file describes how the formula derivation works and where it falls short. It is for someone
continuing the work on it. Expected results are kept in tests, not here: `Kit::check` in the
generated tests (section 1.8) and `testkit::formula_sizes` for formula lengths.

gxpad is a 2D constraint-based geometry pad. A user draws points, lines, circles and conics, adds
constraints and driving dimensions (lengths, angles, coordinates, radii; named `d1`, `θ1`, `r1`,
…, or user variables such as `L`, `s`), and selects a reference dimension. The app then shows a
closed-form formula for that dimension in terms of the driving dimensions and variables.

## 1. How the derivation works

### 1.1 Files

| File | Content |
|---|---|
| `src/model.rs` | `Doc`: entities, parameters, constraints, dimensions. `Doc::equations`, `Doc::con_eqs`: constraints as expressions equal to 0. |
| `src/solver.rs` | Numeric solver (`Sys::build`, `Sys::solve`), used by the app, by the fit and by checks inside the derivation. |
| `src/derive.rs` | The derivation: structure, block solving, target, checks. |
| `src/fit.rs` | Formula discovery from samples: a polynomial relation fitted to numeric solutions. |
| `src/dd.rs` | Double-double arithmetic (about 31 digits) for the fit. |
| `src/expr/mod.rs` | Expression type `Expr`, canonicalizing constructors, evaluation, the simplifier, the work clock. |
| `src/expr/poly.rs` | `Expr`-level polynomial helpers (`coeffs`, `split_frac`). |
| `src/alg.rs` | Sparse multivariate polynomials over Q (`MPoly`), resultants, factorization. |
| `src/app.rs` | `compute_formula`, background search levels, formula display state. |
| `src/testkit.rs` | `Kit`: builds constructions in tests and checks derived formulas. |
| `tests/gen_*.rs` | Generated tests (constructions with known answers). |

### 1.2 Order of work (`derive.rs`)

`derive` / `derive_harder` → `derive_strategies` → `derive_impl` → `derive_inner`.

1. **Equations.** `Doc::equations` gives one expression per scalar constraint. Distances appear
   squared or as `sqrt(dx² + dy²) − d`, angle dimensions as `atan2(cross, dot) − θ` (`prep`
   turns that into `cross·cos θ − dot·sin θ`), point-line distances as `cross / sqrt(...)`.
2. **Constants and grounding.** Locked parameters that are not named become numbers
   (`nice_const`). If the target and the equations are invariant under translation (and
   rotation), one point is moved to the origin and a second onto an axis. There are two
   grounding strategies (`strategy` 0 and 1); strategy 1 runs only when strategy 0's result is
   not exact or has complexity ≥ 300, and consists of one block pass.
3. **Structure.** `rank_select` picks independent equations and unknowns numerically.
   `fix_free` gives remaining free unknowns simple values when the target does not depend on
   them. `matching` + `tarjan` give a block-triangular order of square blocks (a
   Dulmage–Mendelsohn decomposition).
4. **Brief block pass** (strategy 0): the blocks (1.4) with 150 ms. Most constructions are
   solved in that time.
5. **Fit** (1.3), when the brief pass left a formula that is not exact or has complexity above
   40 (`FIT_ABOVE`).
6. **Full block pass**, when the brief pass was not exact and there is no fitted formula of
   complexity ≤ 300 (`FIT_SETTLES`).
7. **Target.** The block results are substituted into the target expression. Then:
   - `fold_constants`: parts that are numerically constant under parameter perturbation become
     exact constants (8 perturbations; only for exact formulas);
   - direct elimination, only without a fitted formula of complexity ≤ 300, and only when the
     block formula is not exact or (with extra search time) larger than complexity 80; each
     method in a short slice (`scaled(250)` plus a fifth of the extra search time):
     - `solve_target_radicals`: each square root in the block-derived formula becomes an
       unknown r with r² = radicand, and elimination keeps a new unknown t = formula;
     - `solve_target`: t = target joins the needed blocks' original equations, and
       elimination keeps only t (`BlockSolver::keep`);
     - a direct result is used only if it agrees with the block-derived formula at perturbed
       parameters (`same_function`), or, without an exact block formula, with numeric re-solves
       of the construction (`agrees_when_perturbed`);
   - the smallest of the block, fitted and direct formulas is kept;
   - `simplify_result` (= `deep_best`), `fourier_best`, `factor_sums` (polynomial sums replaced
     by products when at most 70% of the size), `settle_zero_abs`, `settle_branch`.

### 1.3 Fit to samples (`fit.rs`, `dd.rs`)

The target is an algebraic function of the driving parameters. Its minimal polynomial is
searched for numerically:

- **Samples.** The construction is solved numerically (`solver::Sys`) at random parameter values
  around the current ones (±40%, or what moves an angle by 0.4 rad; the range is halved, down
  to 5%, while samples fail), by continuation from the nearest solved sample. Each solution is
  refined to double-double precision: Newton steps with residuals evaluated in `dd::Dd` and the
  f64 Jacobian. Samples near a singular configuration are rejected.
- **Atoms.** Parameters that occur outside sin/cos are polynomial atoms; arguments of sin/cos
  that are free of unknowns are angle atoms (cos and sin of the argument, of half the argument,
  and their second harmonics). Parameters the target does not depend on are dropped. Scaling
  all lengths tells whether the relation is homogeneous, which removes most monomials.
- **Relation.** A polynomial `Σ c·T^k·monomial·trig = 0` is the null vector of the matrix of
  basis functions at the samples (Householder QR with column pivoting, in double-double).
  Bases are tried from small to large: T or T² equal to a polynomial ("monic"), T or T² as a
  rational function, then quadratic and biquadratic relations in T; within each, total degree
  0..8, up to 170 columns (400 with extra search time).
- **Coefficients.** The null vector is refitted on its support and each coefficient is
  recognized as a fraction, a square root of one, or a quadratic irrational (`nice_coef`), with
  a required agreement of 1e-19. An exact relation leaves a residual near 1e-28; a good
  approximation leaves one many orders larger. In f64 the two cannot be told apart (a smooth
  function is approximated to 1e-10 by a few dozen basis functions), which is the reason for
  double-double.
- **Angles.** For an angle target, a rational combination of the angle atoms and π is tried
  first; otherwise cos T is fitted and the result is `acos(…)`.
- **Checks.** The relation must hold at samples that took no part in the fit. `derive.rs`
  solves it for T (`candidates` / `pick`), and the formula must reproduce the measurement, the
  check samples (`fit::agrees`) and small perturbations (`agrees_when_perturbed`).
- **|a| over the sample range.** When the form with |a| resolved by the current sign fails the
  check samples, the form with |a| is checked instead.

### 1.4 Blocks (`BlockSolver::solve_block`)

Each block is solved in terms of the earlier blocks' results:

- `solve_ray_circle`: a block of two unknowns with a circle and a line through the circle's
  centre (the usual result of a length and an angle dimension) is centre ± radius · unit
  direction. The length of the direction is simplified through the Fourier form, where it is a
  perfect square. Walks with turning angles give coordinates such as
  `d2·cos θ1 + d3·cos(θ1 + θ2) + …` without radicals.
- Otherwise two methods with shared time:
  - substitution search (`search`, `candidates`, `isolate`): isolates one unknown per
    equation (linear, quadratic formula, inverse trig, harmonic forms), with branch choice by
    the current numeric values (`pick`);
  - polynomial elimination (`solve_block_poly_trig` → `solve_block_poly` → `solve_polys`):
    - the equations become `MPoly`s over the block unknowns plus "atoms" (named parameters,
      `sqrt(...)` of known things, `sin θ`, `cos θ`);
    - `merge_trig` replaces `sin θ, cos θ` of a known angle by one atom: `tan(θ/2)` (kept as a
      symbol through elimination, `HalfTan`, converted back by `untrig_atom` /
      `half_tan_form`), or `tan θ` / `cot θ` when every polynomial is homogeneous in the pair;
    - each round: `interreduce` (row reduction over monomials), `vanishing_part` (factor and
      keep only factors that vanish at the current numeric point), then set aside a variable
      that occurs in one equation, substitute a linear one, or eliminate one by a resultant;
    - the last variable comes from its lowest-degree vanishing factor; the others come by
      back-substitution through subresultants; `resolve_smaller` re-solves variables from
      degree ≤ 2 equations when that halves the size.

Branch choices in the block path are local: the sign of each `|·|` (`resolve_abs`, which also
resolves `acos(cos u)` and `acos(sin u)` to ±u + kπ except within 0.1 rad of a fold), the root
of each quadratic (`pick`), and the factor of each eliminated polynomial (`vanishes`) are taken
from the current numeric configuration.

### 1.5 Expression layer (`expr/mod.rs`)

`Expr` is an immutable tree with canonicalizing constructors (`add`, `mul`, `pow`, `fun`):
flattening, rational arithmetic, collecting like terms, sorting into a canonical order,
`sin(x + kπ/2)` reductions, `sqrt` of perfect powers, inverse pairs. Simplification is heuristic:
`deep_best` works bottom-up, and at each node `best_form` produces several candidate forms
(expanded, Pythagoras-reduced by `trig_best`, over a common denominator by `rationalize_pass`,
factored by `factor_common`, cancelled by `cancel_fraction`) and keeps the one with the smallest
`complexity` (a node count).

`trig_linearize` gives the Fourier form of a polynomial in sines and cosines: products and powers
become sums of sin and cos of angle sums. That form is canonical for trigonometric polynomials,
so `sin² + cos² = 1` and the angle addition formulas reduce without a search. `fourier_best`
applies it to sums and radicands where the result is smaller. For everything else there is no
normal form and no zero-equivalence algorithm; identity detection is numeric (`fold_constants`,
`Env::is_zero`, `same_function`).

### 1.6 Algebra (`alg.rs`)

- `MPoly`: sparse polynomial over `BigRational`, monomials as `Vec<u16>`.
- `resultant`: Bareiss fraction-free elimination of the Sylvester matrix. `subresultant_chain`
  for back-substitution.
- `factor_main` (→ `factor_in`), factorization over Q:
  - monomial content, then content in the main variable;
  - quadratics via a perfect-square discriminant;
  - square-free check at evaluation points;
  - for repeated factors, `repeated_part`: lifting gcd(p, ∂p/∂x) out of ∂p/∂x (EEZ-GCD style),
    falling back to a subresultant square-free part limited to degree ≤ 8 and ≤ 400 terms;
  - univariate image factorization, multivariate Hensel lifting modulo four 62-bit primes
    (`lift_mod`), CRT and rational reconstruction, and subset recombination
    (`lift_and_recombine`).
- Monomials in the modular code are packed into one integer key: at most 16 variables
  (`KMAX`), 8 bits of degree each. `factor_main` therefore remaps each polynomial to a ring of
  only the variables it uses before factoring.
- `cheapest_main` picks the main variable for factoring.

### 1.7 Time: the work clock (`expr::work_now`, `charge`, `past_deadline`)

All deadlines are points on a clock that advances with work done, not with real time: one tick
(200 ns) per poll of `past_deadline` and per expression node built, 8 per `MPoly` term, plus
charges for tree comparisons and walks, evaluation, numeric solves and the double-double QR.
A derivation stops at the same point whatever the machine load, so its formula does not depend
on the load. A real-time limit (4 × budget + 4 s, `set_hard_deadline`) stops work that the clock
undercounts.

Strategy 0 gets 1.5 s and strategy 1 0.6 s of work time, plus the extra search time. The app
starts up to 3 automatic "search harder" levels (10 s extra each) while a formula is numeric or
larger than complexity 60 and the last level changed it (`AUTO_SEARCH_LEVELS`, `SETTLED_SIZE`).
The "Search harder" button is shown for the same kind of formula and adds one level per press.
The app's cancel flag (`set_cancel`) stops a derivation at once.

### 1.8 Tests

- `Kit::check` (generated tests): a derived formula must be exact (no numeric fallback), contain
  only named quantities, match the measured value, and keep matching while points are dragged
  and driving values perturbed (each time the construction is re-solved numerically). Tests run
  with the app's own budgets (`set_time_scale(1.0)`). `Kit::max_ms` (30 s of real time) only
  guards against a runaway computation.
- `testkit::formula_sizes::known_short_formulas_stay_short` (lib test): an upper length for
  every reference dimension of 13 example files, and the same formula from two derivations in a
  row.
- Unit tests: `fit` (null vector, coefficient recognition), `dd`, `expr` (Fourier form), `alg`,
  `derive`.

## 2. Shortcomings

### 2.1 Formulas that are long

Lengths in characters, times single-threaded:

| Test | Length | Time | Remark |
|---|---:|---:|---|
| `peaucellier_inversor_rotating` d8 | 11,378 | 1.6 s | See below. |
| `four_bar_coupler_triangle_point` x_D, y_D | 6,383 / 7,995 | 0.6–0.9 s | |
| `simson_feet_distance` d5, d7 | 5,546 / 3,493 | 2.7–3.3 s | These two feet distances involve a distance from P that is not dimensioned. |
| `fermat_point` d5–d7 | 2,002–2,434 | 2.6–4.2 s | Distances from the Fermat point to the vertices. |
| `varignon_other_diagonal` d6 | 1,519 | 3.5 s | A formula with two nested roots is found at search level 1. |
| `pedal_triangle_sides` d6 | 1,493 | 1.3 s | |
| `peaucellier_inversor_fixed_axis` x_B | 1,105 | 0.5 s | See below. |
| `quadratic_fit_solved_coefficients` y_F | 1,069 | 0.4 s | A 323-character rational function is found at a higher search level (direct elimination). |
| `four_bar_floating_distances` d6 | 755 | 0.7 s | |
| `confocal_orthogonal` θ1 | 627 | 0.5 s | |
| `rotated_ellipse_by_radial_point` d2 | 246 | 1.2 s | `acos` and `atan2` inside; the answer is 1/√(sin²φ/a² + cos²φ/b²). |

These are nested radicals whose minimal polynomial is of degree 4 or more in T, or of degree 2
with coefficients of total degree above 8. The fit's bases stop at 170 columns (400 with extra
time), and elimination swells (2.4). By a count of monomials, `simson_feet_distance` d5 would
need a quadratic relation of about 370 columns (not verified).

The Peaucellier tests dimension the six bars separately (d1…d6) and `Kit::check` perturbs each
one, so the formula has to hold for unequal bars, where the linkage is not an inversor and the
answer is not (l² − s²)/OP. A long nested radical is the correct kind of answer there. With one
variable per group of equal bars the fit would have 3 atoms (not tried).

Possible directions: sparse interpolation instead of a dense null space (the support of these
relations is a small part of the basis); a product-structured fit (lengths × angles on a sample
grid) for many angles; denesting of the block formula.

### 2.2 Fit limits

- Dense bases: the number of columns grows as 5^(angles) with second harmonics, so 3 or more
  independent angles together with several lengths do not fit (`rotated_ellipse_by_radial_point`
  has 3 lengths and 2 angles and needs second harmonics: 525 columns).
- `quadratic_fit_solved_coefficients` y_F needs 196 columns (5 free coordinates, a rational
  function of degree 5) and its samples only reach ±5%.
- Angle atoms are the arguments as they stand in the equations. A relation that is short only in
  a combination (θ1 − θ2) is found only through its expansion.
- Relations of degree 3 in T, or above 4, are not tried; nor are targets that are not algebraic
  in the atoms (curves with exp, ln; arc lengths).
- A null space of dimension above 1 is skipped.
- Sampling follows one branch by continuation. A construction that cannot move by 5% gives no
  samples.
- The fit runs for strategy 0 only.

### 2.3 Local validity

|a|, roots and factors in the block path are chosen at the current configuration, and the app
shows one formula with no domain of validity. The same construction can show different formulas
after dragging across a branch boundary. A fitted formula is checked over the sample range
(±40% or what could be sampled), and |a| is kept where the sign changes inside that range, but
the final `resolve_abs` resolves it by the current sign for display. `acos(cos u)` within 0.1
rad of a fold stays as it is.

### 2.4 Block path and simplifier

This is what produces the long formulas of 2.1 when the fit finds nothing:

- **Radical swell.** Each intersection adds a square root of an expression that already has
  roots; there is no denesting and no normal form for nested radicals. `deep_best` returns
  expressions over 3000 nodes unchanged, `best_form` over 4000.
- **Degenerate components.** Elimination has no saturation, Gröbner basis or triangular
  decomposition; the only defence is factoring each resultant and keeping the factors that
  vanish at the current point (`vanishing_part`).
- **Choices at one configuration.** Factors, roots and signs inside `solve_polys` and the
  substitution search are chosen at the current configuration only. Constructions often sit at
  special values (60°, 90°, equal lengths) where several branches coincide. Direct elimination
  results are checked nearby (`same_function`, `agrees_when_perturbed`); block results are not.
- **Trig atoms.** `sin θ` and `sin(θ/2)` (or `sin(θ1 + θ2)`) are unrelated atoms in `PolyCtx`;
  `merge_trig` only merges the sin/cos pair of one argument. Coordinates from
  `solve_ray_circle` carry atoms of angle sums into later blocks.
- **Factorization over Q only**, with the limits in `alg.rs` (16 variables, 8-bit degrees, 4000
  terms, subresultant square-free fallback up to degree 8 and 400 terms, recombination
  exponential in the number of image factors, dense Sylvester resultants, `pick_point` prefers
  sparse points). The fit recognizes quadratic irrational constants, but a constant that
  reaches the final formula through the block path gets only the heuristic simplifier.
- **Greedy local simplification** by node count: the result depends on the order of
  construction. Canonical ordering undoes chosen signs (`d2 + d3 − d1` comes back as
  `−(d1 − d2 − d3)`); `pow` over a product splits off non-negative factors, which separates a
  Heron product into two roots.
- **Numeric identity tests** (`fold_constants`, `same_function`, `Env::is_zero`) use a few
  perturbations in f64 with fixed tolerances.

### 2.5 Work clock calibration

The tick charges are a calibration, not a measurement. Across the suite the median rate is 5.3
million ticks per second of real time (nominal 5 million), with a 10%–90% range of 3.1–9.4
million and extremes of 1.6 and 12.5 million. The real-time limit is the only bound for work
that polls and builds nothing for a long time, and a derivation stopped by it depends on load.
Under 12 parallel test threads the slowest derivations take 10–11 s of real time (3–4 s alone).

### 2.6 Ignored tests

- `gen_functions::sine_inflection_tangent_unpinned`: tangency at an inflection is a triple
  contact; in f64 the numeric solver reaches about 1e-7 there (the residual x·cos x − sin x loses
  its digits near 0). This is a limit of the numeric solver, not of the derivation.
- `alg::tests::factor_dump`, `testkit::derive_file::derive_file`: debugging helpers (section 3).

## 3. Tools

Environment variables:
- `GXPAD_DEBUG=1`: per-block log of the derivation (blocks, equations, elimination steps,
  factor sizes, failure reasons, direct solving), the fit (`fit:` lines: atoms, bases, ranks,
  relations, check results), the stages after the blocks with real time and ticks (`stage`),
  and ticks against real time per strategy (`work:`).
- `GXPAD_DUMP=<file>`: write the last polynomial of a polynomial block, for `factor_dump`.
- `GXPAD_FACTOR_FILE=<file>` (with optional `GXPAD_MAIN=<var>|cheapest`): factor a dumped
  polynomial and report timings (`alg::tests::factor_dump`).
- `GXPAD_DERIVE_FILE=<file.gxp>` (with optional `GXPAD_DERIVE_LEVEL=<n>`): derive every reference
  dimension of a saved construction (`testkit::derive_file`).

Each test run writes its construction to `test_examples/<suite>/<test>.gxp`, which opens in the
app (`cargo run --release -- <file>`).

Commands:

```sh
cargo test --no-fail-fast                                   # full suite
cargo test --lib                                            # unit tests (alg, expr, derive, fit, dd, formula sizes)
timeout 300 cargo test --test gen_invariants ptolemy_diagonals -- --nocapture   # one test, prints formulas
GXPAD_DEBUG=1 timeout 300 cargo test --test gen_linkages peaucellier_inversor_rotating -- --nocapture
GXPAD_DERIVE_FILE=test_examples/gen_constructions/simson_line.gxp \
  cargo test --lib derive_file -- --ignored --nocapture
GXPAD_DUMP=target/p.poly cargo test --test gen_invariants ptolemy_diagonals
GXPAD_FACTOR_FILE=target/p.poly GXPAD_MAIN=cheapest cargo test --lib alg::tests::factor_dump -- --ignored --nocapture
```

Run single tests under `timeout`: a stuck derivation otherwise blocks the run. Formulas do not
depend on machine load (section 1.7); times do, so measure them single-threaded
(`--test-threads=1`).
