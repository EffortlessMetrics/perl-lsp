# Upstream Snapshot Provenance (`tree-sitter-perl-c`)

This file is the audit record for the vendored C grammar under `c-src/`.
Update it in the same commit whenever `c-src/` changes.

## Current vendored snapshot (as checked into this repo)

- Upstream repository: <https://github.com/tree-sitter-perl/tree-sitter-perl>
- Upstream tracking reference: `main` branch
- Upstream commit: **unknown (legacy import before provenance policy)**
- Parser generator: `tree-sitter v0.25.9` (from banner in `c-src/parser.c`)

### Artifact identity (current checkout)

Use these checksums to prove exactly what snapshot is vendored today:

- `c-src/parser.c`: `07b7bb23511188e97cfdbd6ac6289439f872e58504d2c51d9e24e59fae957d2a`
- `c-src/scanner.c`: `01bbea22f0864679692fb0163b29304d346666f2181b1dbbe08f900c9bb219eb`
- `c-src/tsp_unicode.h`: `9cbc0731f8c9bd52bd3de9644fd887f20cecea2d17634b20769db4940eadb566`
- `c-src/bsearch.h`: `cb08206e89750c1fab700b89fc9876afb5cc689827e514ef49a5569c54635b61`

> Next refresh requirement: replace the `unknown` commit with an exact upstream
> commit SHA (or release tag + commit SHA) and keep the checksum block updated.

## Ownership boundary

- **Vendored upstream files:** everything under `c-src/`
- **Local wrapper code:** `build.rs`, `src/lib.rs`, `src/bin/*`, crate docs

Policy: do not hand-edit `c-src/*` for local behavior changes. Fix upstream,
then refresh the snapshot.

## Refresh procedure (local)

1. Clone upstream and check out the target commit/tag.
2. Install/use the intended `tree-sitter` CLI version.
3. Regenerate parser artifacts upstream.
4. Copy the generated snapshot into this crate.
5. Update this file with upstream commit/tag, generator version, and new checksums.
6. Run the validation checklist below.

Reference command sequence:

```bash
# from repo root
TMP_DIR="$(mktemp -d)"
git clone https://github.com/tree-sitter-perl/tree-sitter-perl "$TMP_DIR"
cd "$TMP_DIR"
git checkout <upstream-commit-or-tag>

# pin generator for reproducibility
npx tree-sitter@0.25.9 generate

# copy generated snapshot into this crate
cp src/parser.c /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/parser.c
cp src/scanner.c /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/scanner.c
cp src/tsp_unicode.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/tsp_unicode.h
cp src/bsearch.h /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/bsearch.h
cp -r src/tree_sitter /workspace/perl-lsp/crates/tree-sitter-perl-c/c-src/
```

## Refresh validation checklist

Run these checks after updating `c-src/`:

- [ ] Build/check crate targets
  - `cargo check --all-targets -p tree-sitter-perl-c`
- [ ] Crate tests
  - `cargo test -p tree-sitter-perl-c`
- [ ] Query conformance against upstream query suite
  - in upstream checkout: `tree-sitter test`
- [ ] Benchmark sanity (no obvious regression spikes)
  - `cargo run -p tree-sitter-perl-c --bin bench_parser_c --features test-utils -- <sample.pl>`

If a check fails, do not publish the snapshot refresh until resolved or explicitly
explained in the PR.
