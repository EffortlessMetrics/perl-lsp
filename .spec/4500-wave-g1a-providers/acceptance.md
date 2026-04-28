# Acceptance Criteria — Wave G1a Collapse (Issue #4500)

These are the testable outcomes that prove G1a is complete. Red-TDD should write tests matching these criteria before the builder starts implementation.

---

## Structural Completeness

- [ ] `crates/perl-lsp-rs-core/src/providers/mod.rs` exists
- [ ] All 15 submodules declared in `providers/mod.rs`: `completion_item`, `file_completion`, `code_lens`, `document_highlight`, `folding`, `selection_range`, `inlay_hints`, `type_hierarchy`, `formatting_types`, `on_type_formatting`, `color`, `symbol_query`, `import_management`, `document_links`, `workspace_symbols`
- [ ] `pub mod providers;` added to `crates/perl-lsp-rs-core/src/lib.rs`
- [ ] All 15 original crate directories (`crates/perl-lsp-*/`) deleted

---

## Compilation & Type Safety

- [ ] `cargo check --workspace --lib` passes with 0 errors
- [ ] `cargo clippy --workspace` passes with 0 warnings (G1a-related)
- [ ] `cargo clippy -p perl-lsp-rs-core --all-targets` passes with 0 warnings
- [ ] All intra-module imports rewritten: `perl_lsp_CRATE::` → `perl_lsp_rs_core::providers::MODULE::`
- [ ] Intra-provider consumer imports correct: `file_completion` imports from `crate::providers::completion_item`, `workspace_symbols` imports from `crate::providers::symbol_query`

---

## Test Coverage & Parity

- [ ] All 20 migrated test files copied to `crates/perl-lsp-rs-core/tests/` (see APPENDIX B of checklist)
- [ ] All inline `#[cfg(test)]` blocks migrated with their source modules
- [ ] Test file naming convention: `provider_MODULE_DESCRIPTOR.rs` (e.g., `provider_completion_item_dedup_sort.rs`)
- [ ] `cargo test -p perl-lsp-rs-core --lib` compiles and executes all tests
- [ ] `cargo test --workspace --lib` final count ≥ pre-G1a baseline (no test loss)
- [ ] No test file name collisions in `crates/perl-lsp-rs-core/tests/` directory

---

## Consumer Integration

- [ ] `crates/perl-lsp/Cargo.toml`: 12 direct G1a deps removed, `perl-lsp-rs-core` remains
- [ ] `crates/perl-lsp-completion/Cargo.toml`: `perl-lsp-completion-item` and `perl-lsp-file-completion` deps removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-code-actions/Cargo.toml`: `perl-lsp-import-management` dep removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-formatting/Cargo.toml`: `perl-lsp-formatting-types` dep removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-navigation/Cargo.toml`: `perl-lsp-document-links`, `perl-lsp-type-hierarchy`, `perl-lsp-workspace-symbols` deps removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-providers/Cargo.toml`: `perl-lsp-on-type-formatting`, `perl-lsp-inlay-hints`, `perl-lsp-code-lens`, `perl-lsp-folding` deps removed, `perl-lsp-rs-core` added
- [ ] All import statements in consumer crates updated (7 source files in `perl-lsp`, 3 in `perl-lsp-completion`, 2 in `perl-lsp-code-actions`, 1 in `perl-lsp-formatting`, 5 in `perl-lsp-navigation`, 4 in `perl-lsp-providers`)
- [ ] `cargo check --workspace --lib` passes across all 6 consumer crates

---

## Test Registry Update

- [ ] `crates/perl-lsp/tests/wired_crates_integration_test.rs`: exactly 6 import lines rewritten
  - `perl_lsp_workspace_symbols::WorkspaceSymbolsProvider` → `perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider`
  - `perl_lsp_symbol_query::matches_query` → `perl_lsp_rs_core::providers::symbol_query::matches_query`
  - `perl_lsp_completion_item::*` (3 imports) → `perl_lsp_rs_core::providers::completion_item::*`
  - `perl_lsp_formatting_types::FormatRange` → `perl_lsp_rs_core::providers::formatting_types::FormatRange`
  - `perl_lsp_import_management::collect_imports` → `perl_lsp_rs_core::providers::import_management::collect_imports`
  - `perl_lsp_document_links::compute_links` → `perl_lsp_rs_core::providers::document_links::compute_links`
- [ ] No remaining references to old crate names: `grep -c "perl_lsp_completion_item\|perl_lsp_symbol_query\|perl_lsp_workspace_symbols\|perl_lsp_formatting_types\|perl_lsp_import_management\|perl_lsp_document_links" crates/perl-lsp/tests/wired_crates_integration_test.rs` returns `0`
- [ ] `cargo test -p perl-lsp-rs -- wired_crates` passes

---

## Configuration & Build System

- [ ] `xtask/published-crate-baseline.txt`: contains exactly `59` (14 crates removed: 74 − 15 = 59)
- [ ] `Cargo.toml` root `[workspace.metadata.publish.allow]`: 15 crate names removed, count verified to be 59
- [ ] `xtask/src/tasks/build_timing.rs`: any G1a references removed if present
- [ ] `xtask/src/tasks/targeted_checks.rs`: any G1a references removed if present
- [ ] `scripts/verify-docs-rs.sh`: any G1a references removed if present

---

## Code Quality & Linting

- [ ] `cargo xtask fmt` produces no changes (code is properly formatted)
- [ ] `cargo xtask layer-check` passes (microcrate dependency order intact)
- [ ] `cargo xtask published-crate-count-check` passes (59 published crates)
- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in migrated code
- [ ] All inline doc comments preserved (no loss of documentation)

---

## Integration Testing

- [ ] `cargo test --workspace --lib` all tests pass
- [ ] `cargo test -p perl-lsp-rs-core` all submodule tests pass
- [ ] `cargo test -p perl-lsp-rs` all LSP server tests pass
- [ ] `just pr-fast` (local fast CI gate) passes
- [ ] No snapshot test regressions (insta snapshots updated if needed)

---

## Documentation & Clarity

- [ ] Module docstring added to `crates/perl-lsp-rs-core/src/providers/mod.rs` explaining the 3-group structure
- [ ] Each provider submodule preserves original crate docstrings and public API surface
- [ ] No breaking changes to public re-exports from `perl_lsp_rs_core::providers::<provider>::*`

---

## Parent Issue Tracking

- [ ] Issue #4500 marked `spec-reviewed` and ready for red-TDD
- [ ] Implementation branch `impl/4500-wave-g1a-providers` created off master 2a57448c8
- [ ] `.spec/4500-wave-g1a-providers/` directory populated with checklist.md, acceptance.md, context.md

