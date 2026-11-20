# Symbolic derivation in sgpad: shortcomings

This file describes how the formula derivation works and where it falls short. It is for someone
continuing the work on it. Expected results are kept in tests, not here: `Kit::check` in the
generated tests (section 1.8) and `known_short_formulas_stay_short` for formula lengths.

sgpad is a 2D constraint-based geometry pad. A user draws points, lines, circles and conics, adds
constraints and driving dimensions (lengths, angles, coordinates, radii; named `d1`, `θ1`, `r1`,
…, or user variables such as `L`, `s`), and selects a reference dimension. The app then shows a
closed-form formula for that dimension in terms of the driving dimensions and variables.

## 1. How the derivation works

### 1.1 Files

| File | Content |
|---|---|
| `sggeo/src/model.rs` | `Doc`: entities, parameters, constraints, dimensions. `Doc::equations`, `Doc::con_eqs`: constraints as expressions equal to 0. |
| `sggeo/src/solver.rs` | Numeric solver (`Sys::build`, `Sys::solve`), used by the app, by the fit and by checks inside the derivation; `Sys::analyze` counts the degrees of freedom (Jacobian rank, and at a double or triple root, where rows look dependent, whether a column held 2% away leaves the rest solvable). |
| `sggeo/src/derive.rs` | The derivation: structure, block solving, target, checks. |
| `sggeo/src/nf.rs` | Normal form of formulas with nested square roots: exact arithmetic in the tower of roots. |
| `sggeo/src/fit.rs` | Formula discovery from samples: a polynomial relation fitted to numeric solutions. |
| `sggeo/src/dd.rs` | Double-double arithmetic (about 31 digits) for the fit. |
| `sggeo/src/expr/mod.rs` | Expression type `Expr`, canonicalizing constructors, evaluation, the simplifier, the work clock. |
| `sggeo/src/expr/poly.rs` | `Expr`-level polynomial helpers (`coeffs`, `split_frac`). |
| `sggeo/src/alg.rs` | Sparse multivariate polynomials over Q (`MPoly`), resultants, factorization. |
| `sgpad/src/app.rs` | `compute_formula`, background search levels, formula display state. |
| `sggeo/tests/testkit/mod.rs` | `Kit`: builds constructions in tests and checks derived formulas. |
| `sggeo/tests/gen_*.rs` | Generated tests (constructions with known answers). |
| `sggeo/tests/examples.rs` | Formula lengths of saved examples; `derive_file` for debugging. |

### 1.2 Order of work (`derive.rs`)

`derive` / `derive_harder` → `derive_strategies` → `derive_impl` → `derive_inner`.

In short: the blocks give a formula by substitution; its normal form (1.9) is taken; a fit to
samples (1.3) looks for a simpler relation; the smallest formula is kept.

1. **Equations.** `Doc::equations` gives one expression per scalar constraint. Distances appear
   squared or as `sqrt(dx² + dy²) − d`, angle dimensions as `atan2(cross, dot) − θ` (`prep`
   turns that into `cross·cos θ − dot·sin θ`), point-line distances as `cross / sqrt(...)`.
2. **Constants and grounding.** Locked parameters that are not named become numbers
   (`nice_const`). If the target and the equations are invariant under translation (and
   rotation), one point is moved to the origin and a second onto an axis. The second point is,
   among those at a dimensioned length from the first, the one that closes the most triangles
   of dimensioned lengths with it: the third point of such a triangle needs one square root,
   points further away nested ones. There are two
   grounding strategies (`strategy` 0 and 1); strategy 1 runs only when strategy 0's result is
   not exact, or has complexity ≥ 300 and is not in normal form (a normal form comes out the
   same from the other grounding), and consists of one block pass.
3. **Structure.** `rank_select` picks independent equations and unknowns numerically; where
   the rows look dependent, `generic_rank_select` takes the rank at a nearby configuration
   (known parameters moved a little and the unknowns re-solved), so a double root (the foci of
   an ellipse with a = b) does not leave its unknowns free. A distance that is 0 here (driven
   by a vanishing expression) becomes the two coordinate equations of a coincidence.
   `fix_free` gives remaining free unknowns simple values when the target does not depend on
   them. `matching` + `tarjan` give a block-triangular order of square blocks (a
   Dulmage–Mendelsohn decomposition).
