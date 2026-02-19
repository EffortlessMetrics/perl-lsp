# Dead Code Detection

This document describes the automated dead code detection system for the perl-lsp project.

## Overview

The dead code detection system helps keep the codebase clean by identifying:

- **Unused dependencies** - Crate dependencies that are declared but never used
- **Dead code** - Functions, types, and modules that are defined but never called
- **Unused imports** - Import statements that aren't referenced
- **Unused variables** - Variables that are declared but never used

## Tools Used

### 1. cargo-udeps

**Purpose**: Detects unused dependencies in Cargo.toml files.

**Requirements**:
- Rust nightly toolchain
- `cargo install cargo-udeps --locked`

**Usage**:
```bash
# Check all workspace members
cargo +nightly udeps --workspace --all-targets --locked
```

### 2. Clippy dead_code Lints

**Purpose**: Detects dead code, unused imports, and unused variables in source code.

**Requirements**:
- Rust stable/nightly with clippy component
- `rustup component add clippy`

**Usage**:
```bash
# Run clippy with dead code warnings
cargo clippy --workspace --lib --bins --locked \
  -- -W dead_code -W unused_imports -W unused_variables
```

## Local Development Workflow

### Quick Check

Run dead code detection locally:

```bash
just dead-code
```

This will:
1. Check for unused dependencies with cargo-udeps
2. Check for dead code with clippy
3. Compare against baseline (if exists)
4. Report any issues found

### Generate Baseline

When starting fresh or after intentional changes:

```bash
just dead-code-baseline
```

This creates/updates `.ci/dead-code-baseline.yaml` with current counts.

### Strict Mode

Fail on any increase from baseline:

```bash
just dead-code-strict
```

### Generate Report

Generate a JSON report for analysis:

```bash
just dead-code-report
```

Report is saved to `target/dead-code/report.json`.

## CI Integration

### Automated Checks

Dead code detection runs in CI:

1. **On Pull Requests** (with `ci:dead-code` label):
   - Full dead code detection
   - Baseline comparison
   - Report uploaded as artifact

2. **On Schedule** (weekly):
   - Full scan to catch drift
   - Updates baseline if needed

3. **Fast PR Checks** (all PRs):
   - `unused-deps-only`: Quick cargo-udeps check
   - `clippy-dead-code`: Quick clippy dead code lint

### Workflow Files

- `.github/workflows/dead-code.yml` - Main dead code detection workflow
- `scripts/dead-code-check.sh` - Detection script
- `.ci/dead-code-baseline.yaml` - Baseline configuration

### Triggering CI Checks

#### Option 1: Add Label

Add the `ci:dead-code` label to your PR:

```bash
gh pr edit <PR_NUMBER> --add-label ci:dead-code
```

#### Option 2: Manual Dispatch

Trigger the workflow manually:

```bash
gh workflow run dead-code.yml
```

## Baseline Management

### Baseline File Structure

`.ci/dead-code-baseline.yaml` contains:

```yaml
schema_version: 1
last_updated: "2026-01-28"

thresholds:
  max_unused_dependencies: 5
  max_dead_code_items: 10
  max_unused_imports: 20
  max_unused_variables: 10

baseline:
  unused_dependencies: 2
  dead_code_items: 3
  unused_imports: 8
  unused_variables: 1

allowed_exceptions:
  # Items that are intentionally unused
  - crate: perl-parser
    type: function
    name: legacy_parse_function
    reason: "Public API for backward compatibility"

policy:
  enforcement: warn
  fail_on_baseline_exceeded: true
  warn_threshold_percent: 80
```

### Updating the Baseline

When you intentionally add code that increases dead code counts:

1. **Review the changes**: Ensure the increase is justified
2. **Update baseline**: Run `just dead-code-baseline`
3. **Commit the update**: Include the updated baseline file in your PR
4. **Document the reason**: Add exceptions to `allowed_exceptions` if needed

### Threshold Policy

- **max_unused_dependencies**: Maximum allowed unused dependencies (default: 5)
- **max_dead_code_items**: Maximum allowed dead code items (default: 10)
- **max_unused_imports**: Maximum allowed unused imports (default: 20)
- **max_unused_variables**: Maximum allowed unused variables (default: 10)

If current counts exceed thresholds, CI will fail.

## Allowed Exceptions

Some code may be intentionally unused but should remain:

### Public API

Functions/types that are part of the public API but not used internally:

```yaml
allowed_exceptions:
  - crate: perl-parser
    type: function
    name: parse_with_custom_config
    reason: "Public API for advanced users"
```

### Feature-Gated Code

Code used only in specific feature configurations:

```yaml
allowed_exceptions:
  - crate: perl-lsp
    type: module
    name: experimental_features
    reason: "Used when 'experimental' feature is enabled"
```

### Platform-Specific Code

Code used only on certain platforms:

```yaml
allowed_exceptions:
  - crate: perl-lsp
    type: function
    name: windows_specific_handler
    reason: "Windows-only functionality"
```

### Dependencies Used in Build Scripts

Dependencies that are only used in build.rs:

```yaml
allowed_exceptions:
  - crate: perl-parser
    dependency: bindgen
    reason: "Used in build.rs for FFI bindings"
```

