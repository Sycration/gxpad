# sg

An interactive command line for sgpad's geometry. A line is a statement of the
[.sgp format](../docs/sgp-format.md), which sg adds to the document, or one of a few commands:
`?` derives a closed-form formula for what you measure.

```
sg> triangle ABC
  + A (2.29453, 1.93265)   B (-3.39705, 0.1415)   C (2.69275, -2.68125)
  + segment AB
  + segment BC
  + segment CA
  dof 6
sg> AB ⟂ CA
  dof 5
sg> b = 3
  + b = 3
  dof 4
sg> d1: |AB| = b
  + d1 = 3   driving
  dof 3
sg> d2: |CA| = 4
  + d2 = 4   driving
  dof 2
sg> l1 = bisect(AB, AC)
sg> l2 = bisect(BA, BC)
sg> I = meet(l1, l2)
sg> ? dist(I, AB)
  + d3 = 1   measured
  d3 = (b*d2)/(b + sqrt(b^2 + d2^2) + d2)
    ≈ 1   exact
sg> code py
  import math
  d3 = (b * d2) / (b + math.sqrt(b**2 + d2**2) + d2)
sg> save incircle.sgp
```

```sh
cargo run --release -p sgcli                 # interactive
cargo run --release -p sgcli -- file.sgp     # open a document
sg < script                                  # run lines from a file
sg -c 'triangle ABC' -c 'a1: ∠ABC' -c 'a2: ∠BCA' -c 'a3: ∠CAB' -c '? s = a1 + a2 + a3'
```

At the prompt, lines can be edited and arrows go through the history of earlier lines, which is
kept in `~/.sg_history`. Ctrl-C drops the line being typed, Ctrl-D quits.

`ls` shows the document as the .sgp file `save` writes, and `help` lists the commands. sg reads
statements with sggeo (`ops::apply_sgp`), the reader of .sgp files, so whatever a file can say
sg can say, one line at a time; a line that fails changes nothing, and `undo` takes one back.
