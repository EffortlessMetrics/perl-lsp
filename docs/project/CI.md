# Continuous Integration (CI)

This project uses GitHub Actions with a lean, stable CI configuration optimized for fast feedback and reliable builds.

## Runner Versions

We **pin** GitHub-hosted runners to specific images for stability and predictability:

- **Linux:** `ubuntu-22.04`
- **Windows:** `windows-2022`
- **macOS:** Disabled by default (can be enabled via `ci:mac` label)

**Important:** Please avoid switching these to `*-latest`, as that can introduce breaking changes without notice. Pinned versions ensure consistent build environments and prevent unexpected failures.

## What Runs by Default?

On pull requests, the default jobs include:

### Core Checks (test.yml)
- **Format check**  `cargo fmt --check` ensures consistent code formatting
- **Tautology detection**  Prevents test logic errors
- **Build verification**  Compiles all crates and LSP server
- **Core tests**  via `cargo nextest` with lean build flags:
  - `RUSTFLAGS="-Cdebuginfo=0 -Copt-level=1 --cfg ci"`
  - `CARGO_BUILD_JOBS=2` (prevents linker memory pressure)
- **Test discovery**  Validates test count hasn't regressed (baseline: 720 tests)
- **Incremental tests**  Feature-gated incremental parsing tests

### LSP Tests (lsp-tests.yml)
- **Clippy**  First-party lints with `-D warnings` (properly shell-escaped for Windows)
- **LSP tests**  Full LSP protocol validation with nextest
- **Parser tests**  Comprehensive parser test suite
- **Health checks**  LSP binary health verification

### Quality Checks (quality-checks.yml)
- **Tautology detection**  Test pattern validation
- **Security audit**  `cargo audit` for dependency vulnerabilities
- **Test determinism**  Runs tests 3 times to ensure reproducible results

These typically complete in **~810 minutes** with caching.

## Opt-in CI Jobs via Labels

To keep PRs fast by default, expensive jobs are gated behind labels. Apply these labels to your PR to opt in:

- **`ci:coverage`**  Run code coverage analysis with `cargo-llvm-cov` (generates lcov reports)
- **`ci:bench`**  Run performance benchmarks (nightly toolchain, heavier runtime)
- **`ci:mutation`**  Run mutation testing with `cargo-mutants` (long-running; informational only)
- **`ci:strict`**  Enable stricter Clippy lints (`pedantic`, `nursery`, `cargo`)
- **`ci:semver`**  Check API compatibility with `cargo-semver-checks`
- **`ci:mac`**  Enable macOS jobs (useful for platform-specific changes)

> **Note:** Some heavy jobs may be skipped unless the corresponding label is present.
> If your change impacts parsing/grammar, core infrastructure, or platform-specific code, please add the appropriate labels to your PR.

## Build Optimizations

### Lean Build Flags
All CI jobs use optimized build settings to prevent resource exhaustion:

```bash
RUSTFLAGS="-Cdebuginfo=0 -Copt-level=1 --cfg ci"
CARGO_BUILD_JOBS=2
```

These flags:
- Reduce debug info size (saves ~50% link time)
- Use minimal optimization (faster builds, adequate for testing)
- Enable CI-specific test guards via `--cfg ci`
- Limit parallel build jobs to prevent OOM on GitHub runners

### Nextest Configuration
We use [cargo-nextest](https://nexte.st/) for faster, more reliable test runs:

```toml
# .config/nextest.toml
[profile.ci]
fail-fast = false          # Continue on failures for complete reporting
retries = 1                # Retry flaky tests once
slow-timeout = "60s"       # Warn after 60s
terminate-after = "120s"   # Kill hanging tests at 120s
```

### Test Guards
Some slow or fragile tests are skipped in CI using `#[cfg_attr(ci, ignore)]`:

```rust
#[cfg_attr(ci, ignore)]
#[test]
fn slow_integration_test() {
    // This test runs locally but is skipped in CI
}
```

## Troubleshooting

### Exit code 143 (SIGTERM)
**Cause:** The GitHub runner was terminated, often due to:
- Job timeout (default 360 minutes, but can be lower with resource constraints)
- Memory exhaustion (LLD linker issues on Linux)
- Hanging tests without proper timeouts

**Solution:**
1. Check if the job is using lean build flags (`CARGO_BUILD_JOBS=2`, minimal debug info)
2. Verify nextest is being used with proper timeouts
3. Look for tests that might be hanging (add `timeout` command or nextest slow-timeout)
4. Re-run the job (transient runner issues are common)

### Windows bash steps failing
**Cause:** PowerShell treats `--` as a comment delimiter, breaking cargo commands like:
```bash
cargo clippy -- -D warnings  # PowerShell sees: cargo clippy
```

**Solution:** All bash-specific steps now explicitly set `shell: bash`:
```yaml
- name: Clippy
  shell: bash  # Force bash even on Windows
  run: cargo clippy -- -D warnings
```

### macOS build failures with procfs
**Cause:** The `procfs` crate (used in xtask memory profiling) is Linux-only.

**Solution:** Code is now properly gated:
```rust
#[cfg(target_os = "linux")]
use procfs::process::Process;

#[cfg(target_os = "linux")]
fn get_memory_usage() -> Result<f64> { /* ... */ }

#[cfg(not(target_os = "linux"))]
fn get_memory_usage() -> Result<f64> { Ok(0.0) }  // Fallback
```

### Test discovery regression
**Cause:** The baseline test count (720 tests) has dropped by more than 5%.

**Solution:**
1. Check if test files were accidentally deleted or moved
2. Verify `#[test]` attributes are present
3. Check if tests are being filtered out by new `#[ignore]` attributes
4. Review changes to test discovery logic in `test.yml`

### Determinism test failures
**Cause:** Tests produce different output across runs (non-deterministic behavior).

**Solution:**
1. Check for random number generation without seeds
2. Look for time-based assertions
3. Verify thread-safe access to shared state
4. Check for filesystem dependencies that might change between runs

## Performance Benchmarks

The CI tracks several key metrics:

- **Test execution time:** ~8-10 minutes for full suite
- **LSP tests:** ~30-60 seconds with nextest and RUST_TEST_THREADS=2
- **Parser tests:** ~2-4 minutes with comprehensive coverage
- **Test count baseline:** 720+ tests (enforced with 5% tolerance)

## CI Architecture

### Workflow Organization
- **test.yml**  Core functionality tests (Linux + Windows)
- **lsp-tests.yml**  LSP protocol and integration tests
- **quality-checks.yml**  Advanced quality gates (label-gated)

### Caching Strategy
All workflows cache:
- Cargo registry (`~/.cargo/registry`)
- Cargo index (`~/.cargo/git`)
- Build artifacts (`target/`)

Cache keys include `Cargo.lock` hash for automatic invalidation.

### Matrix Strategy
Default matrix: Linux (ubuntu-22.04) + Windows (windows-2022)
- Stable + Beta toolchains tested
- Nightly toolchain optional (experimental, allowed to fail)

## Future Enhancements

Planned improvements (tracked separately):

1. **sccache integration**  Shared compiler cache for faster cold builds
2. **Incremental macOS testing**  Conditional macOS jobs with `ci:mac` label
3. **Test parallelization tuning**  Adaptive RUST_TEST_THREADS based on runner capacity
4. **Benchmark regression tracking**  Automated performance comparison against baseline

---

**Last Updated:** 2025-11-10
**Related:** [CONTRIBUTING.md](../CONTRIBUTING.md), [COMMANDS_REFERENCE.md](./COMMANDS_REFERENCE.md)
