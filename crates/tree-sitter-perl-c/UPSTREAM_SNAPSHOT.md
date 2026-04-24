# Upstream Snapshot Provenance (`tree-sitter-perl-c`)

This file is the auditable record for the vendored C snapshot in `c-src/`.

## Current vendored snapshot

- **Upstream grammar repository:** `tree-sitter-perl/tree-sitter-perl`
  (`https://github.com/tree-sitter-perl/tree-sitter-perl`)
- **Upstream tracking ref for refreshes:** `master` (resolve to a specific commit
  at refresh time)
- **Generator version used by current `parser.c`:** `tree-sitter v0.25.9`
  (from the generated header comment in `c-src/parser.c`)
- **Import provenance in this repository:** snapshot introduced in commit
  `c57aadcba61cf295c6abc2b2a9c85cdf13de9cbb` by copying the archived C sources
  from `archive/crates/tree-sitter-perl-rs/src/` into `crates/tree-sitter-perl-c/c-src/`.

### Snapshot fingerprints (for audit/diff checks)

- `c-src/parser.c` SHA-256:
  `07b7bb23511188e97cfdbd6ac6289439f872e58504d2c51d9e24e59fae957d2a`
- `c-src/scanner.c` SHA-256:
  `01bbea22f0864679692fb0163b29304d346666f2181b1dbbe08f900c9bb219eb`
- `c-src/tsp_unicode.h` SHA-256:
  `9cbc0731f8c9bd52bd3de9644fd887f20cecea2d17634b20769db4940eadb566`
- `c-src/bsearch.h` SHA-256:
  `cb08206e89750c1fab700b89fc9876afb5cc689827e514ef49a5569c54635b61`

> **Important:** This snapshot predates explicit provenance tracking in this
> crate. During the next refresh, record the exact upstream commit SHA used to
> generate/copy `parser.c` and `scanner.c` in this file.

## What is vendored vs local

### Vendored from upstream grammar snapshot

- `c-src/parser.c` (generated parser)
- `c-src/scanner.c` (external scanner)
- `c-src/bsearch.h`
- `c-src/tsp_unicode.h`
- `c-src/tree_sitter/parser.h`
- `c-src/tree_sitter/array.h`
- `c-src/tree_sitter/alloc.h`

### Local wrapper/maintenance code in this crate

- `src/lib.rs` (FFI declaration + Rust API)
- `build.rs` (compile/link vendored C sources)
- `tests/` (Rust-level behavior/query integration tests)
- `src/bin/` (CLI helpers for parse and benchmark smoke checks)
- `README.md`, `ROADMAP.md`, and this file

## Refresh procedure

1. **Choose upstream commit/ref and record it first**

   ```bash
   UPSTREAM_REF=<commit-or-tag>
   ```

2. **Fetch upstream grammar sources**

   ```bash
   git clone https://github.com/tree-sitter-perl/tree-sitter-perl /tmp/tree-sitter-perl
   cd /tmp/tree-sitter-perl
   git checkout "$UPSTREAM_REF"
   ```

3. **Regenerate parser with pinned CLI (if `src/parser.c` is not committed upstream)**

   ```bash
   tree-sitter --version  # record exact version in this file
   tree-sitter generate
   ```

4. **Copy vendored C snapshot files into this crate**

   ```bash
   cp src/parser.c /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/parser.c
   cp src/scanner.c /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/scanner.c
   cp src/bsearch.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/bsearch.h
   cp src/tsp_unicode.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/tsp_unicode.h
   cp src/tree_sitter/parser.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/tree_sitter/parser.h
   cp src/tree_sitter/array.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/tree_sitter/array.h
   cp src/tree_sitter/alloc.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/tree_sitter/alloc.h
   ```

5. **Update this file**

   - Upstream repo + exact commit/tag
   - Generator version
   - New SHA-256 fingerprints
   - Date of refresh (UTC)

## Refresh validation checklist

Run these checks from repo root after refreshing:

- [ ] **Build / compile sanity**
  - `cargo check --all-targets -p tree-sitter-perl-c`
- [ ] **Tests**
  - `cargo test -p tree-sitter-perl-c`
- [ ] **Query conformance (injections/highlights behavior)**
  - `cargo test -p tree-sitter-perl-c bdd_injections_query`
- [ ] **Benchmark sanity (non-regression smoke)**
  - `cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- tree-sitter-perl/test/corpus/statements`

If any step fails, do not ship the snapshot update without documenting the delta.
