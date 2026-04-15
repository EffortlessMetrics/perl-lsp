# Acceptance Criteria: Remove LSP Provider Re-exports from perl-parser (#4414)

Each criterion is individually verifiable and must pass before sign-off.

## Dependency Removal

- [ ] All 8 LSP crate dependencies removed from `crates/perl-parser/Cargo.toml` (lines 35-42 deleted)
- [ ] `cargo tree -p perl-parser --edges normal | grep "perl-lsp-"` returns empty (no LSP provider dependencies)

## Re-export Deletion

- [ ] Lines 437-496 deleted from `crates/perl-parser/src/lib.rs` (LSP provider re-export modules: code_actions, completion, diagnostics, document_links, implementation_provider, inlay_hints, inlay_hints_provider, references, rename, semantic_tokens, semantic_tokens_provider, type_definition, type_hierarchy, workspace_symbols)
- [ ] Lines 514-519 deleted from `crates/perl-parser/src/lib.rs` (tooling re-exports: performance, perl_critic, perltidy)
- [ ] Lines 498-513 in `crates/perl-parser/src/lib.rs` are preserved (legitimate refactor/tokens re-exports)
- [ ] `mod tooling;` declaration removed from `crates/perl-parser/src/lib.rs`
- [ ] `crates/perl-parser/src/tooling.rs` file deleted entirely

## Code Consumer Updates (Live Compilation)

- [ ] `crates/perl-lsp/src/lib.rs:428` updated: `perl_parser::perl_critic::*` → `perl_lsp_tooling::perl_critic::*`
- [ ] `crates/perl-lsp/src/features/diagnostics/pull.rs:335` updated: `perl_parser::perl_critic::BuiltInAnalyzer` → `perl_lsp_tooling::perl_critic::BuiltInAnalyzer`
- [ ] `crates/perl-parser/tests/ast_snapshot_tests.rs:13` updated to import `semantic_tokens` from `perl_lsp_semantic_tokens` with alias

## Documentation Updates (Examples)

- [ ] `docs/reference/LSP_IMPLEMENTATION_GUIDE.md:156` updated: `perl_parser::completion::*` → `perl_lsp_completion::*`
- [ ] `docs/reference/LSP_IMPLEMENTATION_GUIDE.md:1056` updated: `perl_parser::semantic_tokens::*` → `perl_lsp_semantic_tokens::*`
- [ ] `docs/reference/LSP_IMPLEMENTATION_GUIDE.md:1070` updated: `perl_parser::semantic_tokens_provider::*` → `perl_lsp_semantic_tokens::*`
- [ ] `docs/reference/LSP_PROVIDERS_REFERENCE.md:43` updated: `perl_parser::document_links::*` → `perl_lsp_navigation::*`
- [ ] `docs/reference/LSP_PROVIDERS_REFERENCE.md:106` updated: `perl_parser::document_links::*` → `perl_lsp_navigation::*`
- [ ] `docs/reference/LSP_PROVIDERS_REFERENCE.md:1243` updated: `perl_parser::implementation_provider::*` → `perl_lsp_navigation::*`
- [ ] `docs/how-to/IMPORT_OPTIMIZER_GUIDE.md:105` updated: `perl_parser::code_actions::*` → `perl_lsp_code_actions::*`
- [ ] `crates/perl-lsp/src/features/implementation_provider.rs:54` doc comment updated: `perl_parser::implementation_provider::*` → `perl_lsp_navigation::*`

## Compilation and Build Verification

- [ ] `cargo build -p perl-parser --release` succeeds with zero errors
- [ ] `cargo build -p perl-lsp-rs --release` succeeds with zero errors
- [ ] `cargo build -p perl-lsp-rs --release` (LSP server binary builds cleanly)

## Test Verification

- [ ] `cargo test -p perl-parser --lib` passes (unit tests for parser crate)
- [ ] `cargo test -p perl-parser --test ast_snapshot_tests` passes (snapshot tests, validates semantic_tokens import alias works)
- [ ] `cargo test -p perl-lsp-rs --lib` passes (LSP server tests, validates perl_critic imports work)

## Quality Checks

- [ ] `cargo clippy -p perl-parser --lib` green (no warnings or errors)
- [ ] `cargo clippy -p perl-lsp --lib` green (no warnings in consumers)
- [ ] `cargo xtask fmt --check` passes (all code is properly formatted)
- [ ] No `unwrap()`, `expect()`, `panic!()`, or `todo!()` introduced in refactor

## Scope Verification

- [ ] Total files changed: 11 (1 Cargo.toml, 3 src files in perl-parser, 2 src files in perl-lsp, 1 test file, 4 doc files)
- [ ] No changes to `crates/perl-parser/src/ide/lsp_compat/` (deferred to follow-up per ADR-0041)
- [ ] No behavioral changes to parser or LSP server (pure re-export removal)
- [ ] No feature flag additions or modifications

## Historical Context

- Tracking issue: #4410 (microcrate collapse roadmap)
- ADR: ADR-0041 (PR #4413, documents v0.14.0 clean-break decision)
- Related: This is the first PR (#0) of the microcrate collapse sequence