4. **Brief block pass** (strategy 0): the blocks (1.4) with 150 ms. Most constructions are
   solved in that time.
5. **Normal form** (1.9) of the brief pass's formula, when that is exact and has complexity
   above 40 (`FIT_ABOVE`), with 300 ms.
6. **Fit** (1.3), when the brief pass left a formula that is not exact, or neither it nor its
   normal form is settled: complexity at most 40, no inverse trigonometric function and no
   sine or cosine under a root (angle relations and half angles are what the fit finds). With
   a normal form in hand the fit gets 250 ms instead of 700: the relations much simpler than
   a normal form are in its first, small bases.
7. **Full block pass**, when the brief pass was not exact and there is no fitted formula of
   complexity ≤ 300 (`FIT_SETTLES`).
8. **Target.** The block results are substituted into the target expression. Then:
   - `fold_constants`: parts that are numerically constant under parameter perturbation become
     exact constants (8 perturbations; only for exact formulas);
   - the normal form of a formula from the full pass; a normal form at most a quarter of the
     substituted formula's size replaces it without simplifying that one, otherwise the
     smaller of the normal form and the simplified formula is kept;
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
   - `simplify_result` (`deep_best`, then `divided`), `fourier_best`, half angles
     (`half_angles`, and the normal form again when the formula then has θ and θ/2),
     `divided` again (a normal form much smaller than the substituted formula has not been
     through `simplify_result`), `factor_sums` (polynomial sums replaced by products, with
     multiplicities, when at most 70% of the size; otherwise collected, `collected`: the sum
     in powers of one variable with each coefficient factored or collected again, two
     levels deep, when clearly shorter, `clearly_shorter`), `resubstituted` (a sum written
     as c·S + R through a sum S that is a denominator or occurs twice, R factored: the
     radicand of a four-bar joint becomes (L² − (d2 + d3)²)(L² − (d2 − d3)²) with L² the
     law of cosines of its denominator), `settle_zero_abs`, `settle_branch`.
   - `with_atoms`: the steps on rational functions (`factored_root`, `collected` from
     `factor_sums`, `coef_form`) see roots and trigonometric functions as symbols, so a
     radicand a² + (√H₁·√H₂ − b²)/c² goes over one denominator. Cardano's and Ferrari's
     quantities (p, q, r, discriminants) take the factored quotient (`coef_form`) where it
     is clearly shorter than `best_form`.
   - `divided` (`divide_out`): square roots multiplied together go under one root, and each
     rational function N/D becomes Q + R/D with N = Q·D + R from the division with one
     variable leading (each tried; R and D factored by `factor_sums`). A rational function
     is divided as a whole, never in parts. The result is kept when it is at most 80% of the
     complexity, or no larger with fewer fraction bars. This is what writes the Euler line
     as √(9a²b²c²/Π − a² − b² − c²) (Π the Heron product) instead of a degree-6 numerator
     over Π split into two roots. `simplify_result` also runs on block results, so block
     results reach later blocks divided too. A root of a rational function is also tried
     over one denominator with both sides factored (`factored_root`, `factor_poly`), in
     the product around it, where the factors can cancel: a·√(c² − (a² + c² − b²)²/(4a²))
     /(a + b + c) is √((a + b − c)(a − b + c)(b + c − a)/(a + b + c))/2.

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
  and their second harmonics). Parameters the target does not depend on are dropped: each is
  moved alone, and those that leave the target unchanged are moved again with all the others
  moved too (P on a tangent line x = r leaves the contact point where it is until x_P moves).
  Scaling all lengths tells whether the relation is homogeneous, which removes most monomials.
- **Relation.** A polynomial `Σ c·T^k·monomial·trig = 0` is the null vector of the matrix of
  basis functions at the samples (Householder QR with column pivoting, in double-double).
  Bases are tried from small to large: T or T² equal to a polynomial ("monic"), T or T² as a
  rational function, then quadratic and biquadratic relations in T, then cubic relations in T
  and in T²; within each, total degree 0..8, up to 170 columns (400 with extra search time).
