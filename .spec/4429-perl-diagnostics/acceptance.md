# Acceptance Criteria: #4429 — Wave E Perl Diagnostics Consolidation

## Core Criteria

- [ ] New crate `perl-diagnostics` created at `crates/perl-diagnostics/` (new directory, NOT a rename of `crates/perl-diagnostics-codes/`)
- [ ] Cargo.toml includes all required metadata (version 0.12.4, `edition = "2024"`, workspace settings, optional `serde` feature, required `serde_json` dep, docs config)
- [ ] Module structure correct: `src/lib.rs`, `src/api.rs`, `src/codes/mod.rs`, `src/types/mod.rs`, `src/catalog/mod.rs`
- [ ] All 3 old crate directories deleted: `perl-diagnostics-codes/`, `perl-lsp-diagnostic-catalog/`, `perl-lsp-diagnostic-types/`

## Source Migration

- [ ] `src/codes/mod.rs` contains complete content from `perl-diagnostics-codes/src/lib.rs` (canonical `DiagnosticSeverity`, `DiagnosticTag`, `DiagnosticCategory`, `DiagnosticCode`)
- [ ] `src/types/mod.rs` contains `Diagnostic` and `RelatedInformation` structs from `perl-lsp-diagnostic-types/src/lib.rs` ONLY; `DiagnosticSeverity` and `DiagnosticTag` enum definitions are REMOVED and replaced with `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};`
- [ ] `src/catalog/mod.rs` contains complete content from `perl-lsp-diagnostic-catalog/src/lib.rs` including the inline `#[cfg(test)] mod tests` block (4 tests from lines 169–205 of the original)
- [ ] All inter-crate imports updated to intra-module paths (e.g., `perl_diagnostics_codes::` → `crate::codes::`)
- [ ] `src/api.rs` uses explicit per-symbol re-exports (no wildcard re-exports — this is a locked pattern even where no collision currently exists)
- [ ] `src/api.rs` includes `DiagnosticCategory` in the `codes::` re-export list
- [ ] `src/lib.rs` declares modules and re-exports via `api` module

## Type Unification (orchestrator-locked — unify in this wave)

- [ ] `DiagnosticSeverity` defined exactly once, in `src/codes/mod.rs` (canonical)
- [ ] `DiagnosticTag` defined exactly once, in `src/codes/mod.rs` (canonical)
- [ ] `src/types/mod.rs` does NOT define `DiagnosticSeverity` or `DiagnosticTag` as enums — re-exports them via `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};`
- [ ] `Diagnostic` struct's `severity: DiagnosticSeverity` field resolves to the canonical `codes::DiagnosticSeverity` via the `types::` re-export
- [ ] `tests/type_unification.rs` exists and passes, proving `perl_diagnostics::codes::DiagnosticSeverity` and `perl_diagnostics::types::DiagnosticSeverity` are the same type (cross-path assignment compiles)
- [ ] Same for `DiagnosticTag` in `tests/type_unification.rs`

## Test Migration

- [ ] All 6 external test files migrated to `crates/perl-diagnostics/tests/`:
  - `codes_comprehensive_unit_tests.rs` (from perl-diagnostics-codes)
  - `codes_context_hint_tests.rs` (from perl-diagnostics-codes)
  - `codes_diagnostic_code_completeness.rs` (from perl-diagnostics-codes)
  - `catalog_coverage.rs` (from perl-lsp-diagnostic-catalog)
  - `catalog_context_hint_tests.rs` (from perl-lsp-diagnostic-catalog)
  - `types_comprehensive_unit_tests.rs` (from perl-lsp-diagnostic-types)
- [ ] All test imports updated: `use perl_*::` → `use perl_diagnostics::{codes,types,catalog}::`
- [ ] 4 inline tests from `perl-lsp-diagnostic-catalog/src/lib.rs:169-205` migrated to `src/catalog/mod.rs` as a `#[cfg(test)] mod tests` block
- [ ] New test file `tests/type_unification.rs` present and passing

