//! The help pages.

const OVERVIEW: &str = r#"
gx builds a construction from the statements of a .gxp file, one per line,
solves it, and derives closed-form formulas for what you measure in it.

  triangle ABC           points are made where they are first named
  d1: |AB| = 4           a dimension: with a value it drives the shape
  d2: |BC| = 3
  d3: |CA| = 5
  ? ∠BAC                 derive: θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))

Uppercase names are points (A, P2, Fa) and run together: AB is the line
through A and B. Curves, variables and dimensions are lowercase (k1, s, d1).

Every line is one step: `undo` takes it back, and a line that fails changes
nothing. `ls` shows the document as it would be saved.

  help make        points, segments, circles, conics, curves
  help construct   midpoints, crossings, perpendiculars, bisectors...
  help constrain   constraints: perpendicular, on, tangent...
  help measure     dimensions and variables
  help derive      formulas
  help doc         listing, editing, files

The statements are the .gxp format, which docs/gxp-format.md describes.
"#;

const MAKE: &str = r#"
  point A B C            free points
  point P on k           a point on curve k
  segment AB             a segment (also `segment A B`); makes A and B if new
  line AB                an infinite line
  l = segment AB         a segment with a name of its own
  triangle ABC           the points and the sides AB, BC, CA
  polygon ABCD           ... closing the polygon
  k = circle(A)          a circle around A (`d: r(k) = 2` sets its radius)
  w = arc(O, A, B)       an arc around O, counterclockwise from A to B
  e = ellipse(O)         a conic centered on O; also hyperbola(O)
  p = parabola(V)        a parabola with vertex V
  g = curve(y = x^2)     a function curve; also curve(x^2 + y^2/4 = 1)

  A (2, 1)               place a new point, or drag one that is there
  segment AB [aux]       a helper, drawn dashed; [hidden] hides

Positions only start the solving: the constraints and dimensions decide the
shape, and the positions pick which solution you get (which side, which
crossing). New points are spread out so nothing is on a line by chance.
"#;

const CONSTRUCT: &str = r#"
  M = mid(A, B)          midpoint of A and B; mid(AB) of the segment AB
  P = meet(a, b)         where curves a and b cross (place P to pick a crossing)
  Q = reflect(P, l)      mirror image of P in line l
  T = tangent(P, k)      where a line from P touches k (then: segment PT)
  F = focus(e)           a focus of a conic
  l = perp(AB, P)        the line through P perpendicular to AB
  l = par(AB, P)         ... parallel to AB
  l = bisect(AB, AC)     the bisector of the angle at A between the rays to B
                         and C; bisect(AB, CD, X) at X when they share no point
  l = asymptote(h, 1)    an asymptote of a hyperbola (1 or -1)
  l = directrix(e, F)    the directrix of a conic for its focus F

Directions: AB runs from A to B, BA back; a named line l runs as drawn,
-l the other way.
"#;

const CONSTRAIN: &str = r#"
  AB ⟂ CD     AB perp CD      perpendicular
  AB ∥ CD     AB par CD       parallel
  AB coll CD                  on one line
  |AB| == |CD|                equal lengths; r(k) == r(m) equal radii
  A == B                      the same point
  P on k                      P lies on k
  M == mid(A, B)              M is the midpoint; also reflect, tangent, focus
  a tan b                     tangent; `inside` for a circle in a circle
  k conc m                    concentric
  horiz A B    vert A B       level, one above the other

A constraint that cannot hold, or that the others already imply, is rejected
and changes nothing.
"#;

const MEASURE: &str = r#"
A dimension is a name, a colon and a measure; `= value` makes it drive:

  d1: |AB| = 5           length (len(l) for a named line)
  d2: dist(P, AB)        distance to a line; dist(A, B) between points
  θ1: ∠ABC = 30°         angle at B (numbers are degrees)
  θ2: angle(AB, CD)      between directions; angle(AB, DC) is the supplement
  dx(A, B)  dy(A, B)  x(P)  y(P)  r(k)  diam(k)  a(e)  b(e)

  d1 = 6                 a new value for a dimension that is there
  d1 = s/2               ... or an expression of variables and dimensions
  unset d1               measure it instead

Variables:
  s = 3                  an input
  s ~ 3                  solved for by the constraints, starting at 3
  q = d1/d2              defined by an expression
  d1: |AB| = 2 [numeric] formulas use its value instead of its name

In expressions: + - * / ^ sqrt sin cos tan asin acos atan exp ln abs, pi, e.
Angles inside expressions are in radians.
"#;

const DERIVE: &str = r#"
  ? |AC|            the formula of a measure, from the driving dimensions and
                    the variables (making a measured dimension for it)
  ? d3              ... of a dimension or variable by name
  ? h: dist(A, BC)  ... of a new dimension named h
  ? q = d1/d2       ... of a new variable q
  ??                search longer for the last one (10 s more each time)

  code LANG         the last formula as code: c rust js py mma maxima
  latex             the last formula as LaTeX
  png [FILE]        the last formula typeset as an image

A formula is found while the construction still moves (dof > 0) when the
measure does not depend on how it moves: the angle sum of a triangle, say.
"#;

const DOC: &str = r#"
  ls  [all]         the document as a .gxp file (`all` with the positions)
  plot [W H]        draw the construction (W×H characters)
  del X Y           delete points, curves (AB for a segment), dimensions
                    and variables, with what depends on them
  name OLD NEW      rename a point, curve, dimension or variable
  undo [N]  redo [N]
  new               start over
  open FILE.gxp     open a document; open FILE runs the lines of FILE
  run FILE          run the lines of FILE
  save [FILE.gxp]   save the document
  quit              (or Ctrl-D)

At the prompt, arrows go through earlier lines (kept in ~/.gx_history) and
Ctrl-C drops the line being typed.

`# text` starts a comment.
"#;

pub fn help(topic: &str) -> Vec<String> {
    let t = match topic.trim() {
        "" => OVERVIEW,
        "make" | "points" | "point" | "segment" | "line" | "triangle" | "polygon" | "circle" | "arc" | "ellipse" | "hyperbola" | "parabola" | "curve" => MAKE,
        "construct" | "build" | "mid" | "meet" | "reflect" | "tangent" | "focus" | "perp" | "par" | "bisect" | "asymptote" | "directrix" => CONSTRUCT,
        "constrain" | "constraints" | "on" | "tan" | "conc" | "coll" | "horiz" | "vert" => CONSTRAIN,
        "measure" | "measures" | "dims" | "dimensions" | "variables" | "vars" | "unset" => MEASURE,
        "derive" | "formula" | "formulas" | "?" | "code" | "latex" | "png" => DERIVE,
        "doc" | "ls" | "plot" | "del" | "name" | "undo" | "save" | "open" | "run" | "files" => DOC,
        _ => return vec![format!("no help on {topic}: try help, help make, help construct, help constrain, help measure, help derive, help doc")],
    };
    t.trim_matches('\n').lines().map(String::from).collect()
}