- **Coefficients.** The null vector is refitted on its support and each coefficient is
  recognized as a fraction, a square root of one, or a quadratic irrational (`nice_coef`), with
  a required agreement of 1e-19. A relation T·m₁ = r·m₀ whose ratio r is none of these is
  tried as m·sin(kπ/n) or m·cos(kπ/n) (`trig_constant`, n ≤ 30), then as an algebraic number of
  degree 3 to 6 whose minimal polynomial is found by LLL reduction (`algebraic_relation`); a
  constant target likewise. The side of a regular heptagon, 2r·sin(π/7), comes from there. An exact relation leaves a residual near 1e-28; a good
  approximation leaves one many orders larger. In f64 the two cannot be told apart (a smooth
  function is approximated to 1e-10 by a few dozen basis functions), which is the reason for
  double-double.
- **Angles.** For an angle target, a rational combination of the angle atoms and π is tried
  first; otherwise cos T is fitted and the result is `acos(…)`.
- **Checks.** The relation must hold at samples that took no part in the fit. `derive.rs`
  solves it for T (`candidates` / `pick`), and the formula must reproduce the measurement, the
  check samples (`fit::agrees`) and small perturbations (`agrees_when_perturbed`: each of its
  parameters alone, and all parameters of the construction together, so a parameter it lost
  shows).
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
    equation, with branch choice by the current numeric values (`pick`):
    - polynomials: linear, quadratic formula, Cardano (trigonometric form for three real roots)
      and Ferrari; above degree 4, the factors that vanish, then (`poly_roots`) a polynomial in
      vᵍ (also after a shift of v), a reciprocal polynomial (in v + 1/v), a composition f(g(v))
      of lower degrees, or a Dickson polynomial (x⁵ − 5x³ + 5x = h). A leading coefficient
      that is 0 at the configuration (a variable left at 0) gives the roots as reciprocals of
      the reversed polynomial's, which stay finite there;
    - changes of variable: u = v^(1/L) for fractional powers, u = e^(κv) for exponentials with
      rational rates, Π Aᵢ^qᵢ = e^R for sums of logarithms, and cos κv, sin κv for sin, cos and
      tan of rational multiples of v plus constants (a polynomial in them is solved by the
      harmonic forms, or reduced to one in sin or cos alone);
    - inverse trigonometric and other elementary functions, harmonic forms;
    - an equation in one unknown and nothing else (`find_constant_root`): a fraction with
      denominator ≤ 12, or such a multiple of π, within 1e-3 of the numeric value that makes it
      exactly 0 (a multiple root, such as the contact point of the tangent at an inflection, is
      known numerically only to a power of the rounding error);
  - polynomial elimination (`solve_block_poly_trig` → `solve_block_poly` → `solve_polys`):
    - the equations become `MPoly`s over the block unknowns plus "atoms" (named parameters,
      `sqrt(...)` of known things, `sin θ`, `cos θ`);
    - an unknown angle θ becomes the unknowns cos κθ and sin κθ, κ the unit of the multiples
      of θ that occur (sin 3t and sin t: κ = 1, by the multiple-angle formulas);
    - `merge_trig` replaces `sin θ, cos θ` of a known angle by one atom: `tan(θ/2)` (kept as a
      symbol through elimination, `HalfTan`, converted back by `untrig_atom` /
      `half_tan_form`), or `tan θ` / `cot θ` when every polynomial is homogeneous in the pair;
    - each round: `interreduce` (row reduction over monomials), `vanishing_part` (factor and
      keep only factors that vanish at the current numeric point), then set aside a variable
      that occurs in one equation, substitute a linear one, or eliminate one by a resultant;
    - the last variable comes from its lowest-degree vanishing factor; the others come by
      back-substitution through subresultants (only when the target or a later block uses
      them); `resolve_smaller` re-solves variables from degree ≤ 2 equations when that halves
      the size.

`solve_circle_circle`: a block of two unknowns on two circles whose centres already carry
square roots (a joint hung from two computed joints) is c1 + (k·d ± h·d⊥)/D², with d = c2 − c1,
D² = |d|², k = (r1² − r2² + D²)/2 and h = √(r1²·D² − k²): one new root over the roots of the
centres. Its parts and the two coordinates go through the normal form where that is smaller.
Circles with root-free centres (a triangle from its sides) stay with the methods below.

