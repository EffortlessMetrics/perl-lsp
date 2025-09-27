# Release Checklist for v0.8.3

## Pre-release verification
- [x] All tests pass: `cargo test --workspace`
- [x] No critical clippy warnings: `cargo clippy --workspace`
- [x] Documentation builds: `cargo doc --workspace --no-deps`
- [x] Benchmarks run: `cargo bench --workspace`

## Crate configuration (LOCKED - DO NOT CHANGE)
- [x] `tree-sitter-perl-rs`: publish = false (internal harness only)
- [x] `perl-parser-pest`: clearly marked as v2 experiment with migration guide
- [x] `perl-parser`: positioned as production v3 with Tree-sitter compatibility

## Documentation
- [x] Main README has crate selector table with TL;DR
- [x] STABILITY.md updated with MSRV 1.89 / Edition 2024  
- [x] Migration guide from v2 to v3 in perl-parser-pest README
- [x] Tree-sitter compatibility noted in perl-parser README and docs

## Discovery guarantees
- [x] `perl-parser` has `tree-sitter` keyword
- [x] `perl-parser-pest` has `legacy` keyword
- [x] Tree-sitter compatibility one-liner prominent in perl-parser README
- [x] package.metadata.docs.rs added to perl-parser-pest

## Publishing order (when ready)
1. [ ] `export CARGO_REGISTRY_TOKEN=your_token_here`
2. [ ] `(cd crates/perl-lexer && cargo publish)`
3. [ ] `sleep 30` (wait for crates.io indexing)
4. [ ] `(cd crates/perl-corpus && cargo publish)`
5. [ ] `sleep 30` (wait for crates.io indexing)
6. [ ] `(cd crates/perl-parser-pest && cargo publish)`
7. [ ] `sleep 30` (wait for crates.io indexing)
8. [ ] `(cd crates/perl-parser && cargo publish)`

## Post-release verification
- [ ] Run smoke test: `./scripts/smoke-test-release.sh`
- [ ] Verify crates.io listings show correct descriptions
- [ ] Test installation: `cargo install perl-parser --bin perl-lsp`
- [ ] Verify LSP works: `perl-lsp --version`
- [ ] Create GitHub release with binaries

## CI Guards (future)
- [ ] Add public API diff with cargo-public-api
- [ ] Add S-expr snapshot tests for golden corpus
- [ ] Move clippy to -D warnings after triage
