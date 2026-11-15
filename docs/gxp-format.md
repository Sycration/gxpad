# The .gxp format

A `.gxp` file states a construction the way a geometry problem is stated: what is **given**, what
is **constructed** from it, and what is to be **found**. A last section, **where**, gives one
solution of it: the positions the points had when the file was saved.

```
gxp 2

given
  b = 4.925937527486427
  c = 4.925598907762098
  triangle ABC
    AB ⟂ CA
    d1: |AB| = b
    d2: |CA| = c

construct
  l1 = bisect(AB, AC)
  l2 = bisect(BA, BC)
  I = meet(l1, l2)
  k1 = circle(I)
    BC tan k1
  l3 = perp(BC, I) [aux]
  T = meet(l3, BC)

find
  d3: dist(I, AB)

where
  A (1.3718735075214266, -0.27422240841117557)
  B (5.715466296019598, 2.0491506163968887)
  C (-0.951339803541024, 4.069071792014283)
  I (1.963561838301006, 1.6784195488352145)
  k1 r 1.4427241063314458
  T (2.381901210389936, 3.0591602981252866)
```

This is a right triangle with legs `b` and `c`, its incenter `I` (where two angle bisectors
meet), a circle around `I` touching `BC`, and the foot `T` of the perpendicular from `I` to `BC`.
The quantity to find is the distance from `I` to `AB`, the inradius.

## Lines

Every line is one statement. Text after `#` is a comment. Indentation does not matter: gxpad
indents the constraints and dimensions of a thing under the statement that makes it, but every
line stands on its own. Sections (`given`, `construct`, `find`, `where`) are headings only: a
statement means the same in any of them. The first line is `gxp 2`, the version of the format.

A statement can only use what earlier lines made, except that the `where` section is read first,
so every point starts where it says.

Comments are not kept: gxpad writes the file anew when it saves.

## Names

- **Points** have names like `A`, `P2`, `Fa`: an uppercase letter, then lowercase letters, digits
  or `_`. Such names can run together: `AB` is the line through A and B, `∠ABC` an angle at B.
  A point can have another name (`apex`), but then it cannot run together with others.
- **Curves** (circles, arcs, conics, function curves, construction lines) have lowercase names:
  `k1`, `l2`, `e1`. A segment or line between two named points needs no name: it is `AB`.
- **Variables and dimensions** have names of their own (`b`, `d1`, `θ1`, `x_A`). They never
  stand where a point or curve does, so a variable `A` and a point `A` can both be.
- The origin is `O` and the axes are `xaxis` and `yaxis`; every document has them.

Keywords cannot name a point or curve: `point segment line triangle polygon horiz vert on tan conc
coll perp par origin given construct find where gxp xaxis yaxis`.

### Lines with a direction

Some statements care which way a line runs. `AB` runs from A to B, and `BA` is the same line
from B to A. A named line `l` runs the way it was drawn, and `-l` the other way.

## Given

| Statement | Makes |
| --- | --- |
| `point A B C` | free points |
| `segment AB` | the segment from A to B (also `segment A B`); makes A and B when they are new |
| `line AB` | the infinite line through A and B |
| `l = segment AB`, `l = line AB` | the same, with a name |
| `triangle ABC`, `polygon ABCD` | the points and the sides AB, BC, CA (closing the polygon) |
| `k = circle(A)` | a circle around A |
| `w = arc(O, A, B)` | an arc around O, counterclockwise from A to B |
| `e = ellipse(O)`, `h = hyperbola(O)` | a conic centered on O |
| `p = parabola(V)` | a parabola with vertex V |
| `g = curve(y = x^2)` | a function curve; also implicit: `curve(x^2 + y^2/4 = 1)` |
| `s = 3` | a variable, an input of the construction |
| `s ~ 3` | a variable the constraints solve for, starting at 3 |

## Construct

| Statement | Makes |
| --- | --- |
| `M = mid(A, B)` | the midpoint of A and B |
| `M = mid(AB)` | the midpoint of the segment AB |
| `P = meet(a, b)` | a point where curves a and b cross; `where` picks which crossing |
| `point P on k` | a point on curve k |
| `Q = reflect(P, l)` | the mirror image of P in line l |
| `T = tangent(P, k)` | the point where a line from P touches k (draw it with `segment PT`) |
| `F = focus(e)` | a focus of a conic |
| `l = perp(AB, P)` | the line through P perpendicular to the direction AB |
| `l = par(AB, P)` | the line through P parallel to AB |
| `l = bisect(AB, AC)` | the bisector of the angle between the directions AB and AC, at their shared point; `bisect(AB, CD, X)` at X when they share none |
| `l = asymptote(h, 1)`, `asymptote(h, -1)` | an asymptote of a hyperbola |
| `l = directrix(e, F)` | the directrix of a conic belonging to its focus F |