A block's result with complexity above 40 is replaced by its normal form (1.9, 60 ms) when
that is at most half the size. A form only a little smaller is not taken: its expanded
polynomials make the polynomial elimination of the next block slower. After the first result
that runs out of time, the normal form is no longer tried for the blocks.

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
`complexity` (a node count, in which a coefficient −1 counts as a minus sign: −c costs 4, c 2).

`pow` takes numbers, symbols and monomials that are non-negative out of a root of a product,
but keeps sums (odd powers of them) under it with the rest: the Heron product stays one root.
`split_roots` takes such sums out again: the normal form gets its input that way (separate
roots keep its inverses small), and `simplify_result` tries it too where such a sum also
occurs outside the root (√(a·b)/a = √b/√a).

The display (`bx.rs`) shows a product's sum factors that have more negative than positive
terms negated, in pairs or with the product's sign (`display_signs`): `(d2 + d3 − d1)`
rather than `−(d1 − d2 − d3)`, which the canonical form would undo. A root of a reciprocal is
shown as 1 over the root (`root_of_reciprocal`).

`trig_linearize` gives the Fourier form of a polynomial in sines and cosines: products and powers
become sums of sin and cos of angle sums. That form is canonical for trigonometric polynomials,
so `sin² + cos² = 1` and the angle addition formulas reduce without a search. `fourier_best`
applies it to sums and radicands where the result is smaller. Formulas with square roots have
the normal form of 1.9. Inside the block solver and the simplifier there is no normal form;
identity detection there is numeric (`fold_constants`, `Env::is_zero`, `same_function`).

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
- The primes are 2^62 − c with small c, so a product is reduced by folding the high part onto
  the low part (`redm`), without a division.

### 1.7 Time: the work clock (`expr::work_now`, `charge`, `past_deadline`)

All deadlines are points on a clock that advances with work done, not with real time: one tick
(200 ns) per poll of `past_deadline` and per expression node built, 8 per `MPoly` term, plus
charges for tree comparisons and walks, evaluation, numeric solves and the double-double QR.
A derivation stops at the same point whatever the machine load, so its formula does not depend
on the load. A real-time limit (4 × budget + 4 s, `set_hard_deadline`) stops work that the clock
undercounts.

Strategy 0 gets 1.5 s and strategy 1 0.6 s of work time, plus the extra search time. With
extra time the normal form gets a tenth of it and elements of up to 60,000 terms (8,000
otherwise). The app
starts up to 3 automatic "search harder" levels (10 s extra each) while a formula is numeric or
larger than complexity 60 and the last level changed it (`AUTO_SEARCH_LEVELS`, `SETTLED_SIZE`).
The "Search harder" button is shown for the same kind of formula and adds one level per press.
The app's cancel flag (`set_cancel`) stops a derivation at once.

### 1.8 Tests

- `Kit::check` (generated tests): a derived formula must be exact (no numeric fallback), contain
  only named quantities, match the measured value, and keep matching while points are dragged
  and driving values perturbed (each time the construction is re-solved numerically; a move
  that leaves part of the construction free, such as a distance driven to 0 that becomes a
  small circle, is skipped). Tests run
  with the app's own budgets (`set_time_scale(1.0)`). `Kit::max_ms` (30 s of real time) only
  guards against a runaway computation.
- `known_short_formulas_stay_short` (`tests/examples.rs`): an upper length for every reference
  dimension of 22 example files, and the same formula from two derivations in a row.
- Unit tests: `nf` (cancellation, square factors, identified roots, denesting, absolute values,
  trigonometry),
  `fit` (null vector, coefficient recognition), `dd`, `expr` (Fourier form), `alg`, `derive`.

### 1.9 Normal form (`nf.rs`)

A formula built by substitution is an element of a tower of square roots over the rational
functions of the parameters. `nf::normal_form` computes in that tower exactly:

- **Generators.** Each distinct square root is a generator g with g² = ρ, ρ a polynomial in the
  parameters ("atoms": symbols, π, and functions that are not algebraic) and the earlier
  generators. An element is Σ p_S · Π_{i∈S} g_i over a polynomial denominator kept as a list
  of factors, with p_S polynomials (`MPoly`). Products are reduced with g² = ρ; a denominator
  with roots is multiplied by its conjugates (`rn_inv`); factors of the denominator that
  divide the numerator are cancelled after each operation (`cancel`).
