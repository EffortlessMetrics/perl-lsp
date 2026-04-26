# Parser Ratchet Scope Selection

This document defines **scope selection only** for Parser Ratchet. It does **not** run parser corpus work and does **not** implement parser comparators.

## Goal

Parser Ratchet should always emit a receipt, while only selecting expensive parser work when parser behavior is plausibly affected.

## Policy file

Policy source: `.ci/scope.d/parser-ratchet.toml`.

Selection is `selected=true` when either of these is true:

1. Any parser-relevant path pattern matches.
2. Any parser-relevant risk tag is present.

Otherwise, the receipt must report `selected=false` with a non-selection reason.

## Selected paths

- `crates/perl-parser/**`
- `crates/perl-parser-core/**`
- `crates/perl-lexer/**`
- `crates/perl-token/**`
- `crates/tree-sitter-perl-rs/**`
- `crates/tree-sitter-perl-c/**`
- `xtask/src/tasks/*parser*`
- `xtask/src/tasks/*corpus*`
- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `xtask/src/tasks/ratchet*.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.ci/parser-ratchet/**`
- `.github/workflows/**`
- `tests/parser/**`
- `tests/perl-corpus/**`
- `Cargo.lock` (when parser-relevant dep movement is indicated by parser-related tags/signals)

## Selected risk tags

- `parser`
- `lexer`
- `token`
- `corpus`
- `incremental`
- `tree-sitter`
- `parser-recovery`
- `parser-accuracy`

## Non-parser no-op examples

Expected `selected=false`:

- docs-only updates outside parser docs
- VS Code extension-only updates (unless workflow/scope policy files change)
- DAP-only updates
- editor docs
- forensics docs

## Fixture coverage

Fixtures under `xtask/tests/fixtures/ci-scope/parser-ratchet/` cover:

- docs-only fixture → `selected:false`
- parser crate change → `selected:true`
- lexer/token change → `selected:true`
- `ci_scope.rs` change → `selected:true` (scope-meta)
- workflow change → `selected:true` (policy-defined)
