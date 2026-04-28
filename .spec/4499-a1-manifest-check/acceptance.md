# Acceptance Criteria: #4499 publish-manifest-check

**Issue:** #4499 — A1 offline manifest-lint (consolidate Python allowlist-drift + add LICENSE check)

**Acceptance test format:** One criterion per line, checkboxable. All must pass before PR submission.

---

## Functional acceptance

- [ ] `cargo xtask publish-manifest-check` runs without arguments
- [ ] Command exits 0 when allowlist and publishable set match and all crates have licenses
- [ ] Command exits 1 and reports "drift:" when a crate in allowlist has `publish=false`
- [ ] Command exits 1 and reports "drift:" when a publishable crate is absent from allowlist
- [ ] Command exits 1 and reports "license:" when an allowlisted crate has no `license` or `license-file`
- [ ] Command outputs violations to stderr, one per line, each prefixed "ERROR: publish-manifest-check:"
- [ ] Success message printed to stdout: "publish-manifest-check: OK (N crates checked, 0 violations)"
- [ ] Unit test `clean_metadata_no_violations()` passes (no violations in clean metadata)
- [ ] Unit test `drift_a_allowlist_has_publish_false_crate()` passes (allowlist + publish=false = drift)
- [ ] Unit test `drift_b_publishable_crate_absent_from_allowlist()` passes (publishable not in allowlist = drift)
- [ ] Unit test `missing_license_detected()` passes (missing license detected)
- [ ] Integration test `publish_manifest_check_passes_on_master()` passes (no violations on master HEAD)

## Refactoring acceptance

- [ ] `load_publish_allowlist()` function defined in `xtask/src/utils.rs`
- [ ] `AllowlistMetadata`, `WorkspacePublishMeta`, `AllowList` types defined in utils.rs
- [ ] `publish_closure.rs` imports `load_publish_allowlist` and uses it (no duplicate structs)
- [ ] `count_ratchet.rs` imports `load_publish_allowlist` and uses it (no duplicate structs)
- [ ] `publish_closure.rs` tests still pass (refactor did not break logic)
- [ ] `count_ratchet.rs` tests still pass (refactor did not break logic)
- [ ] `xtask/src/tasks/mod.rs` declares `pub mod publish_manifest_check;`
- [ ] `xtask/src/main.rs` `Commands` enum includes `PublishManifestCheck` variant with doc comment
- [ ] `xtask/src/main.rs` match dispatch includes `Commands::PublishManifestCheck => tasks::publish_manifest_check::run()`

## Integration acceptance

- [ ] `.github/workflows/publish-dry-run.yml` lines 61-70 replaced with single line: `run: cargo xtask publish-manifest-check`
- [ ] `.github/workflows/publish-dry-run.yml` lines 72 onward unchanged (topo-sort step remains in Python)
- [ ] `.github/workflows/publish-dry-run.yml` paths trigger includes `'xtask/src/tasks/publish_manifest_check.rs'`
- [ ] `scripts/publish-topo.py` file still exists (not deleted)
- [ ] `scripts/tests/test-publish-topo.py` step still present in workflow (not removed)
- [ ] `justfile` includes `ci-publish-manifest-check` recipe
- [ ] `just pr-fast` recipe includes call to `publish-manifest-check` gate
- [ ] `just ci-gate` recipe includes call to `ci-publish-manifest-check`
- [ ] `publish-allowlist-check` recipe (line ~2132) delegates to `cargo xtask publish-manifest-check`

## Code quality acceptance

- [ ] No `unwrap()` in production code (xtask/src/tasks/publish_manifest_check.rs)
- [ ] No `expect()` in production code
- [ ] No `panic!()` in production code
- [ ] No `todo!()` in production code
- [ ] No `dbg!()` in production code
- [ ] `cargo clippy -p xtask -- -D warnings` passes (no clippy violations)
- [ ] `cargo xtask fmt` produces no changes (code is formatted)
- [ ] All doc comments follow Rust convention (//! at module level, /// for items)

## Gate acceptance

- [ ] `cargo test -p xtask` passes (all unit + integration tests)
- [ ] `cargo test -p xtask --lib` passes (unit tests only)
- [ ] `cargo test -p xtask --test publish_manifest_check_test` passes (integration test)
- [ ] `cargo xtask publish-manifest-check` exits 0 on master 2a57448c8 (happy path)
- [ ] `cargo xtask publish-manifest-check --help` shows command with doc comment
- [ ] `just pr-fast` passes (includes publish-manifest-check gate)
- [ ] `just ci-gate` passes (includes publish-manifest-check gate)
- [ ] `cargo build -p xtask --release` succeeds
- [ ] Workspace compiles: `cargo build --workspace` succeeds

## Master baseline acceptance

- [ ] Master 2a57448c8 has zero allowlist drift violations (no violations reported)
- [ ] Master 2a57448c8 has zero missing-license violations (all allowlist crates have licenses)
- [ ] Workspace inheritance (44 crates with `license.workspace = true`) resolves to actual license strings in cargo metadata (no false positives)

---

**Approval checklist for PR submission:**

- [ ] All 45+ acceptance criteria checked
- [ ] `cargo test -p xtask` green on builder's machine
- [ ] `just pr-fast` green on builder's machine
- [ ] Builder has not added checks beyond allowlist-drift + LICENSE (no scope creep)
- [ ] No files deleted except possibly dead code
- [ ] Commit message references #4499 and describes: "Consolidate Python allowlist-drift check; add LICENSE-present validation"
