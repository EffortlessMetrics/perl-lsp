# Wave F Acceptance Criteria

Each criterion below is checkboxable and extracted from issue #4489 and plan-review comments.

- [ ] `crates/perl-lsp-rs-core/` directory created with proper structure (src/, tests/, Cargo.toml, build.rs, features_sot.toml)
- [ ] 8 source crates' `src/lib.rs` files moved to `perl-lsp-rs-core/src/features/*.rs` modules (ids, contracts, flags, profile, profile_cli, policy, grid) and `src/capability_map.rs`
- [ ] All intra-wave imports rewritten from `use perl_lsp_feature_*::` to `use crate::features::*::` paths
- [ ] All test files moved to `crates/perl-lsp-rs-core/tests/` using `feature_<short>_<original>.rs` naming convention
- [ ] Test imports updated to use new module paths (`perl_lsp_rs_core::features::*`)
- [ ] Workspace root `Cargo.toml` [workspace.members] updated: 8 crates removed, `perl-lsp-rs-core` added
- [ ] Workspace root `Cargo.toml` [workspace.dependencies] updated: 8 entries removed, `perl-lsp-rs-core` added
- [ ] Workspace root `Cargo.toml` [workspace.metadata.publish].allow updated: 8 crates removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp/Cargo.toml` dependencies updated: 7 feature crate deps removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp/Cargo.toml` [features] section updated: `lsp-ga-lock` and other feature gates rewired to `perl-lsp-rs-core`
- [ ] `crates/perl-lsp-protocol/Cargo.toml` dependencies updated: 2 feature crate deps removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-protocol/src/capabilities.rs` imports rewritten: AdvertisedFeatures, BuildFlags, feature_ids_from_caps updated
- [ ] `crates/perl-lsp-feature-governance/Cargo.toml` dependencies updated: 5 feature crate deps removed, `perl-lsp-rs-core` added
- [ ] `crates/perl-lsp-feature-governance/Cargo.toml` [features] section updated: `lsp-ga-lock` forwarded to `perl-lsp-rs-core`
- [ ] `crates/perl-lsp-feature-governance/src/` imports rewritten to use `perl_lsp_rs_core::features::*` paths
- [ ] `crates/perl-lsp/src/lib.rs` facade re-exports added: `pub use perl_lsp_rs_core::{capability_map, features}`
- [ ] All 8 absorbed crate directories deleted from `crates/`: perl-lsp-feature-ids, perl-lsp-feature-contracts, perl-lsp-feature-flags, perl-lsp-feature-profile, perl-lsp-feature-profile-cli, perl-lsp-feature-policy, perl-lsp-feature-grid, perl-lsp-capability-map
- [ ] `xtask/published-crate-baseline.txt` updated from `81` to `74`
- [ ] `.spec/microcrate-collapse/ledger.md` Wave F rows marked complete (all 8 crates status updated)
- [ ] `cargo check --workspace` passes (no unresolved dependencies)
- [ ] `cargo test -p perl-lsp-rs-core` passes (all 11 test suites pass)
- [ ] `cargo test -p perl-lsp-rs` passes (LSP server tests pass)
- [ ] `cargo test -p perl-lsp-protocol` passes (protocol wrapper tests pass)
- [ ] `cargo test -p perl-lsp-feature-governance` passes (governance tests pass)
- [ ] `cargo xtask layer-check` passes (no layering violations)
- [ ] `cargo xtask fmt` passes (formatting compliant)
- [ ] `cargo clippy -p perl-lsp-rs-core -p perl-lsp-protocol -p perl-lsp-rs` passes (no clippy warnings)
- [ ] No orphaned references to removed crates in any remaining files (grep finds zero)
- [ ] PR title ends with `(#4489)` for validate-title CI check
