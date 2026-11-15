# gx

An interactive command line for gxpad's geometry. A line is a statement of the
[.gxp format](../docs/gxp-format.md), which gx adds to the document, or one of a few commands:
`?` derives a closed-form formula for what you measure.

```
gx> triangle ABC
  + A (2.29453, 1.93265)   B (-3.39705, 0.1415)   C (2.69275, -2.68125)
  + segment AB
  + segment BC
  + segment CA
  dof 6
gx> AB ⟂ CA
  dof 5
gx> b = 3
  + b = 3
  dof 4
gx> d1: |AB| = b
  + d1 = 3   driving
  dof 3
gx> d2: |CA| = 4
  + d2 = 4   driving
  dof 2
gx> l1 = bisect(AB, AC)
gx> l2 = bisect(BA, BC)
gx> I = meet(l1, l2)
gx> ? dist(I, AB)
  + d3 = 1   measured
  d3 = (b*d2)/(b + sqrt(b^2 + d2^2) + d2)
    ≈ 1   exact
gx> code py
  import math
  d3 = (b * d2) / (b + math.sqrt(b**2 + d2**2) + d2)
gx> save incircle.gxp
```

```sh
cargo run --release -p gxcli                 # interactive
cargo run --release -p gxcli -- file.gxp     # open a document
gx < script                                  # run lines from a file
gx -c 'triangle ABC' -c 'a1: ∠ABC' -c 'a2: ∠BCA' -c 'a3: ∠CAB' -c '? s = a1 + a2 + a3'
```

At the prompt, lines can be edited and arrows go through the history of earlier lines, which is
kept in `~/.gx_history`. Ctrl-C drops the line being typed, Ctrl-D quits.

`ls` shows the document as the .gxp file `save` writes, and `help` lists the commands. gx reads
statements with gxgeo (`ops::apply_gxp`), the reader of .gxp files, so whatever a file can say
gx can say, one line at a time; a line that fails changes nothing, and `undo` takes one back.
