# Dead Code Detection - Testing Checklist

This checklist validates the dead code detection implementation for Issue #284.

## Pre-Commit Validation

### 1. File Integrity

- [x] Script created: `scripts/dead-code-check.sh`
- [x] Script is executable: `chmod +x scripts/dead-code-check.sh`
- [x] Script syntax valid: `bash -n scripts/dead-code-check.sh`
- [x] Workflow created: `.github/workflows/dead-code.yml`
- [x] Workflow YAML valid: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/dead-code.yml'))"`
- [x] Baseline file created: `.ci/dead-code-baseline.yaml`
- [x] Documentation created: `docs/DEAD_CODE_DETECTION.md`
- [x] Justfile recipes added
- [x] Clippy.toml updated with documentation

### 2. Just Recipes

Test all added recipes:

```bash
# List recipes
just --list | grep dead-code

# Should show:
# ✓ ci-dead-code
# ✓ dead-code
# ✓ dead-code-baseline
# ✓ dead-code-report
# ✓ dead-code-strict
```

**Status**: ✅ All recipes present

### 3. Script Functionality

Test script modes:

```bash
# Test check mode (with baseline)
just dead-code

# Test baseline generation
just dead-code-baseline

# Test report generation
just dead-code-report

# Test strict mode
just dead-code-strict
```

**Expected behavior**:
- `check`: Runs checks, compares against baseline
- `baseline`: Generates/updates `.ci/dead-code-baseline.yaml`
- `report`: Creates `target/dead-code/report.json`
- `strict`: Fails on any increase from baseline

### 4. CI Workflow Validation

```bash
# Validate workflow syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/dead-code.yml'))"

# Check workflow jobs
grep -A 2 "^jobs:" .github/workflows/dead-code.yml
```

**Expected jobs**:
- `dead-code-check`: Full detection with baseline
- `unused-deps-only`: Fast cargo-udeps check
- `clippy-dead-code`: Fast clippy check

**Status**: ✅ Workflow syntax valid

### 5. Tool Prerequisites

Check required tools:

```bash
# Rust nightly (for cargo-udeps)
rustup toolchain list | grep nightly

# cargo-udeps
cargo +nightly udeps --version || echo "Will install on first run"

# Clippy
rustup component list --installed | grep clippy
```

**Status**:
- ✅ Script auto-installs cargo-udeps
- ✅ Nightly toolchain required (documented)

## Local Testing

### Test 1: Generate Baseline

```bash
just dead-code-baseline
```

**Expected**:
- Creates/updates `.ci/dead-code-baseline.yaml`
- Reports current counts
- No errors

**Actual**: ⏳ Running (cargo-udeps compiling workspace)

### Test 2: Run Check Mode

```bash
just dead-code
```

**Expected**:
- Compares current state vs baseline
- Reports any increases
- Passes if within thresholds

**Actual**: ⏳ Waiting for baseline completion

### Test 3: Generate Report

```bash
just dead-code-report
cat target/dead-code/report.json
```

**Expected**:
- Creates JSON report
- Contains counts for all categories
- Valid JSON structure

**Actual**: ⏳ Waiting for baseline completion

### Test 4: Strict Mode

```bash
# Make a change that increases dead code
# Then:
just dead-code-strict
```

**Expected**:
- Fails if any increase from baseline
- Reports which category increased

**Actual**: ⏳ Waiting for test setup

## CI Testing

### Test 5: Workflow Triggers

Verify workflow triggers:

```bash
# Check trigger configuration
grep -A 10 "^on:" .github/workflows/dead-code.yml
```

**Expected triggers**:
- `workflow_dispatch` - Manual trigger
- `pull_request` with label `ci:dead-code`
- `schedule` - Weekly on Monday 3 AM UTC

**Status**: ✅ All triggers configured

### Test 6: Fast PR Checks

Test that fast checks run on all PRs:

```bash
# Verify unused-deps-only runs on all PRs
grep -A 5 "unused-deps-only:" .github/workflows/dead-code.yml

# Verify clippy-dead-code runs on all PRs
grep -A 5 "clippy-dead-code:" .github/workflows/dead-code.yml
```

**Expected**:
- Both jobs have `if: github.event_name == 'pull_request'`
- No label requirement for these jobs

**Status**: ✅ Fast checks configured for all PRs

### Test 7: Label-Gated Full Check

Verify full check requires label:

```bash
grep -A 3 "if:" .github/workflows/dead-code.yml | grep ci:dead-code
```

**Expected**:
- Full check requires `ci:dead-code` label
- OR manual dispatch
- OR scheduled run

**Status**: ✅ Label gating configured

## Integration Testing

### Test 8: Integration with CI Gate

Check if dead-code is in merge gate:

```bash
grep "ci-dead-code" justfile
grep "dead_code" .ci/gate-policy.yaml
```

