# Acceptance Criteria: Wave B Microcrate Collapse (perl-symbol-*)

Issue: #4428 | Branch: `impl/4428-perl-symbol-wave-b`

---

## New crate structure

- [ ] `crates/perl-symbol/` directory created with flat module folders: `src/types/`, `src/cursor/`, `src/index/`, `src/surface/`
- [ ] `crates/perl-symbol/Cargo.toml` has `name = "perl-symbol"`, `edition.workspace = true` (resolves to 2024), `publish = true`, `[lib] doctest = false`
- [ ] `crates/perl-symbol/Cargo.toml` `[dependencies]` contains only `perl-ast` and `serde` (NOT `perl-position-tracking`, NOT `perl-symbol-types`)
- [ ] `crates/perl-symbol/src/lib.rs` declares 4 modules (`pub mod types; pub mod cursor; pub mod index; pub mod surface;`) plus `pub mod api; pub use api::*;`
- [ ] `crates/perl-symbol/src/api.rs` uses explicit named re-exports (NO wildcards); re-exports `SymbolKind` and `VarKind` at crate root for ergonomic migration
- [ ] `crates/perl-symbol/CLAUDE.md` created; preserves `perl-symbol-surface/CLAUDE.md` "NOT allowed" invariant (`perl-parser-core`, `lsp-types`, and LSP provider crates) verbatim

## Satellite removal

- [ ] Directory `crates/perl-symbol-types/` deleted
- [ ] Directory `crates/perl-symbol-cursor/` deleted
- [ ] Directory `crates/perl-symbol-index/` deleted
- [ ] Directory `crates/perl-symbol-surface/` deleted

## Root `Cargo.toml` edits

- [ ] `[workspace.members]` no longer contains any `crates/perl-symbol-*` entry (4 removed, covering both the isolated `perl-symbol-surface` at line 70 AND the cluster at lines 81-83); adds `crates/perl-symbol`
- [ ] `[workspace.dependencies]` no longer contains any `perl-symbol-types`/`-cursor`/`-index`/`-surface` key (4 removed, covering both the cluster at lines 275-277 AND the isolated `perl-symbol-surface` at line 290); adds `perl-symbol = { path = "crates/perl-symbol", version = "0.12.4" }`
- [ ] `[workspace.metadata.publish].allow` no longer contains any of the 4 satellite names; adds `"perl-symbol"` (net -3 entries)

## Consumer migration (5 crates)

- [ ] `crates/perl-workspace-index/Cargo.toml`: `perl-symbol-types` → `perl-symbol`
- [ ] `crates/perl-workspace-index/src/workspace/workspace_index.rs:1022`: `pub use perl_symbol_types::{SymbolKind, VarKind};` → `pub use perl_symbol::{SymbolKind, VarKind};`
- [ ] `crates/perl-workspace-index/tests/dual_indexing_tests.rs:442`: `perl_symbol_types::SymbolKind` → `perl_symbol::SymbolKind`
- [ ] `crates/perl-semantic-analyzer/Cargo.toml`: `perl-symbol-types` → `perl-symbol`
- [ ] `crates/perl-semantic-analyzer/src/analysis/symbol.rs:37`: `pub use perl_symbol_types::{SymbolKind, VarKind};` → `pub use perl_symbol::{SymbolKind, VarKind};`
- [ ] `crates/perl-lsp/Cargo.toml`: `perl-symbol-cursor` → `perl-symbol`
- [ ] `crates/perl-lsp/src/util/mod.rs:17`: `pub use perl_symbol_cursor::{...};` → `pub use perl_symbol::cursor::{...};`
- [ ] `crates/perl-lsp-rename/Cargo.toml`: `perl-symbol-cursor` → `perl-symbol`
- [ ] `crates/perl-lsp-rename/src/rename/resolve.rs:7`: `use perl_symbol_cursor as cursor;` → `use perl_symbol::cursor as cursor;`
- [ ] `crates/perl-lsp-performance/Cargo.toml`: `perl-symbol-index` → `perl-symbol`
- [ ] `crates/perl-lsp-performance/src/lib.rs:14`: `pub use perl_symbol_index::SymbolIndex;` → `pub use perl_symbol::SymbolIndex;` (crate-root re-export)
- [ ] `crates/perl-lsp-workspace-symbols/src/lib.rs:298` (comment-only): `perl_symbol_types::SymbolKind` → `perl_symbol::SymbolKind`

## Test migration (7 test files in `crates/perl-symbol/tests/`)

- [ ] `types_comprehensive_unit_tests.rs` (from `perl-symbol-types/tests/comprehensive_unit_tests.rs`, prefixed to avoid collision)
- [ ] `types_extended.rs` (from `perl-symbol-types/tests/symbol_types_extended.rs`)
- [ ] `cursor_comprehensive_unit_tests.rs` (from `perl-symbol-cursor/tests/comprehensive_unit_tests.rs`, prefixed)
- [ ] `cursor_bdd.rs` (from `perl-symbol-cursor/tests/cursor_symbol_bdd.rs`, renamed to drop redundant prefix)
- [ ] `index_trie_and_fuzzy.rs` (from `perl-symbol-index/tests/trie_and_fuzzy.rs`, prefixed)
- [ ] `surface_decl.rs` (from `perl-symbol-surface/tests/symbol_decl_tests.rs`, renamed)
- [ ] `facade_api_completeness.rs` (NEW — guards public API surface; Wave 1 pattern)
- [ ] All test files updated import paths from `perl_symbol_X::` to `perl_symbol::` (crate root) or `perl_symbol::X::` (module path)

## Verification

- [ ] `grep -rn 'perl_symbol_types\|perl_symbol_cursor\|perl_symbol_index\|perl_symbol_surface' crates/ --include='*.rs'` returns zero hits
- [ ] `grep -rn 'perl-symbol-types\|perl-symbol-cursor\|perl-symbol-index\|perl-symbol-surface' crates/ --include='*.toml'` returns zero hits
- [ ] `cargo metadata --no-deps` shows workspace member count decreased by 3 from pre-change baseline
- [ ] `cargo xtask publish-closure` lists `perl-symbol`; lists none of the 4 old satellite names; allowlist count decreased by 3
- [ ] `cargo check --workspace` passes
- [ ] `cargo build --workspace --lib` passes
- [ ] `cargo test -p perl-symbol` passes (all 7 test binaries)
- [ ] `cargo test --workspace --lib` passes (no regressions)
- [ ] `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` passes
- [ ] `cargo clippy --workspace --lib` produces no new warnings
- [ ] `cargo xtask fmt` leaves no diff
- [ ] All 5 consumer crates build individually: `perl-workspace-index`, `perl-semantic-analyzer`, `perl-lsp-rs`, `perl-lsp-rename`, `perl-lsp-performance`

## PR hygiene

- [ ] PR title ends with `(#4428)` (validate-title CI check)
- [ ] PR is based on master AFTER Wave A (#4434) merged (verified: commit `b6b8d1d7d` or newer in history)
- [ ] PR commit message follows conventional format: `refactor(symbol): collapse perl-symbol-* (4 crates) → perl-symbol facade (Wave B) (#4428)`
