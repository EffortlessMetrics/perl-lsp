# Acceptance Criteria: Wave C Microcrate Collapse (lexer satellites)

Issue: #4444 | Branch: `impl/4444-perl-lexer-wave-c`

---

## Absorbing crate structure (`perl-lexer`)

- [ ] `crates/perl-lexer/src/keywords/mod.rs` exists and contains content from `perl-keywords/src/lib.rs`
- [ ] `crates/perl-lexer/src/builtins/mod.rs` exists with `pub mod builtin_signatures; pub mod phf_lookup; pub use phf_lookup as builtin_signatures_phf;`
- [ ] `crates/perl-lexer/src/builtins/builtin_signatures.rs` exists (from `perl-builtins/src/builtin_signatures.rs`)
- [ ] `crates/perl-lexer/src/builtins/phf_lookup.rs` exists (from `perl-builtins-phf/src/lib.rs`) — single file, NOT a folder
- [ ] `crates/perl-lexer/src/tokenizer/mod.rs` exists with `pub mod token_stream; pub mod token_wrapper; pub mod util;` and `pub use perl_token::{Token, TokenKind}; pub use token_stream::TokenStream; pub use token_wrapper::TokenWithPosition;`
- [ ] `crates/perl-lexer/src/tokenizer/token_stream.rs`, `token_wrapper.rs`, and `util.rs` exist (AST-agnostic slice only)
- [ ] `crates/perl-lexer/src/tokenizer/` does NOT contain `trivia.rs` or `trivia_parser.rs` (moved to parser-core)
- [ ] `crates/perl-lexer/src/api.rs` exists with explicit named re-exports (NO wildcards); re-exports all 14 keyword items, 7 builtin items, 6 tokenizer items enumerated in checklist Phase 2.4
- [ ] `crates/perl-lexer/src/lib.rs` declares `pub mod keywords; pub mod builtins; pub mod tokenizer; pub mod api;` and ends with `pub use api::*;`
- [ ] `crates/perl-lexer/src/lib.rs` line 139 uses `use crate::keywords::is_lexer_keyword;` (not `perl_keywords`)
- [ ] `crates/perl-lexer/src/token.rs` is UNCHANGED (existing lexer native token type — not from perl-token)

## Trivia relocation to `perl-parser-core`

- [ ] `crates/perl-parser-core/src/tokens/trivia.rs` exists (content of `perl-tokenizer/src/trivia.rs`)
- [ ] `crates/perl-parser-core/src/tokens/trivia_parser.rs` exists (content of `perl-tokenizer/src/trivia_parser.rs`, with internal `use super::trivia::*;`)
- [ ] `crates/perl-parser-core/src/tokens/mod.rs` declares `pub mod trivia; pub mod trivia_parser;` (local modules, not `pub use perl_tokenizer::`)
- [ ] `crates/perl-parser-core/src/tokens/mod.rs` uses `pub use perl_lexer::tokenizer::token_wrapper;` (not `perl_tokenizer`)
- [ ] `crates/perl-parser-core/src/tokens/token_stream.rs` line 27 uses `pub use perl_lexer::tokenizer::token_stream::*;`
- [ ] `crates/perl-parser-core/src/lib.rs` line 89 uses `pub use perl_lexer::builtins;` (not `perl_builtins`)
- [ ] `crates/perl-parser-core/src/lib.rs` line 114 uses `pub use perl_lexer::tokenizer::util;` (not `perl_tokenizer::util`)
- [ ] Public path `perl_parser_core::trivia::{Trivia, TriviaToken, NodeWithTrivia}` still resolves
- [ ] Public path `perl_parser_core::trivia_parser::{TriviaPreservingParser, format_with_trivia}` still resolves

## `Cargo.toml` changes

### Root `Cargo.toml`