- **Square roots** (`sqrt_depth`). The radicand is factored and its square factors leave the
  root (√(q²·h) = |q|·√h, the sign of q at the current configuration). Where q is zero there
  (equal lengths, coinciding points) its sign cannot be read off, and |q| becomes a generator
  with |q|² = q² (`abs_poly`): products still reduce, and only a division by |q| ± q fails.
  A radicand that is zero there is a root like any other. A radicand without generators is then a set of primes (irreducible
  polynomials, integer primes, −1), and it is a product of known roots exactly when the sets
  cancel (`root_base`): √(f·g) with √f and √g known, the same root up to a square factor.
  A radicand a + b·g is denested when a² − b²·ρ is a square d² in the field below
  (`root_nested`): √(a + b·g) = √((a + d)/2) ± √((a − d)/2); this is also what takes the
  root of a perfect square. What remains becomes a new generator.
- **Trigonometry.** Sines and cosines of sums and multiples are expanded over base angles
  β = term / (common denominator of its coefficients), with cos β an atom and sin β a
  generator (sin² β = 1 − cos² β). cos and sin of acos, asin, atan and atan2 are taken
  algebraically.
- **Output.** The plain square roots of a term are written as one root (√f·√(f·g) = |f|·√g),
  what all terms share goes in front, and polynomials are written as products where that is
  at most 70% of the size.

With independent generators the form is canonical. The derivation still compares the result
with the input numerically (`same_function`), because signs are taken at the current
configuration.

## 2. Shortcomings

### 2.1 Formulas that are long

Lengths in characters, times single-threaded (release build):

| Test | Length | Time | Remark |
|---|---:|---:|---|
| `peaucellier_inversor_rotating` d8 | 2,233 | 1.1 s | See below. |
| `peaucellier_inversor_fixed_axis` x_B | 990 | 0.2 s | See below. |
| `four_bar_coupler_triangle_point` x_D, y_D | 588 / 532 | 0.7 s | Two-circle joints (1.4). |
| `fermat_point` d5–d7 | 298 | 0.3 s | Normal form, its radicand collected (`collected`). |
| `pedal_triangle_sides` d6 | 304 | 0.5 s | Normal form. |
| `four_bar_floating_distances` d6 | 292 | 0.2 s | Normal form. |
| `quadratic_fit_solved_coefficients` d1 | 226 | 0.2 s | Normal form; a root of a discriminant. |
| `simson_feet_distance` d7 | 263 | 0.6 s | Normal form. |
| `confocal_orthogonal` θ1 | 264 | 0.4 s | Normal form of the cosine; the angle is π/2 (not recognized). |
| `long_truss_chain` (6 joints) | 145,993 | 0.9 s | Joints at given distances from the two before, unequal bars: each nests the roots of the earlier ones (4 joints: 282, 5: 895; 7: numeric after 1.9 s). |
| `ellipse_ellipse_rotated_quartic` d1 | 217,072 | 0.4 s | A rotated ellipse (angle locked, so sin and cos of it are constants) meets an offset one. |
| `elliptic_chord_third_point_symbolic` x_C | 1,189 | 0.7 s | Vieta gives m² − x_P − x_Q; (x − x_P)(x − x_Q) is not divided out because √(x_P³ + a·x_P + b) is an atom of its own. |
| `morley_triangle_side` d2 | 1,034 | 1.4 s | 8R·sin(A/3)·sin(B/3)·sin(C/3) is not found. |
| `symmetric_system_cubic` y_D | 916 | 1.0 s | The other variables come from a quadratic over the first one's root, not as the cubic's other trigonometric branches. |
| `varignon_other_diagonal` d6 | 142 | 0.1 s | Normal form. |
| `alhazen_reflection` x_C | 17,994 | 0.6 s | Ferrari's formula from elimination; `factor_sums` alone halves it but runs out of its slice. |
| `rotated_ellipse_by_radial_point` d2 | 246 | 0.5 s | `acos` and `atan2` inside; the answer is 1/√(sin²φ/a² + cos²φ/b²). |

The Peaucellier tests dimension the six bars separately (d1…d6) and `Kit::check` perturbs each
one, so the formula has to hold for unequal bars, where the linkage is not an inversor and the
answer is not (l² − s²)/OP. Checked with Maxima for the fixed-axis test:

- For unequal bars x_Q is a root of an irreducible polynomial of degree 4 over the bar lengths
  (the eliminant is its square; |OQ|² of the rotating test has degree 4 as well). A formula
  with two levels of square roots is therefore the simplest kind there is.
- The formula written by hand (A and B from OP, then Q on the two circles) has 860 characters
  in Maxima's notation; sgpad's has 990. Maxima's own simplifiers only lengthen it: `ratsimp`
  4,190, `radcan` 4,277, `fullratsimp` 4,023, `ratsimp` with `algebraic: true` 2,866,
  `factor` 28,105 characters, and `sqrtdenest` changes nothing. The form over a denominator
  without roots did not finish in 10 minutes.
- For the rotating test, |OQ| written by hand as √(x² + y²) has 1,751 characters, and
  through |OQ|² = |OA|² + |AQ|² + 2·OA·AQ it has 1,103. sgpad measures √(x² + y²) of its
  coordinates and has 2,233: the shorter arrangement is not found.

Both tests sit at equal bars, where factors such as d1 − d2 are zero. The normal form keeps
the absolute value of such a factor (1.9), so that is not what stops it: over a denominator
without roots these formulas pass 8,000 terms (and 60,000 with extra search time), which
agrees with Maxima not finishing that form.

The normal form's limits:

- **Size.** Rationalizing a denominator doubles the degree for each root in it, and the
  polynomials are expanded. There is no polynomial gcd; cancellation is by trial division with
  the denominator's factors, which the factorization has to find first (most of the normal
  form's time is factoring).
- **Angles.** A formula with more than two angle terms is not attempted: products of sines and
  cosines of many angles grow much faster than their Fourier form. `polar_walk_large_turning`
  and `rotated_ellipse_by_radial_point` (which counts its `acos` and `atan2` terms) fall
  under this.
- **Nested roots** are identified only when their radicands are equal, or by denesting.
  Dependent nested roots (√(a + b·g) and √(a − b·g), whose product is in the field below) stay
  separate generators, and the form is then not canonical.
- **Constants** such as the angle of `confocal_orthogonal` (acos of an expression that is 0)
  are found only when the cosine reduces to a number.
- **Blocks.** Normal forms of block results help only when much smaller (1.4); the equations
  of later blocks still receive earlier results by substitution, not as symbols.
- **Special configurations.** A denominator that is zero, a sine that is zero where its sign
  is needed, or a radicand that is negative at the current configuration ends the attempt,
  for the rest of the derivation (`nf::degenerate_seen`). A nested root that is zero there is
  not denested.
- **The target's own form.** A distance is the root of x² + y² of the coordinates as derived;
  a shorter arrangement of the same quantity (a law of cosines over known lengths, as for
  the rotating Peaucellier linkage) is not looked for.

Possible directions: a polynomial gcd (so that denominators need no factoring); a Fourier
basis for the trigonometric part; earlier block results as symbols with their minimal
polynomials in later blocks; sparse interpolation in the fit.

Measured (2026-11): for the linkages the normal form is not short of time but larger than the
substituted formula, so none of the speed directions above shortens them:

- `peaucellier_inversor_fixed_axis`: the normal form of the whole formula completes, with
  complexity 5,410 against 996 substituted (and −∞ at the current configuration: a
  denominator vanishes at equal bars). The "power too large" lines in the log are the 60 ms
  block-level attempts running out of time, not the final one.
- `four_bar_coupler_triangle_point`: 759 → 990 and 646 → 659.
- Taking the normal form one level of the tower at a time (each radicand, and then the
  whole, with roots nested deeper held as symbols) changed no formula of the suite: already
  one level with only the two Heron roots as generators grows from 394 to 1,437. Expanded
  polynomials over a common denominator are longer than the nested fractions that
  substitution leaves, and those fractions are close to the hand-written formula (see the
  Maxima figures above).

What is left for these formulas is their arrangement, not their normal form:

- Named intermediate quantities. The app (sgpad, the XP pane and sgweb) shows a formula as
  `latex_short` and the definitions `u_1 = …`, … from `expr::cse`: repeated subexpressions
  (sums of symbols only from 10 nodes, so a Heron factor stays in place), then, while the
  formula is longer than 60 nodes, its largest once-used roots and sums up to half its size,
  numbered in reading order (each definition uses only earlier ones).
  `peaucellier_inversor_rotating` shows as 7 rows of 395 characters in all instead of 2,126.
- The target's own form, tried and not kept (2026-11): a distance |PQ| through a point R at
  driving distances from both, |PQ|² = |PR|² + |RQ|² + 2(R − P)·(Q − R), with Q − R taken
  as the offset from the circle centre where Q was found on two circles
  (`solve_circle_circle`). For the rotating linkage that gives 1,849 against 1,836 through C
  (2,026 through D), and it changed no other formula of the suite. The short hand-written
  form goes through A, the rhombus centre, where |AB| is a diagonal and not a dimension: it
  needs B computed in a frame centred on A (a grounding chosen for the target), not a
  rewriting of the distance.

### 2.2 Fit limits

- Dense bases: the number of columns grows as 5^(angles) with second harmonics, so 3 or more
  independent angles together with several lengths do not fit (`rotated_ellipse_by_radial_point`
  has 3 lengths and 2 angles and needs second harmonics: 525 columns).
- `quadratic_fit_solved_coefficients` y_F needs 196 columns (5 free coordinates, a rational
  function of degree 5) and its samples only reach ±5%.
- Angle atoms are the arguments as they stand in the equations. A relation that is short only in
  a combination (θ1 − θ2) is found only through its expansion.
- Relations of degree 5 in T, of degree 6 with odd powers, and full quartics are not tried; nor
  are targets that are not algebraic in the atoms (curves with exp, ln; arc lengths).
- A null space of dimension above 1 is skipped.
- Sampling follows one branch by continuation. A construction that cannot move by 5% gives no
  samples.
- The fit runs for strategy 0 only.
- A fitted relation whose root needs a sign that changes within the sample range (the root of
  a discriminant Q²·H is |Q|·√H, and the function has Q·√H) fails the check samples. The
  Fermat point distances were such a case; the normal form finds them now, the fit still
  does not.
- The QR in double-double arithmetic is the fit's cost (n²·m operations for n columns), done
  again for each basis.

### 2.3 Local validity

|a|, roots and factors in the block path are chosen at the current configuration, and the app
shows one formula with no domain of validity. A cubic's roots are Cardano's form where it has
one real root and the trigonometric form where it has three, and no formula in real radicals and
trigonometric functions holds on both sides of the discriminant's zero
(`gen_polynomials::cubic_near_double_root` stays on one side). Likewise the focus of an
ellipse is √(a² − b²) along the first axis for a > b and √(b² − a²) along the second for a < b;
at a = b the formula is that of one side, and which focus the point becomes when a and b part
is open (`gen_conics::ellipse_degenerate_circle_focus`). The same construction can show different formulas
after dragging across a branch boundary. A fitted formula is checked over the sample range
(±40% or what could be sampled), and |a| is kept where the sign changes inside that range, but
the final `resolve_abs` resolves it by the current sign for display. `acos(cos u)` within 0.1
rad of a fold stays as it is. The normal form takes the sign of each square factor that leaves
a root, and the branch of each denested root, at the current configuration as well.

### 2.4 Block path and simplifier

This is what produces the long formulas of 2.1 when the fit finds nothing:

- **Radical swell.** Each intersection adds a square root of an expression that already has
  roots. The normal form (1.9) removes it where it applies (limits in 2.1); `deep_best`
  returns expressions over 3000 nodes unchanged, `best_form` over 4000.
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
  `−(d1 − d2 − d3)`); only the display chooses them again (1.5).
- **Numeric identity tests** (`fold_constants`, `same_function`, `Env::is_zero`) use a few
  perturbations in f64 (`Env::perturbed`: each symbol moved by up to 2.5% of its value, so a
  construction at scale 1e-5 or 1e4 is moved as one at scale 1), with tolerances relative to
  the values and a floor at the rounding of the expression's parts. A test against the sum of
  the parts alone would let a large cancelling formula pass with a part that varies.

### 2.5 Work clock calibration

