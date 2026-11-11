/* Loads a .gxp file, solves it and prints the formula of every dimension.
 *
 *   cargo build -p gxgeo --release
 *   cc -Iinclude examples/c/derive.c -L../target/release -lgxgeo -o derive
 *   LD_LIBRARY_PATH=../target/release ./derive tests/law_of_cosines.gxp
 */
#include "gxgeo.h"
#include <stdio.h>
#include <stdlib.h>

static char *read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    rewind(f);
    char *s = malloc(n + 1);
    if (s && fread(s, 1, n, f) == (size_t)n) s[n] = 0; else { free(s); s = NULL; }
    fclose(f);
    return s;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s FILE.gxp\n", argv[0]);
        return 2;
    }
    char *text = read_file(argv[1]);
    if (!text) {
        perror(argv[1]);
        return 1;
    }
    GxDoc *doc = gx_doc_from_gxp(text);
    free(text);
    if (!doc) {
        fprintf(stderr, "%s\n", gx_last_error());
        return 1;
    }
    size_t dof;
    printf("gxgeo %s: solve %d, dof %zu\n", gx_version(), gx_doc_solve(doc, &dof), dof);
    for (size_t i = 0; i < gx_doc_dim_count(doc); i++) {
        GxFormula *f = gx_derive(doc, GX_TARGET_DIM, i, 0);
        if (!f) {
            printf("dim %zu: %s\n", i, gx_last_error());
            continue;
        }
        printf("%s\n  latex: %s\n  exact %d, value %.12g, measured %.12g\n", f->plain, f->latex, f->exact, f->value, f->actual);
        gx_formula_free(f);
    }
    gx_doc_free(doc);
    return 0;
}