- [ ] `[workspace.members]` no longer contains `"crates/perl-tokenizer"`, `"crates/perl-keywords"`, `"crates/perl-builtins"`, or `"crates/perl-builtins-phf"` (4 removed)
- [ ] `[workspace.members]` still contains `"crates/perl-token"` (NOT absorbed per ADR Amendment 4)
- [ ] `[workspace.members]` still contains `"crates/perl-lexer"`
- [ ] `[workspace.members]` count = 97 (was 101, delta -4)
- [ ] `[workspace.dependencies]` no longer contains `perl-tokenizer`, `perl-keywords`, `perl-builtins`, or `perl-builtins-phf` keys
- [ ] `[workspace.metadata.publish].allow` no longer contains `"perl-tokenizer"`, `"perl-keywords"`, `"perl-builtins"`, `"perl-builtins-phf"`
- [ ] `[workspace.metadata.publish].allow` count = 94 (was 98, delta -4)
- [ ] `[workspace.metadata.publish].allow` still contains `"perl-lexer"` and `"perl-token"`

### `perl-lexer/Cargo.toml`

- [ ] `[dependencies]` contains new entry `phf = { version = "0.13.1", features = ["macros"] }`
- [ ] `[dependencies]` NO LONGER contains `perl-keywords = { workspace = true }`
- [ ] `[dependencies]` does NOT contain `perl-ast-v2`, `perl-error`, or `perl-token` (trivia deps stay with moved modules in parser-core)
- [ ] `[dev-dependencies]` contains `perl-parser-core = { workspace = true }` (for migrated trivia-touching tests)
- [ ] `edition.workspace = true` preserved (resolves to 2024)

### Consumer Cargo.tomls (9 crates)

- [ ] `crates/perl-parser-core/Cargo.toml`: removed `perl-builtins`, removed `perl-tokenizer`; `perl-lexer` present
- [ ] `crates/perl-dap/Cargo.toml`: removed `perl-keywords`; `perl-lexer` present (add if missing)
- [ ] `crates/perl-lsp/Cargo.toml`: removed `perl-keywords`; `perl-lexer` present (already or add)
- [ ] `crates/perl-lsp-code-actions/Cargo.toml`: removed `perl-builtins`; `perl-lexer` present
- [ ] `crates/perl-lsp-completion/Cargo.toml`: removed `perl-keywords`; `perl-lexer` present
- [ ] `crates/perl-lsp-inlay-hints/Cargo.toml`: removed `perl-builtins`; `perl-lexer` present
- [ ] `crates/perl-lsp-rename/Cargo.toml`: removed `perl-keywords`; `perl-lexer` present
- [ ] `crates/perl-parser/Cargo.toml`: removed `perl-keywords`; `perl-lexer` present (direct or transitive)

## Source import rewrites

- [ ] `crates/perl-dap/src/debug_adapter/mod.rs:43`: `perl_lexer::DAP_COMPLETION_KEYWORDS`
- [ ] `crates/perl-lsp/src/runtime/language/completion.rs:21`: `perl_lexer::LSP_RUNTIME_COMPLETION_KEYWORDS`
- [ ] `crates/perl-lsp-code-actions/src/enhanced/import_management.rs:92`: `perl_lexer::is_builtin`
- [ ] `crates/perl-lsp-completion/src/completion/keywords.rs:11`: `perl_lexer::LSP_COMPLETION_KEYWORDS`
- [ ] `crates/perl-lsp-inlay-hints/src/inlay_hints.rs:21`: `perl_lexer::create_builtin_signatures`
- [ ] `crates/perl-lsp-rename/src/rename/validate.rs:5`: `perl_lexer::is_rename_keyword`
- [ ] `crates/perl-parser/src/ide/lsp_compat/rename.rs:59`: `perl_lexer::is_parser_lsp_keyword`
- [ ] `crates/perl-parser/src/ide/lsp_compat/completion.rs:64`: `perl_lexer::PARSER_LSP_KEYWORDS`

## Test migration (15 test files total)

Tests landing in `crates/perl-lexer/tests/` (14 files):