## Consumer Updates

- [ ] `perl-lsp-code-actions`: Cargo.toml dependency updated (`perl-diagnostics-codes` → `perl-diagnostics`); source imports updated
- [ ] `perl-lsp-diagnostics`: Cargo.toml dependencies updated (2 → 1: remove `perl-diagnostics-codes` + `perl-lsp-diagnostic-types`, add `perl-diagnostics`); source imports updated. NOTE: `perl-lsp-diagnostics` stays as its own crate (Wave G1 scope, NOT absorbed in Wave E).
- [ ] `perl-lsp` (binary in `crates/perl-lsp/`): Cargo.toml dependencies updated (2 → 1: remove `perl-diagnostics-codes` + `perl-lsp-diagnostic-catalog`, add `perl-diagnostics`); source imports updated

## Workspace Integration

- [ ] Workspace `Cargo.toml` `[workspace] members` updated: 123 → 121 (removed 3, added 1)
- [ ] Workspace `Cargo.toml` `[workspace.dependencies]` updated: old 3 crates removed, new crate added
- [ ] Workspace `Cargo.toml` `[workspace.metadata.publish]` allowlist updated: 120 → 118 entries
  - Removed: `perl-diagnostics-codes`, `perl-lsp-diagnostic-catalog`, `perl-lsp-diagnostic-types`
  - Added: `perl-diagnostics`
- [ ] New crate positioned in Tier 3 of publish allowlist (with other LSP analysis crates)

## Layer-check Rule

- [ ] xtask layer-check configuration updated to forbid `perl-diagnostics` from depending on any `perl-lsp-*` crate
- [ ] `cargo xtask layer-check` passes
- [ ] (Sanity) Manually inducing a `perl-lsp-*` dependency in `crates/perl-diagnostics/Cargo.toml` causes layer-check to fail; reverted before merge

## Documentation

- [ ] New crate includes `README.md` explaining module structure (codes/types/catalog) and the type-unification pattern (canonical in `codes/`, re-exported via `types/`)
- [ ] Crate-root docstring in `src/lib.rs` documents the type unification
- [ ] All module documentation preserved or migrated from original crates

## Compilation & Verification

- [ ] `cargo build -p perl-diagnostics --release` succeeds
- [ ] `cargo test -p perl-diagnostics` passes (6 external test files + 4 inline catalog tests + 2 unification tests)
- [ ] `cargo test -p perl-lsp-code-actions --lib` passes
- [ ] `cargo test -p perl-lsp-diagnostics --lib` passes
- [ ] `cargo test --workspace --lib` passes with no regressions
- [ ] `cargo clippy --workspace` produces no new warnings in migrated code
- [ ] `cargo xtask fmt` produces no formatting issues
- [ ] `cargo xtask publish-closure` passes
- [ ] `cargo xtask layer-check` passes
- [ ] No broken doc links: `cargo doc -p perl-diagnostics --no-deps`

## Edge Cases (from Oppositional + Plan Review)

- [ ] No compile error "ambiguous reexports" in `api.rs` — explicit lists prevent collisions
- [ ] All callers of old crate names still find symbols via new module paths (no missing re-exports — including `DiagnosticCategory`)
- [ ] Feature flag `serde` works on types in the `codes` module; `serde_json` is unconditional (not behind a feature)
- [ ] Inline tests from `perl-lsp-diagnostic-catalog/src/lib.rs:169-205` migrated and passing
- [ ] `types::DiagnosticSeverity` and `codes::DiagnosticSeverity` are the same type (not separate enums) — verified by `tests/type_unification.rs`

## Final Verification

- [ ] Workspace members count exactly 121
- [ ] Publish allowlist count exactly 118
- [ ] No stray references to `perl-diagnostics-codes`, `perl-lsp-diagnostic-catalog`, or `perl-lsp-diagnostic-types` in source code
- [ ] Git status clean except for new crate and modifications to existing files
- [ ] Build succeeds: `cargo build -p perl-lsp-rs --release` (full LSP server build)