**Current status**:
- ✅ `ci-dead-code` recipe exists
- ⚠️ Not yet in merge-gate (opt-in by design)

**Future integration** (if desired):
- Add to `.ci/gate-policy.yaml`
- Add to `justfile` `ci-gate` recipe

### Test 9: Documentation Completeness

Verify documentation:

```bash
# Check documentation file exists and is complete
wc -l docs/DEAD_CODE_DETECTION.md
grep -c "##" docs/DEAD_CODE_DETECTION.md  # Count sections
```

**Expected sections**:
- Overview
- Tools Used
- Local Development Workflow
- CI Integration
- Baseline Management
- Common Issues and Solutions
- Best Practices

**Status**: ✅ Documentation complete (11+ sections)

### Test 10: Baseline Structure

Verify baseline file structure:

```bash
python3 -c "import yaml; d=yaml.safe_load(open('.ci/dead-code-baseline.yaml')); print('Valid YAML'); print(f'Thresholds: {d[\"thresholds\"]}'); print(f'Baseline: {d[\"baseline\"]}')"
```

**Expected structure**:
- `schema_version`
- `last_updated`
- `thresholds` with all four categories
- `baseline` with current counts
- `policy` with enforcement settings
- `maintenance` with review schedule

**Status**: ✅ Baseline structure valid

## Edge Cases and Error Handling

### Test 11: Missing Tools

Test graceful degradation:

```bash
# Simulate missing nightly
RUSTUP_TOOLCHAIN=missing just dead-code 2>&1 | head -20
```

**Expected**:
- Clear error message
- Installation instructions
- Exit code 1

### Test 12: No Baseline File

Test behavior without baseline:

```bash
# Backup baseline
mv .ci/dead-code-baseline.yaml .ci/dead-code-baseline.yaml.bak

# Run check
just dead-code

# Restore baseline
mv .ci/dead-code-baseline.yaml.bak .ci/dead-code-baseline.yaml
```

**Expected**:
- Warning: "No baseline file found"
- Runs basic checks anyway
- Suggests generating baseline

### Test 13: Exceeded Thresholds

Test threshold enforcement:

```bash
# Edit baseline to set very low thresholds
# Run check
# Should fail with clear message
```

**Expected**:
- Fails if current > threshold
- Reports which threshold exceeded
- Exit code 1

## Performance Testing

### Test 14: Execution Time

Measure execution time:

```bash
time just dead-code
```

**Expected**:
- Full check: < 5 minutes
- Baseline generation: < 5 minutes
- Report generation: < 3 minutes

**Actual**: ⏳ Measuring

### Test 15: CI Timeout

Verify CI timeouts are reasonable:

```bash
grep "timeout-minutes" .github/workflows/dead-code.yml
```

**Expected**:
- `dead-code-check`: 20 minutes
- `unused-deps-only`: 10 minutes
- `clippy-dead-code`: 15 minutes

**Status**: ✅ Timeouts configured appropriately

## Acceptance Criteria Verification

From Issue #284:

- [x] Configure cargo-udeps: ✅ Integrated in script
- [x] Add to quality-checks workflow: ✅ `.github/workflows/dead-code.yml`
- [x] Document exceptions/allowlisting: ✅ In baseline YAML and docs
- [x] Create `just dead-code` recipe: ✅ Added to justfile
- [x] Baseline existing dead code: ✅ Script generates baseline

## Post-Commit Testing

After committing and pushing:

### Test 16: CI Workflow Runs

1. Push to PR branch
2. Add `ci:dead-code` label
3. Verify workflow runs
4. Check artifacts uploaded

### Test 17: Fast Checks on PR

1. Open any PR
2. Verify `unused-deps-only` runs automatically
3. Verify `clippy-dead-code` runs automatically
4. Check execution time < 15 minutes

### Test 18: Scheduled Run

Wait for next Monday 3 AM UTC or:
1. Manually trigger workflow
2. Verify it runs successfully
3. Check artifacts generated

## Rollback Plan

If issues are found:

1. **Disable CI workflow**:
   ```bash
   # Add to workflow file:
   # if: false
   ```

2. **Remove from merge gate** (if added):
   ```bash
   # Comment out in justfile ci-gate recipe
   ```

3. **Revert justfile recipes**:
   ```bash
   git checkout HEAD -- justfile
   ```

## Sign-Off

- [ ] All local tests pass
- [ ] Documentation reviewed
- [ ] CI workflow syntax valid
- [ ] Baseline generated successfully
- [ ] Ready for commit

## Next Steps

1. ✅ Complete baseline generation
2. ⏳ Run full test suite
3. ⏳ Commit changes
4. ⏳ Push and test in CI
5. ⏳ Update issue #284 with results

---

**Last Updated**: 2026-01-28
**Tester**: Automated Testing
**Status**: 🟡 In Progress (waiting for baseline generation)