- [ ] `tokenizer_comprehensive_unit_tests.rs`
- [ ] `tokenizer_extended_unit_tests.rs`
- [ ] `tokenizer_edge_case_tests.rs`
- [ ] `tokenizer_bdd_consolidated.rs` (no additional prefix — already had one)
- [ ] `tokenizer_bridge_coverage_tests.rs`
- [ ] `keywords_comprehensive_unit_tests.rs`
- [ ] `keywords_classification.rs`
- [ ] `keywords_edge_cases.rs`
- [ ] `keywords_bdd_qw.rs`
- [ ] `builtins_comprehensive_unit_tests.rs`
- [ ] `builtins_extended_unit_tests.rs`
- [ ] `builtins_bdd_scenarios.rs`
- [ ] `builtins_phf_comprehensive_unit_tests.rs`
- [ ] `facade_api_completeness.rs` (NEW — Wave 1 pattern, guards public API)

Tests landing in `crates/perl-parser-core/tests/` (1 migrated):

- [ ] `trivia_edge_cases.rs` (from perl-tokenizer; imports rewritten to `perl_parser_core::{trivia, trivia_parser}`)

Test import correctness:

- [ ] All `perl_keywords::` imports in tests now `perl_lexer::` (or `perl_lexer::keywords::`)
- [ ] All `perl_builtins::` imports in tests now `perl_lexer::` (or `perl_lexer::builtins::...`)
- [ ] All `perl_builtins_phf::` imports in tests now `perl_lexer::` (or `perl_lexer::builtins::phf_lookup::`)
- [ ] All `perl_tokenizer::{token_stream,token_wrapper,util}::` imports now `perl_lexer::tokenizer::...` (or `perl_lexer::`)
- [ ] All `perl_tokenizer::{trivia,trivia_parser}::` imports now `perl_parser_core::{trivia,trivia_parser}::`

## Satellite directory removal

- [ ] Directory `crates/perl-tokenizer/` DELETED
- [ ] Directory `crates/perl-keywords/` DELETED
- [ ] Directory `crates/perl-builtins/` DELETED
- [ ] Directory `perl-builtins-phf/` DELETED
- [ ] `crates/perl-token/` PRESERVED (not absorbed)
- [ ] `crates/perl-lexer/` PRESERVED and expanded with absorbed content

## Verification commands (all must pass)

- [ ] `grep -rn 'perl_tokenizer\|perl_keywords\|perl_builtins\|perl_builtins_phf' crates/ --include='*.rs' | grep -v '.spec/'` returns zero hits
- [ ] `grep -rn 'perl-tokenizer\|perl-keywords\|perl-builtins\|perl-builtins-phf' crates/ --include='*.toml'` returns zero hits
- [ ] `cargo metadata --no-deps --format-version 1` parses cleanly; workspace_members length = 97
- [ ] `cargo xtask publish-closure` shows 94-entry allowlist; `perl-lexer` present; 4 satellites absent; `perl-token` present
- [ ] `cargo check --workspace` passes
- [ ] `cargo build --workspace --lib` passes
- [ ] `cargo test -p perl-lexer` passes all test binaries (~34 total post-migration)
- [ ] `cargo test -p perl-parser-core` passes (includes the migrated `trivia_edge_cases.rs`)
- [ ] `cargo test --workspace --lib` passes (no regressions)
- [ ] `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` passes
- [ ] `cargo clippy --workspace --lib` produces no new warnings
- [ ] `cargo xtask fmt` leaves no diff
- [ ] `cargo xtask layer-check` passes; confirms `perl-lexer` has no lib-graph deps on parser/semantic/LSP crates
- [ ] All 9 consumer crates build individually (perl-parser-core, perl-dap, perl-lsp-rs, perl-lsp-code-actions, perl-lsp-completion, perl-lsp-inlay-hints, perl-lsp-rename, perl-parser, perl-lexer)

## PR hygiene

- [ ] PR title ends with `(#4444)` for validate-title CI check
- [ ] PR commit message: `refactor(lexer): collapse lexer satellites -> perl-lexer (Wave C) (#4444)`
- [ ] PR is based on master AFTER #4446 merge (commit `6992efcb9` or newer)
- [ ] No `git stash` operations (worktree stash prohibition); if forced, verify no foreign changes landed
- [ ] Branch pushed to `origin` as `impl/4444-perl-lexer-wave-c`
