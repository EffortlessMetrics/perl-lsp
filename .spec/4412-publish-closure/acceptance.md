# Acceptance Criteria: publish-closure gate

**Issue**: #4412
**Command**: `cargo xtask publish-closure`

These are the exact test assertions that verify the implementation is complete.

## Functional Criteria

### Default behavior (all crates)
- [ ] `cargo xtask publish-closure` exits with status 0
- [ ] Output contains: `publish-closure: OK (132 crates checked, 0 violations)`
- [ ] No stderr output on success

### Single-crate filtering
- [ ] `cargo xtask publish-closure --crate-name perl-token` exits with status 0
- [ ] Output contains: `publish-closure: OK (1 crate checked, 0 violations)`

### Invalid crate name
- [ ] `cargo xtask publish-closure --crate-name nonexistent-crate-xyz` exits with status 1
- [ ] Stderr contains: `Crate 'nonexistent-crate-xyz' not found in publish allowlist`

### Help text
- [ ] `cargo xtask publish-closure --help` exits with status 0
- [ ] Help text includes: `Check only this crate`
- [ ] Help text shows flag: `--crate-name`

## Violation Detection (manual verification on contrived data)

If the implementation were to encounter a published crate with a transitive normal dep on a `publish = false` crate, it MUST output:

```
ERROR: publish-closure violation
  Published crate `<published_name>` has transitive normal dep on `<forbidden_name>` (publish = false)
```

**Note**: No such violations exist on master — gate starts green.

## Integration Criteria

### justfile recipe
- [ ] `just ci-publish-closure` exists and runs `cargo xtask publish-closure`
- [ ] Recipe exits 0 on master
- [ ] Recipe includes emoji and status messages

### pr-fast gate
- [ ] `just pr-fast` includes step: `just _timed "publish-closure" "just ci-publish-closure"`
- [ ] Step runs after `test-core`
- [ ] pr-fast passes on master

### ci-gate
- [ ] `just ci-gate` includes: `just ci-publish-closure`
- [ ] Step runs after `just hook-tests`
- [ ] ci-gate passes on master

## Test Criteria

### Unit tests
- [ ] `cargo test -p xtask -- publish_closure` passes
- [ ] Test `publish_closure_passes_on_master` passes
- [ ] Test `publish_closure_single_crate_flag` passes
- [ ] Test `publish_closure_unknown_crate_exits_nonzero` passes

## Quality Criteria

### Linting
- [ ] `cargo clippy -p xtask` produces no warnings
- [ ] All clippy checks with `-D warnings` pass

### Formatting
- [ ] `cargo xtask fmt --check` requires no changes
- [ ] Code follows project formatting conventions

### Banned patterns
- [ ] No `unwrap()` in production code (tests may use `perl_tdd_support::must`)
- [ ] No `expect()` in production code
- [ ] No `panic!()` in production code
- [ ] No `todo!()` in production code
- [ ] No `dbg!()` in production code
- [ ] All errors use `Result<()>` or `.ok_or_else()` pattern

### Dependencies
- [ ] No new dependencies added to `xtask/Cargo.toml`
- [ ] Uses existing: `serde`, `serde_json`, `color_eyre`

## Spec Conformance

### CLI interface
- [ ] Command: `cargo xtask publish-closure`
- [ ] Flag: `--crate-name <NAME>` (optional)
- [ ] Return type: `Result<()>` in publish_closure.rs

### Algorithm
- [ ] Calls `cargo metadata --format-version 1` WITHOUT `--no-deps`
- [ ] Parses `resolve` section (required for transitive walk)
- [ ] Identifies `publish = false` crates as `publish: []` in JSON
- [ ] BFS/DFS follows only normal deps (skips `kind == "dev"` and `kind == "build"`)
- [ ] Uses `resolve.nodes[].deps[].pkg` field (NOT `id`)
- [ ] Reports all violations before exiting 1

### Output format
- [ ] Success: `publish-closure: OK (N crates checked, 0 violations)`
- [ ] Error: `ERROR: publish-closure violation` (per violation)
- [ ] Invalid crate: error message names the unrecognized crate

## File Completeness

- [ ] `xtask/src/tasks/publish_closure.rs` exists (~150 lines)
- [ ] `xtask/src/tasks/mod.rs` has `pub mod publish_closure;`
- [ ] `xtask/src/main.rs` has `PublishClosure` variant in `Commands` enum
- [ ] `xtask/src/main.rs` dispatches to `publish_closure::run()`
- [ ] `justfile` has `ci-publish-closure` recipe
- [ ] `justfile` pr-fast includes publish-closure step
- [ ] `justfile` ci-gate includes publish-closure step
- [ ] `xtask/tests/publish_closure.rs` exists with 3 tests
