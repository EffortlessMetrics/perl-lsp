# CLAUDE.md (perl-module)

## Role

Unified Perl module-resolution facade. Absorbs what were thirteen separate
`perl-module-*` microcrates into one published crate with internal module
folders, covering module naming, `use`/`require` modeling, path resolution,
reference extraction, and safe renaming.

## Owns

- `name` -- canonical module name parsing/normalization
  (`normalize_package_separator`, `legacy_package_separator`).
- `path` -- module name <-> filesystem path conversion.
- `import` -- `use`/`require` modeling (`ImportBehavior`, `LoadTiming`,
  `RequireForm`, `ModuleImportKind`).
- `import_match` -- candidate filtering for import-line matching.
- `reference` -- `ModuleReference` extraction (feeds go-to-definition /
  find-references).
- `rename` -- safe module/file rename edit planning
  (`plan_module_rename_edits`, `apply_module_rename_edits`).
- `resolution` -- the `@INC`-aware resolution pipeline (`IncRoot`,
  `IncRootKind`, `resolve_module_uri`, `resolve_module_path`).
- `token` / `token_core` / `token_parser` -- lexical helpers for
  module-syntax token boundaries.
- `api` -- curated public facade; consumers should import from the
  `perl_module` crate root only, never from submodules directly.

## Does not own

- Full Perl syntax parsing -- consumes token/AST types from
  `perl-parser-core`.
- Workspace-wide indexing -- delegates to `perl-workspace`.
- LSP provider wiring -- this crate is consumed by providers, not the other
  way around.

## Neighbors

- Upstream: `perl-parser-core`, `perl-workspace`, `url`.
- Downstream: `perl-lsp-rs-core`, `perl-dap`, `perl-lsp-rs`, `perl-parser`,
  `perl-refactoring`, `perl-semantic-analyzer`, `tree-sitter-perl-rs`.

## Read first

- `src/lib.rs` -- module map with doc comments.
- `src/api.rs` -- the entire public surface in one file; read this before
  the internals to know what's actually stable.
- `src/resolution.rs` -- the `@INC`/URI resolution pipeline most callers
  actually need.

## Focused validation

`cargo test -p perl-module`. Most internal modules have matching bdd / fuzz /
integration / property test quartets (e.g.
`module_resolution_{bdd,fuzz,integration,prop}.rs`) -- when changing a
module's public behavior, update all four kinds, not just the one you
noticed failing. `tests/facade_api_completeness.rs` guards against `api.rs`
drifting from the internal modules.

## Review hotspots

- `resolution.rs` -- `@INC` root ordering and kind semantics; subtle and
  high-consequence for go-to-definition correctness.
- `rename.rs` -- edit planning must stay conservative; a false-positive
  rename edit corrupts user code.
- `token` / `token_core` / `token_parser` -- three similarly-named modules
  with a narrow split of responsibility; easy to duplicate logic across them
  instead of reusing.

## Claim boundary

Describes module ownership as authored in `lib.rs`/`api.rs`. Does not assert
complete `@INC` semantics coverage against every real-world Perl
distribution/build-tool layout.
