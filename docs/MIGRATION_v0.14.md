# Migration Guide — v0.14.0

This guide is for downstream users of perl-lsp crates who are upgrading to v0.14.0.

## What's changing in v0.14.0

v0.14.0 is a **clean break** release. The published crate count drops from 132 to **30**.
Approximately 100 product-internal microcrates stop being published. Their code moves
into subfolder modules inside the owning published crate. There are no bridge crates
or re-export shims — old crate names will no longer appear on crates.io after this release.

This is a deliberate one-time cost to eliminate a permanent operational burden. See
[ADR-0041](adr/0041-microcrate-collapse.md) for the full rationale.

## If you depend on perl-lsp-rs, perl-parser, perl-dap, or perllsp

**No change.** These four product crates survive the collapse unchanged and continue to be
published under the same names. Their public APIs are not affected by the internal module
reorganization.

The same applies to the other 26 crates in the published set:

- `tree-sitter-perl-c`, `tree-sitter-perl-rs`
- `perl-parser-pest`
- `perl-lexer`, `perl-token`, `perl-line-index`, `perl-uri`, `perl-pod`
- `perl-diagnostic-catalog`
- `perl-lsp-protocol`, `perl-content-length-framing`
- `perl-semantic-analyzer`, `perl-module`, `perl-workspace-index`
- `perl-symbol`
- `perl-lsp-perltidy`
- `perl-corpus`, `perl-tdd-support`, `perl-test-must`, `perl-test-generators`
- `perl-feature-catalog`, `perl-incremental-parsing`, `perl-refactoring`
- `perl-dead-code`, `perl-heredoc-anti-patterns`, `perl-path-security`

## If you depend on a retired crate

If your `Cargo.toml` lists a dependency on any crate not in the list above, that crate
has been retired. Its code now lives as a module inside one of the 30 published crates.

**Steps to migrate:**

1. Find the retired crate name in the migration table below.
2. Replace the `Cargo.toml` dependency line with the new owning crate.
3. Update import paths: `use perl_lsp_folding::` becomes `use perl_lsp::folding::` (example).

### Migration table (placeholder)

The full crate-by-crate retired→new-module-path table will be filled in as each wave PR
merges during the collapse. The collapse runs across ~14 PRs over several weeks; each PR
updates this table with the crates it absorbs.

Track progress at [tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410).

| Retired crate | New owning crate | New module path | Wave PR |
|---------------|-----------------|-----------------|---------|
| *(populated as wave PRs merge)* | | | |

Once all waves land, this table will list all ~100 retired crates with their exact new
module paths. Subscribe to issue #4410 for updates.

## Why

The microcrate architecture delivered on agent-friendly work units but did not deliver on
decoupled versioning, smaller publish surface, or faster compile times. The fundamental
constraint is that crates.io forbids path-only dependencies in published crates, so every
internal architectural seam expressed as a crate boundary became a permanent public
artifact and a semver contract. The full analysis is in
[ADR-0041](adr/0041-microcrate-collapse.md).

## Timeline

- The collapse is running now, before the next release ships.
- v0.14.0 will be the first release with the new 30-crate surface.
- Each wave PR updates this migration table as it lands.
- There is no extended migration window — old crate names are not re-published as shims.

If you have questions, open a discussion on
[tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410).
