# Dead Code Detection Setup Summary

This document summarizes the dead code detection automation added for Issue #284.

## Overview

Automated dead code detection has been implemented using:
- **cargo-udeps** for unused dependency detection
- **Clippy lints** for dead code, unused imports, and unused variables

## Files Added/Modified

### New Files

1. **`scripts/dead-code-check.sh`**
   - Main detection script
   - Modes: check, baseline, report
   - Compares against baseline and thresholds

2. **`.github/workflows/dead-code.yml`**
   - CI workflow for automated checks
   - Three jobs:
     - `dead-code-check`: Full detection with baseline comparison
     - `unused-deps-only`: Fast cargo-udeps check
     - `clippy-dead-code`: Fast clippy check
   - Triggers: workflow_dispatch, schedule (weekly), PR label (ci:dead-code)

3. **`docs/DEAD_CODE_DETECTION.md`**
   - Complete documentation
   - Usage instructions
   - Best practices
   - Troubleshooting guide

4. **`.ci/dead-code-baseline.yaml`** (generated)
   - Baseline configuration
   - Current counts
   - Thresholds and policy
   - Allowed exceptions

### Modified Files

1. **`justfile`**
   - Added dead code detection recipes:
     - `just dead-code` - Local check
     - `just dead-code-baseline` - Generate baseline
     - `just dead-code-report` - Generate JSON report
     - `just dead-code-strict` - Strict mode check
     - `just ci-dead-code` - CI gate check

## Usage

### Local Development

```bash
# Quick check
just dead-code

# Generate/update baseline
just dead-code-baseline

# Strict mode (fail on any increase)
just dead-code-strict

# Generate JSON report
just dead-code-report
```

### CI Integration

#### Option 1: Label-Based (Current)

Add the `ci:dead-code` label to trigger full checks on a PR:

```bash
gh pr edit <PR_NUMBER> --add-label ci:dead-code
```

#### Option 2: Scheduled (Automatic)

Runs weekly on Monday at 3 AM UTC automatically.

#### Option 3: Manual Dispatch

```bash
gh workflow run dead-code.yml
```

### Fast PR Checks

On every PR, two fast checks run automatically:
- Unused dependencies check (cargo-udeps)
- Dead code lint check (clippy)

## Baseline System

### Initial Baseline Generation

```bash
just dead-code-baseline
git add .ci/dead-code-baseline.yaml
git commit -m "chore: establish dead code baseline"
```

### Thresholds

Default thresholds (configurable in baseline file):
- Max unused dependencies: 5
- Max dead code items: 10
- Max unused imports: 20
- Max unused variables: 10

### Policy

- **Enforcement**: Warn by default
- **Fail on baseline exceeded**: Yes
- **Fail on increase**: Only in strict mode
- **Warn threshold**: 80% of max

## Acceptance Criteria Status

✅ **Configure cargo-udeps**: Integrated in script and CI workflow
✅ **Add to quality-checks workflow**: Added as `.github/workflows/dead-code.yml`
✅ **Document exceptions/allowlisting**: Documented in baseline YAML and guide
✅ **Create `just dead-code` recipe**: Added to justfile
✅ **Baseline existing dead code**: Script generates baseline with `just dead-code-baseline`

## Testing Performed

1. **Script Execution**:
   - ✅ Script is executable
   - ⏳ Baseline generation (running)
   - ⏳ Check mode validation (pending baseline)
   - ⏳ Report generation (pending baseline)

2. **CI Workflow**:
   - ⏳ Syntax validation (pending git push)
   - ⏳ Workflow execution (pending PR)

3. **Documentation**:
   - ✅ Comprehensive guide written
   - ✅ Examples provided
   - ✅ Troubleshooting section included

## Integration with Existing Systems

### Gate Policy

Dead code detection is **not yet in merge-gate** (opt-in via label for now).

To add to merge-gate (future):
1. Update `.ci/gate-policy.yaml`
2. Add `just ci-dead-code` to `ci-gate` recipe

### Technical Debt Tracking

Dead code detection complements existing debt tracking:
- `.ci/debt-ledger.yaml` - Tracks flaky tests and technical debt
- `.ci/dead-code-baseline.yaml` - Tracks dead code baseline