## Common Issues and Solutions

### Issue: cargo-udeps Requires Nightly

**Problem**: cargo-udeps only works with nightly Rust.

**Solution**:
```bash
# Install nightly toolchain
rustup toolchain install nightly

# Run with nightly
cargo +nightly udeps --workspace
```

### Issue: False Positives for Feature-Gated Code

**Problem**: Code used only in certain features appears as dead code.

**Solution**: Add to `allowed_exceptions` in baseline file with clear documentation.

### Issue: Build Dependencies Flagged as Unused

**Problem**: Dependencies used in build.rs are flagged by cargo-udeps.

**Solution**: This is expected behavior. Add to `allowed_exceptions` with reason.

### Issue: Test-Only Imports Flagged

**Problem**: Imports used only in tests show as unused in library code.

**Solution**: This is expected. The check runs on `--lib --bins` only, not tests.

### Issue: Baseline Drift

**Problem**: Baseline becomes outdated over time.

**Solution**:
- Schedule weekly runs update baseline automatically
- Manual refresh: `just dead-code-baseline`
- Review and commit the updated baseline

## Integration with CI Gate

### Adding to Merge Gate

To make dead code detection required for merging:

1. **Update gate policy** (`.ci/gate-policy.yaml`):

```yaml
gates:
  - name: dead_code
    tier: merge_gate
    description: "Check for dead code and unused dependencies"
    required: true
    command: just ci-dead-code
    timeout_seconds: 300
```

2. **Update justfile** (`ci-gate` recipe):

```bash
ci-gate:
    @echo "Running fast merge gate..."
    just ci-workflow-audit && \
    # ... other checks ...
    just ci-dead-code && \
    # ... remaining checks ...
```

### Current Status

As of January 2026:
- ✅ Dead code detection script implemented
- ✅ Justfile recipes added
- ✅ CI workflow configured
- ✅ Baseline system established
- ⏳ **Not yet in merge-gate** (opt-in via label)

## Best Practices

### 1. Check Locally Before Pushing

Always run `just dead-code` before pushing:

```bash
just dead-code
```

### 2. Review Before Updating Baseline

Don't blindly update the baseline. Review why counts increased:

```bash
# See what changed
git diff .ci/dead-code-baseline.yaml

# Review the actual dead code
cat target/dead-code/clippy-dead-code.txt
cat target/dead-code/udeps-output.txt
```

### 3. Clean Up Dead Code Regularly

Set a reminder to review and clean up dead code:

```bash
# Generate report
just dead-code-report

# Review findings
cat target/dead-code/report.json
```

### 4. Document Exceptions

Always document why code is intentionally unused:

```yaml
allowed_exceptions:
  - crate: my-crate
    type: function
    name: my_function
    reason: "Clear explanation of why this is unused but should remain"
    issue: "#123"  # Optional: link to tracking issue
```

### 5. Use Feature Flags Wisely

Minimize feature-gated code to reduce false positives:

- Keep feature-specific code isolated
- Document feature dependencies clearly
- Use `#[cfg(feature = "...")]` consistently

## Metrics and Monitoring

### Key Metrics

Track over time:
- Total unused dependencies
- Total dead code items
- Trend direction (increasing/decreasing)
- Time since last cleanup

### Alerts

The system alerts when:
- Current counts exceed thresholds
- Counts increase from baseline (in strict mode)
- Baseline is outdated (>30 days)

### Reporting

View current status:

```bash
# Terminal report
just dead-code

# JSON report
just dead-code-report
cat target/dead-code/report.json
```

## Troubleshooting

### Script Fails with "Nightly toolchain not installed"

Install nightly:
```bash
rustup toolchain install nightly
```

### cargo-udeps Not Found

Install cargo-udeps:
```bash
cargo install cargo-udeps --locked
```

### Baseline File Not Found

Generate a new baseline:
```bash
just dead-code-baseline
git add .ci/dead-code-baseline.yaml
git commit -m "chore: add dead code baseline"
```

### CI Workflow Not Running

Ensure:
1. Workflow file is in `.github/workflows/dead-code.yml`
2. PR has `ci:dead-code` label (if triggered by label)
3. Workflow is not disabled in repository settings

## Future Enhancements

Potential improvements:

1. **cargo-machete Integration**: Faster unused dependency detection
2. **Automatic PR Comments**: Post detailed findings to PR
3. **Trend Tracking**: Historical analysis of dead code over time
4. **Crate-Level Breakdown**: Per-crate dead code reports
5. **Auto-Cleanup Suggestions**: Generate automated cleanup PRs
6. **Integration with Features System**: Link to features.toml

## Related Documentation

- [CI Gate Policy](../.ci/gate-policy.yaml) - Gate configuration
- [Technical Debt Tracking](DEBT_TRACKING.md) - Flaky tests and debt management
- [Commands Reference](COMMANDS_REFERENCE.md) - All just commands

## References

- [cargo-udeps](https://github.com/est31/cargo-udeps) - Unused dependency detection
- [cargo-machete](https://github.com/bnjbvr/cargo-machete) - Alternative tool
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/index.html) - All clippy lints
- Issue [#284](https://github.com/EffortlessMetrics/perl-lsp/issues/284) - Original request