The direction of `perp` and `par` is two points (`AB`, with no line needed between them) or a
line (`l`, `-l`). In `bisect`, the directions pick the angle: `bisect(AB, AC)` halves the angle
at A between the rays to B and C, `bisect(AB, CA)` the angle beside it.

## Constraints

| Statement | Means |
| --- | --- |
| `A == B` | the same point |
| `P on k` | P lies on k |
| `AB ⟂ CD`, `AB perp CD` | perpendicular |
| `AB ∥ CD`, `AB par CD` | parallel |
| `AB coll CD` | on one line |
| `\|AB\| == \|CD\|`, `r(k) == r(m)`, `\|AB\| == r(k)` | equal lengths and radii (`len(l)` for a named line) |
| `k conc m` | concentric |
| `a tan b` | tangent, on the side they are on in `where`; `inside` for a circle inside another; `near (x, y)` where the contact of two other curves is |
| `horiz A B`, `vert A B` | A and B level, one above the other |
| `M == mid(A, B)`, `M == mid(AB)` | M is a midpoint |
| `Q == reflect(P, l)` | Q is the mirror image of P |
| `T == tangent(P, k)` | a line from P touches k at T |
| `F == focus(e)` | F is a focus of e |
| `l == directrix(e, F)` | l is the directrix of e for its focus F |

What a construction statement makes, a constraint can state about a point that is already there.
gxpad writes the construction when it can and the constraint when the point came first. A few
more forms (`Q == offset(P, A, B, 90°)`, `Q == bisect(AB, AC, X)`, `Q == asymptote(h, 1)`,
`A B == directrix(e, F)`) only appear when a construction line was deleted and its hidden helper
point kept.

## Dimensions

A dimension is a name, a colon and a measure. With `= value` it drives the construction (it is an
input); without, it is measured, and gxpad derives its formula.

```
d1: |AB| = 5          # length of segment AB
d2: |AB| = b/2        # ... driven by an expression of variables and dimensions
θ1: ∠BAC = 30°        # angle at A between the rays to B and C (numbers are degrees)
d3: dist(I, AB)       # measured
```

| Measure | |
| --- | --- |
| `\|AB\|`, `len(l)` | length of a segment |
| `dist(A, B)` | distance between points |
| `dist(P, l)` | distance from a point to a line |
| `dx(A, B)`, `dy(A, B)` | horizontal, vertical distance |
| `∠ABC` | angle at B between the rays to A and C |
| `angle(AB, CD)` | angle between the directions AB and CD: `angle(AB, DC)` is its supplement |
| `r(k)`, `diam(k)` | radius, diameter |
| `a(e)`, `b(e)` | half-axes of a conic (focal length of a parabola) |
| `x(P)`, `y(P)` | coordinates |

Expressions are in radians (`2*θ1`, `pi/6`); plain numbers for angles are degrees.

Variables defined by an expression (`q = d2/d1`) are written under `find`.

`[numeric]` after a dimension or variable makes formulas use its value instead of its name.

## Flags

`[aux]` after a statement draws what it makes dashed, as a helper; `[hidden]` hides it. Both:
`[aux, hidden]`. A line naming an existing thing and its flags sets them: `xaxis [hidden]`.
`origin P` renames the origin.

## Where

| Line | |
| --- | --- |
| `A (1.5, 2)` | position of a point |
| `k r 2.5` | radius of a circle |
| `e a 5 b 3 angle 30°` | half-axes and turn of a conic (a parabola has no `b`) |
| `d1 label (0.3, 0.3)` | where the label of a dimension sits, from its anchor |
| `[fixed]` | after a position, radius or conic: locked; `[fixed x]`, `[fixed angle]` lock some values |

The constraints decide the shape; the positions pick which of the shapes they allow is meant
(which side of a line, which crossing of two circles). gxpad writes them exactly, so a document
opens just as it was saved.

Everything in `where` is optional. A point without a position starts where its construction puts
it (a midpoint between its points, a crossing where the curves cross) or at a spot of its own,
spread out from the others; a circle starts with radius 1. The file is then solved, and the
result is one solution of the construction:

```
gxp 2
# the inradius of a 3-4-5 triangle
triangle ABC
  d1: |AB| = 4
  d2: |BC| = 3
  d3: |CA| = 5
l1 = bisect(AB, AC)
l2 = bisect(BA, BC)
I = meet(l1, l2)
r: dist(I, AB)
```

## Reading back

A file reads back as the same entities, constraints and dimensions, numbered in the order of the
file. gxpad orders a saved file so that every statement comes after what it uses: the given
things first, in the order they were made, then the constructions. A constraint or driving
dimension comes right under the last thing it involves.