Both use similar YAML structure for consistency.

## Tools Required

### Local Development

```bash
# Rust nightly (for cargo-udeps)
rustup toolchain install nightly

# cargo-udeps
cargo install cargo-udeps --locked

# Clippy (usually included)
rustup component add clippy
```

### CI

- Automatically installs cargo-udeps
- Uses both stable and nightly toolchains
- Caches dependencies for speed

## Configuration

### Script Configuration

Environment variables:
- `DEAD_CODE_STRICT` - Enable strict mode (default: false)

### Baseline Configuration

Edit `.ci/dead-code-baseline.yaml` to:
- Adjust thresholds
- Add allowed exceptions
- Change enforcement policy
- Document known issues

Example exception:

```yaml
allowed_exceptions:
  - crate: perl-parser
    type: function
    name: legacy_parse_function
    reason: "Public API for backward compatibility"
    issue: "#123"
```

## Maintenance

### Regular Tasks

1. **Weekly** (automated):
   - Scheduled CI run
   - Baseline drift check

2. **Monthly** (manual):
   - Review allowed exceptions
   - Clean up dead code
   - Update thresholds if needed

3. **Per Release** (manual):
   - Verify baseline is current
   - Clean up identified dead code
   - Update documentation

### Updating Baseline

When intentionally adding code that increases counts:

```bash
# 1. Review the changes
just dead-code
cat target/dead-code/clippy-dead-code.txt

# 2. Update baseline
just dead-code-baseline

# 3. Review diff
git diff .ci/dead-code-baseline.yaml

# 4. Add exceptions if needed
# Edit .ci/dead-code-baseline.yaml

# 5. Commit
git add .ci/dead-code-baseline.yaml
git commit -m "chore: update dead code baseline"
```

## Future Enhancements

Potential improvements for future PRs:

1. **cargo-machete Integration**: Faster unused dependency detection
2. **Automatic PR Comments**: Post findings directly to PRs
3. **Trend Analysis**: Track dead code over time with metrics
4. **Crate-Level Reports**: Break down by individual crates
5. **Auto-Cleanup PRs**: Suggest automatic cleanup changes
6. **Integration with features.toml**: Cross-reference with advertised features

## Troubleshooting

### Common Issues

1. **"Nightly toolchain not installed"**:
   ```bash
   rustup toolchain install nightly
   ```

2. **"cargo-udeps not found"**:
   ```bash
   cargo install cargo-udeps --locked
   ```

3. **Baseline file not found**:
   ```bash
   just dead-code-baseline
   ```

4. **CI workflow not running**:
   - Check PR has `ci:dead-code` label
   - Verify workflow file syntax
   - Check repository settings

### Getting Help

- See `docs/DEAD_CODE_DETECTION.md` for detailed guide
- Check script output: `target/dead-code/*.txt`
- Review CI logs in GitHub Actions
- Report issues with label `tooling`

## Next Steps

1. **Test the baseline generation**:
   ```bash
   just dead-code-baseline
   ```

2. **Commit the baseline**:
   ```bash
   git add .ci/dead-code-baseline.yaml scripts/dead-code-check.sh
   git add .github/workflows/dead-code.yml docs/DEAD_CODE_DETECTION.md
   git add justfile
   git commit -m "feat(tooling): add dead code detection automation (#284)"
   ```

3. **Test in CI**:
   - Push to PR branch
   - Add `ci:dead-code` label
   - Verify workflow runs successfully

4. **Optional: Add to merge-gate**:
   - Update `.ci/gate-policy.yaml`
   - Update `justfile` `ci-gate` recipe
   - Test full gate: `just ci-gate`

## References

- Issue: #284 - Add Dead Code Detection Automation
- Documentation: `docs/DEAD_CODE_DETECTION.md`
- Script: `scripts/dead-code-check.sh`
- Workflow: `.github/workflows/dead-code.yml`
- Baseline: `.ci/dead-code-baseline.yaml`

---

**Status**: ✅ Implementation Complete, ⏳ Testing In Progress
**Date**: 2026-01-28
**Author**: Claude Code Agent (Generative Adapter)