The tick charges are a calibration, not a measurement. On the machine they were calibrated on,
the median rate across the suite was 5.3 million ticks per second of real time (nominal 5
million), with a 10%–90% range of 3.1–9.4 million and extremes of 1.6 and 12.5 million (not
measured again since). The modular products of the Hensel lifting (`PP::mul_t`) were not
charged at all; they now cost a tenth of a tick each. The normal forms looked at took between
0.6 and 1.4 times their work time. The real-time limit is the only bound for work
that polls and builds nothing for a long time, and a derivation stopped by it depends on load.
The whole suite's derivations take 16 s single-threaded in a release build (27 s before the
normal form, on the same machine); the slowest single derivation takes 1.1 s.

### 2.6 Ignored tests

- `alg::tests::factor_dump`, `derive_file` (`tests/examples.rs`): debugging helpers (section 3).
- Constructions with a closed form the derivation does not reach in its time:
  - `gen_conics::common_tangent_ellipse_circle`, `common_tangent_two_ellipses`: the four
    unknowns of the two contact points give a resultant of about 450 terms whose factoring runs
    out of time; with 15 s of extra time the result is exact but has 20,000 nodes, where the
    tangent line's slope is the root of a quartic.
  - `gen_conics::steiner_inellipse`: each tangency to a conic is a contact-point block with
    trigonometric atoms; the problem is linear in the dual conic, which nothing uses.
  - `gen_circle_tangency::malfatti_circles`: the last polynomial is an irreducible octic that
    is a tower of quadratics (Malfatti's formulas), not one of the shapes `poly_roots` solves.

## 3. Tools

Environment variables:
- `SGGEO_DEBUG=1`: per-block log of the derivation (blocks, equations, elimination steps,
  factor sizes, failure reasons, direct solving), the fit (`fit:` lines: atoms, bases, ranks,
  relations, check results), the normal form (`normal form:` sizes, time and ticks, and the
  reason of a failure), the stages after the blocks with real time and ticks (`stage`),
  and ticks against real time per strategy (`work:`).
- `SGGEO_DUMP=<file>`: write the last polynomial of a polynomial block, for `factor_dump`.
- `SGGEO_SIMPLIFY_FILE=<file>` with `cargo test --release --lib simplify_file -- --nocapture`:
  each line (`name = formula` or a formula) through `simplify_result`, with complexity before
  and after; `SGGEO_SIMPLIFY_FACTOR=1` uses `factor_sums` instead, `SGGEO_SIMPLIFY_RESUB=1`
  `resubstituted`, and `SGGEO_SIMPLIFY_CSE=1` also prints the named form the app shows. The
  parser multiplies a number into a sum (2(a + b) is read as 2a + 2b).
- `SGGEO_FACTOR_FILE=<file>` (with optional `SGGEO_MAIN=<var>|cheapest`): factor a dumped
  polynomial and report timings (`alg::tests::factor_dump`).
- `SGGEO_DERIVE_FILE=<file.sgp>` (with optional `SGGEO_DERIVE_LEVEL=<n>`): derive every reference
  dimension of a saved construction (`derive_file` in `tests/examples.rs`).

Each test run writes its construction to `test_examples/<suite>/<test>.sgp`, which opens in the
app (`cargo run --release -p sgpad -- <file>`).

Commands:

```sh
# (in the sggeo directory)
cargo test --no-fail-fast                                   # full suite
cargo test --lib                                            # unit tests (alg, expr, derive, nf, fit, dd)
timeout 300 cargo test --test gen_invariants ptolemy_diagonals -- --nocapture   # one test, prints formulas
SGGEO_DEBUG=1 timeout 300 cargo test --test gen_linkages peaucellier_inversor_rotating -- --nocapture
SGGEO_DERIVE_FILE=test_examples/gen_constructions/simson_line.sgp \
  cargo test --test examples derive_file -- --ignored --nocapture
SGGEO_DUMP=target/p.poly cargo test --test gen_invariants ptolemy_diagonals
SGGEO_FACTOR_FILE=target/p.poly SGGEO_MAIN=cheapest cargo test -p sggeo --lib alg::tests::factor_dump -- --ignored --nocapture
```

Run single tests under `timeout`: a stuck derivation otherwise blocks the run. Formulas do not
depend on machine load (section 1.7); times do, so measure them single-threaded
(`--test-threads=1`).
