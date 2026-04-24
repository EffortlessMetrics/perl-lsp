# `tree-sitter-perl-c` upstream snapshot provenance

This document is the canonical provenance record for the vendored C grammar in
`crates/tree-sitter-perl-c/c-src/`.

## Current vendored snapshot

- **Upstream repository:** <https://github.com/tree-sitter-perl/tree-sitter-perl>
- **Upstream tracking branch:** `release`
- **Current snapshot commit:** _not recorded in historical import_
  - This snapshot was carried forward from the archived harness
    (`archive/crates/tree-sitter-perl-rs/src/`) before explicit provenance
    recording existed.
  - To keep the snapshot auditable despite the missing pin, we record content
    fingerprints below.
- **Generator version (`parser.c` header):** `tree-sitter v0.25.9`

### Snapshot fingerprints (current `c-src/`)

| File | SHA-256 |
|---|---|
| `c-src/parser.c` | `07b7bb23511188e97cfdbd6ac6289439f872e58504d2c51d9e24e59fae957d2a` |
| `c-src/scanner.c` | `01bbea22f0864679692fb0163b29304d346666f2181b1dbbe08f900c9bb219eb` |
| `c-src/bsearch.h` | `cb08206e89750c1fab700b89fc9876afb5cc689827e514ef49a5569c54635b61` |
| `c-src/tsp_unicode.h` | `9cbc0731f8c9bd52bd3de9644fd887f20cecea2d17634b20769db4940eadb566` |
| `c-src/tree_sitter/parser.h` | `180b893c8734778fd32f372dfbc27bd6ad1cd2221f26150b31256ff6716320d2` |
| `c-src/tree_sitter/array.h` | `5bdf6ed1a78e3409fd443e085ca967a64c188a5d082aaf7f819bccd53a471c94` |
| `c-src/tree_sitter/alloc.h` | `b29c1c9fb7cc82f58c84b376df1297d6e2737a1d655fd356db0859e3c29c2fea` |

## Refresh procedure (local maintainer workflow)

1. Pick an upstream commit SHA on the `release` branch and record it here.
2. Copy vendored files from upstream into `c-src/`:
   - `src/parser.c` -> `c-src/parser.c`
   - `src/scanner.c` -> `c-src/scanner.c`
   - `src/bsearch.h` -> `c-src/bsearch.h`
   - `src/tsp_unicode.h` -> `c-src/tsp_unicode.h`
   - `src/tree_sitter/parser.h` -> `c-src/tree_sitter/parser.h`
   - `src/tree_sitter/array.h` -> `c-src/tree_sitter/array.h`
   - `src/tree_sitter/alloc.h` -> `c-src/tree_sitter/alloc.h`
3. Recompute and update the SHA-256 fingerprint table:

   ```bash
   sha256sum \
     crates/tree-sitter-perl-c/c-src/parser.c \
     crates/tree-sitter-perl-c/c-src/scanner.c \
     crates/tree-sitter-perl-c/c-src/bsearch.h \
     crates/tree-sitter-perl-c/c-src/tsp_unicode.h \
     crates/tree-sitter-perl-c/c-src/tree_sitter/parser.h \
     crates/tree-sitter-perl-c/c-src/tree_sitter/array.h \
     crates/tree-sitter-perl-c/c-src/tree_sitter/alloc.h
   ```
4. Update **generator version** from line 1 of `c-src/parser.c`.
5. Ensure `README.md` / `ROADMAP.md` still point to this file.

## Refresh validation checklist

Run these checks before opening a refresh PR:

- [ ] Build / compile:
  - `cargo check --all-targets -p tree-sitter-perl-c`
- [ ] Tests:
  - `cargo test -p tree-sitter-perl-c`
- [ ] Query conformance (workspace highlight query path still works):
  - `cargo test -p tree-sitter-perl-c --test bdd_workflows`
- [ ] Benchmark sanity (no obvious regressions / failures):
  - `cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- <perl-file>`

## Ownership boundary

- **Vendored upstream grammar artifacts:** everything under `c-src/`.
- **Local wrapper / integration code:** `build.rs`, `src/lib.rs`, `src/bin/*`,
  `tests/*`, and this provenance document.
