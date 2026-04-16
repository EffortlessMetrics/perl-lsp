# Acceptance Criteria: #4429 — Wave E Perl Diagnostics Consolidation

## Core Criteria

- [ ] New crate `perl-diagnostic-catalog` created at `crates/perl-diagnostic-catalog/`
- [ ] Cargo.toml includes all required metadata (version 0.12.4, workspace settings, serde feature, docs config)
- [ ] Module structure correct: `src/lib.rs`, `src/api.rs`, `src/codes/mod.rs`, `src/types/mod.rs`, `src/catalog/mod.rs`
- [ ] All 3 old crate directories deleted: `perl-diagnostics-codes/`, `perl-lsp-diagnostic-catalog/`, `perl-lsp-diagnostic-types/`

## Source Migration

- [ ] `src/codes/mod.rs` contains complete content from `perl-diagnostics-codes/src/lib.rs`
- [ ] `src/types/mod.rs` contains complete content from `perl-lsp-diagnostic-types/src/lib.rs`
- [ ] `src/catalog/mod.rs` contains complete content from `perl-lsp-diagnostic-catalog/src/lib.rs`
- [ ] All inter-crate imports updated to intra-module paths (e.g., `perl_diagnostics_codes::` → `crate::codes::`)
- [ ] `src/api.rs` uses explicit per-symbol re-exports (no wildcard re-exports)
- [ ] `src/lib.rs` declares modules and re-exports via api module

## Test Migration

- [ ] All 6 test files migrated to `crates/perl-diagnostic-catalog/tests/`:
  - `codes_comprehensive_unit_tests.rs` (from perl-diagnostics-codes)
  - `codes_context_hint_tests.rs` (from perl-diagnostics-codes)
  - `codes_diagnostic_code_completeness.rs` (from perl-diagnostics-codes)
  - `catalog_coverage.rs` (from perl-lsp-diagnostic-catalog)
  - `catalog_context_hint_tests.rs` (from perl-lsp-diagnostic-catalog)
  - `types_comprehensive_unit_tests.rs` (from perl-lsp-diagnostic-types)
- [ ] All test imports updated: `use perl_*::` → `use perl_diagnostic_catalog::{codes,types,catalog}::`
- [ ] All inline tests (if any) in original source files migrated to test files

## Consumer Updates

- [ ] `perl-lsp-code-actions`: Cargo.toml dependency updated; source imports updated
- [ ] `perl-lsp-diagnostics`: Cargo.toml dependencies updated (2→1); source imports updated
- [ ] `perl-lsp`: Cargo.toml dependencies updated (2→1); source imports updated

## Workspace Integration

- [ ] Workspace `Cargo.toml` [workspace] members updated: 122 → 120 (removed 3, added 1)
- [ ] Workspace `Cargo.toml` [workspace.dependencies] updated: old 3 crates removed, new crate added
- [ ] Workspace `Cargo.toml` [workspace.metadata.publish] allowlist updated: 120 → 118 entries
  - Removed: `perl-diagnostics-codes`, `perl-lsp-diagnostic-catalog`, `perl-lsp-diagnostic-types`
  - Added: `perl-diagnostic-catalog`
- [ ] New crate positioned in Tier 3 of publish allowlist (with other LSP analysis crates)

## Documentation

- [ ] New crate includes `README.md` explaining structure and type duplication resolution path
- [ ] Inline docs note that `codes::DiagnosticSeverity` is canonical; `types::DiagnosticSeverity` marked deprecated in favor of future unification (v0.15.0)
- [ ] All module documentation preserved or migrated from original crates

## Compilation & Verification

- [ ] `cargo build -p perl-diagnostic-catalog --release` succeeds
- [ ] `cargo test -p perl-diagnostic-catalog --lib` passes (all 6 test files)
- [ ] `cargo test -p perl-lsp-code-actions --lib` passes
- [ ] `cargo test -p perl-lsp-diagnostics --lib` passes
- [ ] `cargo test --workspace --lib` passes with no regressions
- [ ] `cargo clippy --workspace` produces no new warnings in migrated code
- [ ] `cargo xtask fmt` produces no formatting issues
- [ ] No broken doc links: `cargo doc -p perl-diagnostic-catalog --no-deps`

## Type Duplication Handling

- [ ] Both `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` present (intentional; semantic duplicates)
- [ ] Both `codes::DiagnosticTag` and `types::DiagnosticTag` present (intentional; semantic duplicates)
- [ ] API re-exports both explicitly (not via wildcards) to avoid ambiguity compile error
- [ ] README documents duplication and notes unification will occur in v0.15.0

## Edge Cases (from Oppositional Review)

- [ ] No compile error "ambiguous reexports" when re-exporting both `codes::DiagnosticSeverity` and `types::DiagnosticSeverity`
- [ ] All callers of old crate names still find symbols via new module paths (no missing re-exports)
- [ ] Feature flag `serde` works on types in both `codes` and `types` modules
- [ ] Inline tests from `catalog/mod.rs:169-205` migrated and passing

## Final Verification

- [ ] Workspace members count exactly 120
- [ ] Publish allowlist count exactly 118
- [ ] No stray references to `perl-diagnostics-codes`, `perl-lsp-diagnostic-catalog`, or `perl-lsp-diagnostic-types` in source code
- [ ] Git status clean except for new crate and modifications to existing files
- [ ] Build succeeds: `cargo build -p perl-lsp-rs --release` (full LSP server build)

