# Acceptance Criteria for #4430 — Wave H Microcrate Collapse

## Functional Requirements

- [ ] All 11 satellite crates absorbed into `perl-dap/src/<module>/mod.rs` with flat layout (no hierarchical grouping)
- [ ] `src/platform.rs` and `src/security.rs` converted to `platform/mod.rs` and `security/mod.rs` BEFORE satellite content merged
- [ ] All satellite source code copied to destination modules with internal imports updated to `use crate::<module>::...;`
- [ ] Inter-satellite dependencies resolved: `command_args` before `platform`, `platform` before `shell`, `value` before `variables` in lib.rs module declarations
- [ ] Type name collisions resolved: `src/types/mod.rs` types do not cause ambiguity with `src/protocol.rs` types; `debug_adapter/mod.rs` uses `use crate::types::...;` qualified paths
- [ ] `src/api.rs` created with explicit named re-exports from all 11 modules (no wildcard re-exports)

## Migration Requirements

- [ ] All 28 test files migrated to `crates/perl-dap/tests/` with prefixed names per mapping table
- [ ] All 28 test files registered as explicit `[[test]]` entries in `crates/perl-dap/Cargo.toml` (total: 51 test targets)
- [ ] Ten internal use-path updates completed (see Trap 4 in checklist)
- [ ] `crates/perl-lsp/Cargo.toml`: dependency changed from `perl-dap-platform` to `perl-dap`
- [ ] `crates/perl-lsp/src/runtime/lifecycle/workspace.rs`: import path updated to `perl_dap::platform::*`
- [ ] `crates/perl-lsp-config/Cargo.toml`: dependency changed from `perl-dap-platform` to `perl-dap`
- [ ] `crates/perl-lsp-config/src/lib.rs`: import path updated to `perl_dap::platform::*`

## Workspace & Build Requirements

- [ ] Root `Cargo.toml` members section cleaned: 11 `"crates/perl-dap-*"` entries removed (lines 109-121)
- [ ] Root `Cargo.toml` publish allowlist cleaned: 11 `"perl-dap-*"` entries removed, keep `"perl-dap"` (lines 237-250)
- [ ] Root `Cargo.toml` workspace dependencies cleaned: 11 `perl-dap-* = { path = ... }` entries removed (lines 344-356)
- [ ] All 11 satellite crate directories removed from `crates/` filesystem

## Verification Gates (Must Pass)

- [ ] `cargo build -p perl-dap --lib` succeeds with zero errors
- [ ] `cargo test -p perl-dap` succeeds (51 registered tests total, all passing)
- [ ] `cargo test -p perl-dap --list` outputs exactly 51 test targets
- [ ] `cargo build -p perl-lsp --release` succeeds
- [ ] `cargo build -p perl-lsp-config --release` succeeds
- [ ] `cargo xtask publish-closure` reports 112 workspace members (down from 123, a drop of 11)
- [ ] `cargo clippy -p perl-dap` returns zero warnings in new code
- [ ] `cargo xtask fmt` shows zero formatting violations

## Quality Requirements

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code (Wave 1 standards)
- [ ] All module exports explicitly re-exported in `api.rs` (no implicit wildcard re-exports)
- [ ] All test files correctly prefixed to prevent name collisions with existing 46 test files in `perl-dap/tests/`
- [ ] Type ambiguity resolved: running `cargo check -p perl-dap` shows no "multiple applicable items in scope" errors

## Scope Boundaries (Out of Scope)

- [ ] No refactoring of satellite code (copy as-is)
- [ ] No feature flag changes (all 11 remain published)
- [ ] No public API changes (consumer imports from `perl_dap::platform::*` instead of `perl_dap_platform::*`)
- [ ] No internal reorganization of `perl-dap` beyond module folder creation

## Final State

- [ ] 11 satellite crates completely removed
- [ ] `perl-dap` is the only remaining DAP crate (published)
- [ ] Workspace member count: 123 → 112 (confirmed by `cargo xtask publish-closure`)
- [ ] All tests passing
- [ ] External consumers updated and building successfully
- [ ] Ready to merge
