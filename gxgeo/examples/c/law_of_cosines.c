/* Builds the law of cosines construction from scratch (the law_of_cosines
 * test in tests/gen_example.rs), solves it, prints the derived formula of
 * the angle and writes the document as a .gxp file.
 *
 *   cargo build -p gxgeo --release
 *   cc -Iinclude examples/c/law_of_cosines.c -L../target/release -lgxgeo -o law_of_cosines
 *   LD_LIBRARY_PATH=../target/release ./law_of_cosines law_of_cosines.gxp
 */
#include "gxgeo.h"
#include <stdio.h>
#include <stdlib.h>

static GxDoc *doc;

/* Exits with the library's message when a call returning an index failed. */
static ptrdiff_t check(ptrdiff_t r, const char *what) {
    if (r < 0) {
        fprintf(stderr, "%s: %s\n", what, gx_last_error());
        gx_doc_free(doc);
        exit(1);
    }
    return r;
}

/* Exits with the library's message unless a status call returned 1. */
static void ok(int r, const char *what) {
    if (r != 1) {
        fprintf(stderr, "%s: %s\n", what, gx_last_error());
        gx_doc_free(doc);
        exit(1);
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s OUT.gxp\n", argv[0]);
        return 2;
    }
    doc = gx_doc_new();

    /* a triangle with one corner on the origin */
    size_t o = check(gx_doc_origin(doc), "origin");
    size_t b = check(gx_doc_add_point(doc, 4.0, 0.2), "point B");
    size_t c = check(gx_doc_add_point(doc, 1.0, 3.0), "point C");
    size_t ab = check(gx_doc_add_line(doc, o, b, false), "line OB");
    size_t bc = check(gx_doc_add_line(doc, b, c, false), "line BC");
    size_t ca = check(gx_doc_add_line(doc, c, o, false), "line CO");
    ok(gx_doc_constrain(doc, GX_CON_HORIZONTAL, ab, 0), "horizontal OB");

    /* side lengths 5, 4, 3, labels at (0.3, 0.3) from each side's midpoint */
    size_t sides[3] = {ab, bc, ca};
    const char *lengths[3] = {"5", "4", "3"};
    for (int k = 0; k < 3; k++) {
        size_t d = check(gx_doc_add_dim(doc, GX_DIM_LENGTH, sides[k], 0, 0.3, 0.3), "length");
        ok(gx_doc_dim_driving(doc, d), "length is determined already");
        ok(gx_doc_set_dim(doc, d, lengths[k]), "length value");
    }

    /* the angle at the origin, measured */
    size_t ang = check(gx_doc_add_dim(doc, GX_DIM_ANGLE, ab, ca, 0.3, 0.3), "angle");
    ok(gx_doc_set_dim_driving(doc, ang, false), "measured angle");

    size_t dof;
    ok(gx_doc_solve(doc, &dof), "solve");
    printf("solved, %zu degrees of freedom left\n", dof);

    GxFormula *f = gx_derive(doc, GX_TARGET_DIM, ang, 0);
    if (!f) {
        fprintf(stderr, "derive: %s\n", gx_last_error());
        gx_doc_free(doc);
        return 1;
    }
    printf("%s\n  latex: %s\n  exact %d, value %.12g, measured %.12g\n", f->plain, f->latex, f->exact, f->value, f->actual);
    gx_formula_free(f);

    char *text = gx_doc_to_gxp(doc);
    FILE *out = text ? fopen(argv[1], "w") : NULL;
    if (!out || fputs(text, out) < 0 || fclose(out) != 0) {
        perror(argv[1]);
        gx_string_free(text);
        gx_doc_free(doc);
        return 1;
    }
    printf("wrote %s\n", argv[1]);
    gx_string_free(text);
    gx_doc_free(doc);
    return 0;
}
