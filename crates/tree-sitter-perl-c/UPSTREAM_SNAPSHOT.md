# tree-sitter-perl-c Upstream Snapshot

This document is the canonical provenance + refresh log for the vendored C
snapshot in `crates/tree-sitter-perl-c/c-src/`.

## Current vendored snapshot

- **Upstream repository:** <https://github.com/tree-sitter-perl/tree-sitter-perl>
- **Upstream branch/reference policy:** `master` (refreshes are pulled from upstream
  default branch unless a specific release/tag is requested)
- **Pinned upstream commit:** **LEGACY-UNKNOWN** for the currently vendored
  baseline introduced in `c57aadcba61cf295c6abc2b2a9c85cdf13de9cbb` on
  **2026-04-23**.
- **tree-sitter generator version (from `c-src/parser.c` header):** `v0.25.9`

### Snapshot fingerprints (current baseline)

Use these to verify the exact C snapshot currently shipped by this crate.

```text
c-src/parser.c                    sha256 07b7bb23511188e97cfdbd6ac6289439f872e58504d2c51d9e24e59fae957d2a
c-src/scanner.c                   sha256 01bbea22f0864679692fb0163b29304d346666f2181b1dbbe08f900c9bb219eb
c-src/bsearch.h                   sha256 cb08206e89750c1fab700b89fc9876afb5cc689827e514ef49a5569c54635b61
c-src/tsp_unicode.h               sha256 9cbc0731f8c9bd52bd3de9644fd887f20cecea2d17634b20769db4940eadb566
c-src/tree_sitter/parser.h        sha256 2d5f15e194c8f52f96645ebf9e647f322f6eb8f8daa7135af79f76bf0ec77fcd
c-src/tree_sitter/array.h         sha256 d230d6f16f045be8fddf6f69a78f38db37f6db49fce4df410d11f7d1655ab0be
c-src/tree_sitter/alloc.h         sha256 b83867de8b96f30f2bbf2ca30cdf9f8f7efff761f4f2fccc743789aec98bdd95
```

## Vendored vs local files

### Vendored from upstream grammar snapshot

- `c-src/parser.c`
- `c-src/scanner.c`
- `c-src/bsearch.h`
- `c-src/tsp_unicode.h`
- `c-src/tree_sitter/parser.h`
- `c-src/tree_sitter/array.h`
- `c-src/tree_sitter/alloc.h`

### Local wrapper/runtime integration (this repository)

- `build.rs`
- `src/lib.rs`
- `src/bin/*`
- `tests/*`
- `README.md`, `ROADMAP.md`, this file

## Refresh procedure

1. **Select upstream ref**
   - Choose a concrete upstream commit/tag from
     `https://github.com/tree-sitter-perl/tree-sitter-perl`.
   - Record it in this file before opening the PR.
2. **Regenerate parser artifacts**
   - In an upstream checkout at that commit, run:
     - `tree-sitter --version` (record exact version here)
     - `tree-sitter generate`
3. **Copy vendored files into this crate**
   - Copy `src/parser.c`, `src/scanner.c`, `src/bsearch.h`, `src/tsp_unicode.h`.
   - Copy `src/tree_sitter/{parser.h,array.h,alloc.h}`.
4. **Update metadata in this file**
   - Update pinned commit/tag and all SHA-256 fingerprints.

## Refresh validation checklist

Run these before commit/PR:

- [ ] Build check: `cargo check --all-targets -p tree-sitter-perl-c`
- [ ] Crate tests: `cargo test -p tree-sitter-perl-c`
- [ ] Query conformance:
  `cargo test -p tree-sitter-perl-c bdd_query_parsing_succeeds_for_injections_query`
- [ ] Benchmark sanity (non-regression smoke):
  `cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- <sample.pl>`

## Provenance hardening follow-up

The current baseline was imported before provenance metadata was added. On the
next snapshot refresh, replace `LEGACY-UNKNOWN` with the exact upstream commit
or release tag and keep that value immutable for future audits.
