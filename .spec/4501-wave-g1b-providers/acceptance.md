# Wave G1b Provider Collapse — Acceptance Criteria

## Primary Deliverables

- All 10 crate directories deleted from `crates/`
- All provider functionality accessible via `perl_lsp_rs_core::providers::*`
- `perl_lsp_rs_core::providers::lsp_compat` contains ~1,600 LOC of original implementations from perl-lsp-providers/ide/lsp_compat/
- `crates/perl-lsp/Cargo.toml` — all 10 G1b dependencies removed (lines 36,37,48-56)
- All `crates/perl-lsp/src/` import sites updated from old crate names to new providers module paths
- `xtask/published-crate-baseline.txt` updated from 59 to 49
- 4 diagnostics snapshot files migrated to `crates/perl-lsp-rs-core/tests/snapshots/` with byte-identical content verification
- perl-lsp-providers test files migrated to `crates/perl-lsp-rs-core/tests/` with updated imports
- `perl-lsp-rs-core/Cargo.toml` updated with new dependencies (perl-lsp-text-utils, ureq)

## Build & Compilation

- `cargo check -p perl-lsp-rs-core` passes with zero errors
- `cargo check -p perl-lsp-rs` passes with zero unresolved imports
- `cargo clippy --workspace` produces zero lint warnings from G1b changes

## Test Coverage

- `cargo test -p perl-lsp-rs-core` green (all unit tests pass)
- `cargo test -p perl-lsp-rs-core diag_snap` passes with migrated snapshot content matching original byte-for-byte
- `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` green (LSP threading model verified)
- `just ci-gate` green (full continuous integration suite passes)

## Module Structure

- `perl_lsp_rs_core::providers::rename` module exists and exports all public items
- `perl_lsp_rs_core::providers::diagnostics` module exists and exports all public items
- `perl_lsp_rs_core::providers::inline_completion` module exists and exports all public items
- `perl_lsp_rs_core::providers::semantic_tokens` module exists and exports all public items
- `perl_lsp_rs_core::providers::formatting` module exists and exports all public items
- `perl_lsp_rs_core::providers::ai` module exists and exports all public items
- `perl_lsp_rs_core::providers::completion` module exists and exports all public items
- `perl_lsp_rs_core::providers::navigation` module exists and exports all public items
- `perl_lsp_rs_core::providers::code_actions` module exists and exports all public items
- `perl_lsp_rs_core::providers::lsp_compat` module exists with all signature_help, linked_editing, selection_range, folding, on_type_formatting implementations

## Backward Compatibility

- `perl_lsp_rs_core::providers` re-exports all 9 collapsed provider modules and lsp_compat
- Deprecated `tooling_export` alias preserved with `#[deprecated(since = "0.9.0")]` attribute
- `perl-lsp-tooling` dependency retained in perl-lsp-rs-core/Cargo.toml
- All existing public API surface of the 10 collapsed crates remains accessible via new module paths

## Dependency Management

- All inter-provider dependencies resolved within `perl_lsp_rs_core::providers` (no cross-crate imports)
- Cycle audit: `cargo check` enforces no circular module dependencies
- `perl-lsp-rs-core/Cargo.toml` includes: perl-lsp-text-utils, ureq, perl-lsp-tooling

## Documentation & Process

- PR body documents snapshot migration: "Migrated 4 diagnostics snapshots; content verified byte-identical to pre-G1b content."
- PR body notes any added wrapper constructors not present in original crates
- All 10 crate deletions are clean (no residual references in workspace)

## Behavioral Equivalence

- Zero observable behavior change through LSP protocol
- All LSP capabilities function identically pre- and post-collapse
- No new error messages or deprecation warnings from LSP server during normal operation
