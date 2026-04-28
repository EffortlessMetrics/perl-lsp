# Acceptance Criteria: #4541 — Wave Final PR B

## Functional requirements

- [ ] Absorption complete: `perl-feature-catalog`, `perl-lsp-config`, `perl-content-length-framing` moved into `perl-lsp-rs-core` as internal modules
- [ ] Cycle broken: `perl-lsp-config` no longer depends on `perl-dap`; now depends on `perl-lsp-rs-core::platform`
- [ ] Platform functions copied: `resolve_perl_path_with_toolchain`, `detect_perlbrew_perl`, `detect_plenv_perl` accessible via `perl-lsp-rs-core::platform::`
- [ ] Config accessible: `perl_lsp_rs_core::config::*` types available to `perl-lsp` runtime callers
- [ ] Framing accessible: `perl_lsp_rs_core::transport::framing::{ContentLengthFramer, frame}` available to DAP and LSP tests
- [ ] Feature catalog accessible: `perl_lsp_rs_core::feature_catalog::*` available to build.rs in both perl-dap and perl-lsp-rs-core

## Dependency cleanup

- [ ] `perl-lsp-config` removed from `perl-lsp/Cargo.toml`
- [ ] `perl-content-length-framing` removed from `perl-lsp/Cargo.toml`
- [ ] `perl-feature-catalog` removed from `perl-lsp-rs-core/Cargo.toml` build-dependencies
- [ ] `perl-content-length-framing` removed from `perl-dap/Cargo.toml`
- [ ] `perl-dap` updated to depend on `perl-lsp-rs-core` in build-dependencies (for feature-catalog access)
- [ ] All consumer files updated: perl-dap (4 files), perl-lsp (4 files)

## Import rewiring

- [ ] `perl-lsp/src/runtime/language/misc.rs`: all `perl_lsp_config::` → `perl_lsp_rs_core::config::`
- [ ] `perl-lsp/src/runtime/lifecycle/module_resolution.rs`: all `perl_lsp_config::` → `perl_lsp_rs_core::config::`
- [ ] `perl-dap/src/debug_adapter/mod.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-dap/src/tcp_attach.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-dap/tests/dap_attach_e2e.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-dap/tests/tcp_attach_tests.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-lsp/tests/support/lsp_harness.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-lsp/tests/support/message_framing.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-lsp/tests/lsp_content_length_framing_integration.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-lsp/tests/lsp_streaming_completion_tests.rs`: all `perl_content_length_framing::` → `perl_lsp_rs_core::transport::framing::`
- [ ] `perl-lsp-rs-core/build.rs`: all `perl_feature_catalog::` → `crate::feature_catalog::`
- [ ] `perl-dap/build.rs`: all `perl_feature_catalog::` → `perl_lsp_rs_core::feature_catalog::`

## Baseline and accounting

- [ ] Published crate count reduced from 34 → 31
- [ ] `xtask/published-crate-baseline.txt` updated to `31`
- [ ] Root `Cargo.toml` `[workspace.metadata.publish.allow]` reduced by 3 entries: `perl-feature-catalog`, `perl-lsp-config`, `perl-content-length-framing`
- [ ] G3 baseline tests updated: `crates/perl-lsp-rs-core/tests/g3_published_count.rs` asserts `== 31`
- [ ] G3 baseline tests updated: `crates/perl-lsp-rs-core/tests/g3_publish_baseline_enforcement.rs` asserts `== 31`

## Test cleanup and new tests

- [ ] G3 negative tests deleted: `crates/perl-lsp-rs-core/tests/g3_config_stays_standalone.rs` REMOVED
- [ ] G3 negative tests deleted: `crates/perl-lsp-rs-core/tests/g3_content_length_framing_stays.rs` REMOVED
- [ ] New test file created: `crates/perl-lsp-rs-core/tests/wave_final_absorption_tests.rs` with:
  - [ ] `config_accessible_via_rs_core()` — asserts `perl_lsp_rs_core::config::*` types are public and usable
  - [ ] `framing_accessible_via_rs_core_transport()` — asserts `perl_lsp_rs_core::transport::framing::*` types are public
  - [ ] `feature_catalog_accessible_via_rs_core()` — asserts `perl_lsp_rs_core::feature_catalog::*` types are public
  - [ ] `platform_resolver_accessible_via_rs_core()` — asserts `perl_lsp_rs_core::platform::{resolve_perl_path_with_toolchain, detect_perlbrew_perl, detect_plenv_perl}` are public
  - [ ] `wave_final_crates_have_publish_false()` — asserts all three old crates have `publish = false` in Cargo.toml
  - [ ] `published_count_is_31_after_wave_final()` — asserts baseline file reads 31 and `cargo xtask published-crate-count` confirms 31 published

## Publish flag updates

- [ ] `crates/perl-feature-catalog/Cargo.toml` has `publish = false`
- [ ] `crates/perl-lsp-config/Cargo.toml` has `publish = false`
- [ ] `crates/perl-content-length-framing/Cargo.toml` has `publish = false`

## Compilation and verification gates

- [ ] All crates compile: `cargo check --all`
- [ ] All tests pass: `cargo test --all`
- [ ] Library tests pass: `cargo test -p perl-lsp-rs-core --lib && cargo test -p perl-dap --lib && cargo test -p perl-lsp --lib`
- [ ] No clippy warnings: `cargo clippy -p perl-lsp-rs-core -p perl-dap -p perl-lsp -- -D warnings`
- [ ] Formatted: `cargo xtask fmt`
- [ ] No new dependency cycles: `cargo xtask layer-check`
- [ ] Cargo metadata agrees on count: `cargo metadata --no-deps | jq '[.packages[] | select(.publish != [])] | length'` = `31`

## Edge cases and error paths

- [ ] Backward compatibility: `perl-lsp-config` can still be built as standalone (for tooling consumers), even though `publish = false` prevents re-publication
- [ ] No orphaned imports: grep confirms no remaining `use perl_lsp_config::` in production code (only in absorbed modules now in rs-core)
- [ ] No orphaned imports: grep confirms no remaining `use perl_content_length_framing::` in production code (only in absorbed transport/framing.rs)
- [ ] Cross-crate consumers updated: Windows platform detection logic in perl-dap still works after platform function move to rs-core

## Documentation

- [ ] Amendment 9 added to `docs/adr/0041-microcrate-collapse.md` (optional in this PR, separate PR acceptable)
  - Baseline correction documented (G3 left 37, actual was 34)
  - Wave Final scope documented (3 crates absorbed, 31 end count)
  - perl-feature-catalog ledger correction documented (was listed for perl-parser, actually perl-lsp-rs-core build-dep)
  - G3 negative tests deletion documented as superseded

## Integration checks

- [ ] LSP server builds: `cargo build -p perl-lsp-rs --release`
- [ ] DAP server builds: `cargo build -p perl-dap --release`
- [ ] All integration tests still pass (if any depend on absorbed modules)
