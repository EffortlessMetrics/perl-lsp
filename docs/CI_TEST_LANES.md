# CI Test Lanes

This document describes the test lane architecture for the Perl LSP project. Understanding these lanes is critical for maintaining CI budget discipline.

## Quick Reference

| Lane | Feature Flag | Default | Purpose |
|------|-------------|---------|---------|
| Core | (none) | ✅ PR | Fast, essential tests |
| LSP | (none) | ✅ PR | LSP integration tests |
| Stress | `stress-tests` | ❌ Label | Long-running stability tests |
| Extras | `lsp-extras` | ❌ Label | Optional LSP features |
| Security | `stress-tests` | ❌ Label | Security edge cases (can hang) |

## Local-First Workflow

**IMPORTANT**: CI should be a confirmation step, not your iteration loop.

### Before Any Push

```bash
# Fast merge gate (~2-5 min) - REQUIRED
just ci-gate

# Full CI pipeline (~10-20 min) - RECOMMENDED for large changes
just ci-full

# Nix users (deterministic, reproducible) - CANONICAL LOCAL GATE
nix develop -c just ci-gate
```

> **Note**: `nix flake check` doesn't work due to Nix sandbox blocking network
> access for Cargo dependencies. Use `nix develop -c just ci-gate` instead.

### What `just ci-gate` Runs

1. `cargo fmt --check --all` - Format check
2. `cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs` - Lint
3. `cargo test --workspace --lib --locked` - Library tests
4. `.ci/scripts/check-from-raw.sh` - Policy checks
5. LSP semantic definition tests

### What `just ci-full` Adds

1. Full clippy (all targets)
2. Core tests (lib + bins)
3. LSP integration tests (thread-constrained)
4. Documentation build

## Test Lane Details

### Core Lane (Default)

**Runs on**: Every PR
**Command**: `cargo test --workspace --lib --locked`

Fast, essential tests that must always pass. This is the minimum bar for any merge.

### LSP Lane (Default)

**Runs on**: Every PR with code changes
**Command**: `RUST_TEST_THREADS=2 cargo test -p perl-lsp --locked`

LSP integration tests with adaptive threading. Uses thread constraints to prevent resource exhaustion on CI runners.

### Stress Lane (Label-Gated)

**Runs on**: PRs with `ci:stress` label
**Command**: `cargo test -p perl-lsp --features stress-tests --locked`

Long-running stability tests, memory pressure tests, and performance benchmarks. Gated because they can take 10+ minutes and burn CI minutes.

Tests in this lane:
- `lsp_stress_tests.rs`
- `lsp_memory_pressure.rs`
- `lsp_performance_benchmarks.rs`

### Extras Lane (Label-Gated)

**Runs on**: PRs with `ci:extras` label
**Command**: `cargo test -p perl-lsp --features lsp-extras --locked`

Optional LSP features that aren't part of the core protocol. These tests are informational.

Tests in this lane:
- `lsp_new_features_tests.rs`
- `lsp_advanced_features_test.rs`

### Security Lane (Label-Gated)

**Runs on**: PRs with `ci:stress` label
**Reason**: Some security tests can hang on malformed input

Tests in this lane:
- `lsp_security_edge_cases.rs`
- `lsp_protocol_violations.rs`

**Known Issue**: The test harness `read_response()` can block forever on malformed requests. Until we add `read_response_timeout()` handling, these tests are gated.

## CI Budget Controls

### Concurrency Cancellation

All workflows now include:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

This means pushing 5 times only pays for the last run, not all 5.

### Path Filters

Most workflows exclude docs-only changes:

```yaml
paths-ignore:
  - 'docs/**'
  - '**/*.md'
  - '.claude/**'
```

### Label-Gating

Expensive jobs require explicit labels:

- `ci:bench` - Performance benchmarks
- `ci:coverage` - Code coverage analysis
- `ci:mutation` - Mutation testing
- `ci:semver` - API compatibility checks
- `ci:strict` - Strict quality checks
- `ci:stress` - Stress/security tests
- `ci:extras` - Extra LSP features
- `ci:all-tests` - Comprehensive test suite

## Adding New Tests

### Default Lane (Always Runs)

Place test in the appropriate crate's `tests/` directory. No feature flag needed.

```rust
#[test]
fn test_my_feature() {
    // This runs on every PR
}
```

### Gated Lane (Opt-In)

Add feature flag to `Cargo.toml`:

```toml
[features]
stress-tests = []
```

Guard your test:

```rust
#[test]
#[cfg(feature = "stress-tests")]
fn test_stress_scenario() {
    // Only runs with ci:stress label
}
```

## Troubleshooting

### CI Hangs

1. Check if test is in security/protocol lane
2. Add explicit timeout: `read_response_timeout(server, Duration::from_secs(5))`
3. Consider gating under `stress-tests` feature

### CI Minutes Burning

1. Check for missing `concurrency` block
2. Check for missing `paths-ignore`
3. Consider if test needs to be in default lane

### Flaky Tests

1. Use `RUST_TEST_THREADS=2` for LSP tests
2. Use adaptive timeouts from `common/mod.rs`
3. Consider moving to stress lane if inherently unstable

## Workflow Reference

| Workflow | Trigger | Lane |
|----------|---------|------|
| `ci.yml` | PR to master | Core + LSP |
| `lsp-tests.yml` | Code changes | LSP (matrix) |
| `test.yml` | Code changes | Core (matrix) |
| `property-tests.yml` | Code changes | Property tests |
| `ci-expensive.yml` | Labels | Bench, Mutation |
| `quality-checks.yml` | Code changes | Quality gates |
| `nightly.yml` | Schedule | Deep tests |
