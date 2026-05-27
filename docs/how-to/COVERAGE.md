# Code Coverage

This document describes the code coverage infrastructure for perl-lsp.

## Overview

The perl-lsp project uses **cargo-llvm-cov** for code coverage generation and **Codecov** for coverage aggregation, trending, and PR reporting.

Branch coverage is enabled in the coverage lane with `cargo-llvm-cov --branch`. That requires a nightly Rust toolchain because LLVM branch coverage uses unstable `-Z coverage-options=branch` support.

The initial PR slice keeps the branch gate on library/unit tests (`--lib`) so it stays stable on current master. Integration snapshot suites can still be run separately outside the coverage lane.

## Quick Start

### Local Coverage Reports

Generate an HTML coverage report locally:

```bash
rtk just coverage
```

This will:
1. Install `cargo-llvm-cov` if not present
2. Use a nightly Rust toolchain for branch coverage support
3. Run tests with coverage instrumentation
4. Generate an HTML report at `target/coverage/index.html`
5. Attempt to open the report in your browser

### Terminal Summary

Get a quick coverage summary in the terminal:

```bash
rtk just coverage-summary
```

### LCOV Format (for CI)

Generate coverage in LCOV format (compatible with Codecov):

```bash
rtk just coverage-lcov
```

This creates `lcov.info` in the project root.

## CI Integration

### Automatic Coverage Reports

Patch coverage is the front-door PR coverage gate in Codecov policy:

- **Patch coverage**: Target `95%` with `0%` threshold.
- **Project coverage**: Target `95%`, informational during burn-down.
- **Coverage scope**: Workspace policy includes proof-rail `xtask/src/` code.

Workflow wiring remains a separate follow-up slice. This page documents the Codecov policy posture, coverage receipt, and local patch coverage quality-gate commands.
Project coverage remains informational during burn-down.

### Patch Coverage Quality Gate

Generate the coverage receipt from LCOV before running the patch gate:

```bash
rtk cargo xtask coverage-baseline --lcov target/lcov.info --receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml --patch-coverage <patch-percent>
```

Then run the patch coverage quality gate:

```bash
rtk cargo xtask quality-gate --mode enforce-patch-coverage --coverage-receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml
```

Use `--check` on either command to validate existing receipts instead of rewriting them. A failing patch gate writes JSON and Markdown receipts under `target/receipts/quality/` and names the missing proof:

- `coverage_receipt_not_current` means the coverage receipt is missing or stale for the current commit.
- `patch_coverage_unknown` means the receipt is current but does not contain a patch coverage percentage.
- `patch_coverage_below_target` means patch coverage is below 95%.

Failure output includes sample uncovered lines and repair guidance. Treat those as behavior-oriented tests to add around error paths, boundaries, config parsing, serialization, cancellation, and output contracts, not as a prompt to add line-touch tests.

### Viewing Coverage in PRs

When Codecov reports on a PR:

1. The nightly coverage job runs automatically
2. Coverage data is uploaded to Codecov
3. Codecov posts a comment to the PR showing:
   - Overall coverage percentage
   - Coverage diff (lines added/removed)
   - Per-file coverage changes
   - Flags for each crate (parser, lsp, lexer, dap, corpus)

The nightly coverage lane also generates branch coverage in `lcov.info` and checks it against `.ci/coverage-baseline.txt`. That lane remains a ratchet: branch coverage can stay flat or improve, and it fails if the total drops by more than the allowed percentage point budget.

### Coverage Badge

The README includes a Codecov badge showing the current coverage on the `master` branch:

[![codecov](https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/perl-lsp)

## Coverage Thresholds

Coverage thresholds are defined in `codecov.yml`:

| Target | Threshold | Notes |
|--------|-----------|-------|
| **Patch** | 95% | Blocking PR gate with `0%` threshold |
| **Project** | 95% | Informational during burn-down; final target is blocking |

### Threshold Philosophy

- **95% for patches**: New code must carry its own proof.
- **95% project target**: Project coverage remains transitional until the burn-down closes.
- **No per-flag targets**: Codecov status targets live under project and patch status blocks.

Branch coverage uses the checked-in policy file instead of Codecov status thresholds for the initial PR slice:

- `.ci/coverage-baseline.txt` stores the current branch-coverage baseline and regression budget
- `scripts/check-coverage-baseline.sh` compares the generated `lcov.info` against that baseline
- `scripts/update-coverage-baseline.sh` refreshes the checked-in baseline after an intentional improvement
- `target_branch_coverage` documents the intended longer-term 80% floor
- `.ci/README-coverage.md` is the terse operator note for the gate and baseline workflow

## Exclusions

The following paths are excluded from coverage analysis (configured in `codecov.yml`):

- `archive/**` - Legacy code
- `crates/tree-sitter-perl-rs/**` - C-based legacy parser
- `crates/tree-sitter-perl-c/**` - C bindings wrapper
- `crates/*/tests/**` - Test code
- `crates/*/benches/**` - Benchmark code
- `crates/*/examples/**` - Example code
- `crates/*/build.rs` - Build scripts
- `fuzz/**` - Fuzzing infrastructure
- `**/*_generated.rs` - Generated code

`xtask/**` is not excluded because proof-rail CLI and receipt code must stay visible to patch coverage.

## Coverage Workflow

The nightly coverage job (`.github/workflows/ci-nightly.yml`, `test-coverage`) performs these steps:

1. **Install toolchain**: Rust stable with `llvm-tools-preview` component
2. **Install cargo-llvm-cov**: Using `taiki-e/install-action@v2` for speed
3. **Cache dependencies**: Uses `Swatinem/rust-cache@v2` for faster builds
4. **Create fixtures**: Legacy LSP test fixtures (if needed)
5. **Generate coverage**: Run tests with LLVM instrumentation and branch coverage
6. **Display summary**: Show coverage summary in logs
7. **Check baseline**: Compare branch coverage to `.ci/coverage-baseline.txt`
8. **Upload to Codecov**: Upload `lcov.info` to Codecov service
9. **Archive report**: Save `lcov.info` as workflow artifact (30-day retention)

### Environment Variables

The workflow uses optimized settings:

- `RUSTFLAGS="-Copt-level=1"` - Fast builds with basic optimization
- `CARGO_BUILD_JOBS=2` - Limit parallelism to avoid memory pressure
- `RUST_TEST_THREADS=2` - Adaptive threading for LSP tests

## Codecov Repository Secret

Set this GitHub Actions secret at the repository or organization level:

- `CODECOV_TOKEN`

The coverage and test-results uploads use this token. Codecov upload failures are configured as non-blocking so service outages or missing token setup do not block merge-gate CI.

## Configuration Files

### codecov.yml

The `codecov.yml` file at the project root configures:

- Coverage precision and rounding
- Project and patch coverage thresholds
- Exclusion patterns
- Per-crate flags for detailed tracking
- PR comment layout

### .github/workflows/ci-nightly.yml

The coverage workflow file defines:

- When coverage runs (push to main, PR labels, manual dispatch)
- Build and test environment
- Upload configuration
- Artifact retention

## Troubleshooting

### cargo-llvm-cov not found

The `just coverage` recipes automatically install `cargo-llvm-cov` if missing. To install manually:

```bash
rtk cargo install cargo-llvm-cov --locked
```

### Coverage report generation fails

Some tests may be flaky under coverage instrumentation due to timing or memory constraints. The workflow uses:

- Lower optimization (`-Copt-level=1`) for faster builds
- Limited parallelism (`CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`)

If branch coverage is involved, make sure you are using `cargo +nightly llvm-cov` or a nightly-installed Rust toolchain. Stable rustc cannot compile the `--branch` instrumentation path.
If your shell does not proxy `cargo +nightly` correctly on Windows, use `rustup run nightly cargo ...` instead.

### Codecov upload fails

The workflow sets `fail_ci_if_error: false` so Codecov upload failures don't block PRs. Check:

1. Codecov service status: https://status.codecov.io/
2. Workflow logs for upload details
3. Codecov dashboard for processing status

### Test Analytics upload fails

Check:

1. `CODECOV_TOKEN` is configured as a GitHub Actions secret.
2. The JUnit file exists under `target/test-results/`.
3. The receipt JSON exists under `target/receipts/`.
4. The Codecov upload step is non-blocking, so failures should be visible in logs without failing the gate.

### Coverage numbers look wrong

Coverage excludes:
- Test code itself (`tests/`, `benches/`)
- Legacy/archived code (`archive/`, `tree-sitter-perl-rs/`)
- Generated code (`*_generated.rs`)

To see what's included, check the exclusion patterns in `codecov.yml`.

## Best Practices

### Writing Testable Code

To improve coverage:

1. **Prefer small, focused functions** - Easier to test comprehensively
2. **Use Result/Option patterns** - Test both success and error paths
3. **Avoid unwrap/expect** - Return Result and test error cases
4. **Extract complex logic** - Make it testable in isolation

### Reviewing Coverage

When reviewing PRs with coverage:

1. Check the Codecov PR comment for coverage delta
2. Look for uncovered lines in changed files
3. Ask: "Are the uncovered lines error paths that should be tested?"
4. Don't aim for 100% - focus on meaningful test coverage

For branch coverage, pay attention to error paths and recovery branches. A file can have acceptable line coverage while still leaving branches untested.

### Coverage-Driven Development

Use coverage to find gaps:

```bash
# Generate HTML report
rtk just coverage

# Open target/coverage/index.html
# Find uncovered lines (red highlighting)
# Write tests for uncovered behavior
# Re-run coverage to verify
```

## Integration with CI Gates

Patch coverage is the Codecov PR coverage gate for new code. Project coverage remains informational during burn-down and is promoted to blocking after the project reaches the `95%` target.

Branch coverage in the parser coverage lane is enforced separately through the baseline ratchet in `.ci/coverage-baseline.txt`.

## Test Analytics

Codecov Test Analytics is populated from CI receipt artifacts.

The CI gates already emit structured receipts under `target/receipts/`. The helper script `scripts/ci/receipts-to-junit.py` converts those receipts into JUnit XML and uploads them with `codecov/test-results-action@v1`.

Current uploaded suites:

- `pr-fast` — PR smoke test results
- `gate-<shard>` — Merge gate shard results (meta, foundation, analysis, lsp, support, corpus, policy)
- `ux-regression` — UX regression test results from main CI
- `ux-regression-gate` — UX regression gate results (standalone workflow)

This avoids rerunning tests solely to produce JUnit XML and keeps the existing `xtask gates` runner as the source of truth.

## Bundle Analysis

Codecov Bundle Analysis is intentionally deferred.

The VS Code extension currently compiles TypeScript and packages a VSIX. It does not use Vite, Rollup, Webpack, Remix, Nuxt, SvelteKit, or SolidStart as its production bundler.

Do not add `@codecov/vite-plugin` until the extension adopts a supported JavaScript bundler. When that happens, add the Codecov bundler plugin to the actual bundler config and use `CODECOV_TOKEN` for upload.

## Related Documentation

- [CLAUDE.md](../../CLAUDE.md) - Project commands and structure
- [CONTRIBUTING.md](../../CONTRIBUTING.md) - Contribution guidelines
- [CURRENT_STATUS.md](../project/CURRENT_STATUS.md) - Project metrics and health

## References

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - Coverage tool
- [Codecov Documentation](https://docs.codecov.com/) - Service documentation
- [Rust instrumentation-based coverage](https://doc.rust-lang.org/rustc/instrument-coverage.html) - rustc coverage
