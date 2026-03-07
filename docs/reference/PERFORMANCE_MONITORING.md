# Performance Monitoring and Regression Alerts

This document describes the automated performance monitoring and regression detection system for perl-lsp.

## Overview

The perl-lsp project uses an automated performance monitoring system to detect performance regressions before they reach production. The system:

- Runs benchmarks on every PR (when labeled with `ci:bench`)
- Compares results against committed baselines
- Generates automated alerts for performance regressions
- Posts PR comments with actionable performance feedback
- Optionally gates merge on critical regressions

## Quick Start

### Run Benchmarks Locally

```bash
# Run all benchmarks
just bench

# Quick smoke test (fast, ~30s)
just bench-quick

# Compare against baseline
just bench-compare

# Generate performance alerts
just bench-alert

# Check for critical regressions (exits non-zero)
just bench-alert-check
```

### CI Integration

1. Add the `ci:bench` label to your PR
2. Benchmarks will run automatically
3. Alerts will be posted as PR comments if regressions are detected

## Architecture

### Components

```
┌─────────────────────┐
│ Benchmark Execution │
│   (Criterion.rs)    │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Results Collection  │
│  (JSON output)      │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Baseline Comparison │
│   (compare.py)      │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Alert Generation    │
│    (alert.py)       │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│  PR Comment / Gate  │
│  (GitHub Actions)   │
└─────────────────────┘
```

### Key Files

| File | Purpose |
|------|---------|
| `.ci/benchmark-thresholds.yaml` | Configuration for alert thresholds |
| `benchmarks/baselines/v*.json` | Committed baseline results |
| `benchmarks/results/latest.json` | Current benchmark results |
| `benchmarks/scripts/run-benchmarks.sh` | Benchmark runner |
| `benchmarks/scripts/compare.py` | Baseline comparison |
| `benchmarks/scripts/alert.py` | Alert generation |
| `.github/workflows/benchmark.yml` | CI workflow |

## Configuration

### Threshold Configuration

Edit `.ci/benchmark-thresholds.yaml` to configure alert thresholds:

```yaml
# Global defaults
defaults:
  warn_threshold_pct: 10        # Yellow warning
  regression_threshold_pct: 20  # Red regression
  critical_threshold_pct: 50    # CI gate (optional)
  improvement_threshold_pct: 10 # Green improvement

# Category-specific overrides
categories:
  lsp:
    warn_threshold_pct: 5       # Stricter for LSP
    regression_threshold_pct: 15
    critical_threshold_pct: 30

# Critical path benchmarks (highest priority)
critical_path:
  - name: "lsp/document_insertions"
    warn_threshold_pct: 5
    regression_threshold_pct: 10
    critical_threshold_pct: 20
    target_ms: 1.0
```

### Alert Levels

| Level | Threshold | Action | Example |
|-------|-----------|--------|---------|
| **OK** | < warn_threshold | None | +5% change with 10% threshold |
| **WARNING** | warn_threshold to regression_threshold | PR comment | +12% change with 10% warn, 20% regression |
| **REGRESSION** | regression_threshold to critical_threshold | PR comment, highlight | +25% change with 20% regression, 50% critical |
| **CRITICAL** | > critical_threshold | PR comment, optional gate | +60% change with 50% critical |
| **IMPROVED** | < -improvement_threshold | PR comment (positive) | -15% change with 10% improvement |

### Alerting Behavior

Configure alerting behavior in `.ci/benchmark-thresholds.yaml`:

```yaml
alerting:
  comment_on_warning: true      # Post PR comment on warnings
  comment_on_regression: true   # Post PR comment on regressions
  fail_on_critical: false       # Gate merge on critical regressions
  include_trends: false         # Include historical trends (future)
  channels:
    github_comment: true        # GitHub PR comments
    slack_webhook: false        # Slack notifications (future)
    email: false                # Email alerts (future)
```

## Usage

### Running Benchmarks

#### Local Development

```bash
# Full benchmark suite (comprehensive)
just bench

# Quick smoke test (fast validation)
just bench-quick

# Specific benchmark categories
just bench-parser   # Parser benchmarks only
just bench-lexer    # Lexer benchmarks only
just bench-lsp      # LSP benchmarks only
just bench-index    # Workspace index benchmarks only
```

#### CI/CD

Benchmarks run automatically when:

1. **PR labeled with `ci:bench`**: Runs on PR commits
2. **Nightly schedule**: Runs at 3am UTC daily
3. **Manual trigger**: Via GitHub Actions UI

### Comparing Results

```bash
# Compare against committed baseline
just bench-compare

# Compare specific files
./benchmarks/scripts/compare.py benchmarks/baselines/v0.9.0.json benchmarks/results/latest.json

# Fail on regression (for CI)
./benchmarks/scripts/compare.py --fail-on-regression

# Custom threshold
./benchmarks/scripts/compare.py --threshold 30
```

### Generating Alerts

```bash
# Terminal output (colored)
just bench-alert

# Markdown output (for PR comments)
just bench-alert-md > alert.md

# Check for critical regressions (exits non-zero)
just bench-alert-check
```

### Managing Baselines

#### Creating Baselines

```bash
# Create baseline for current results
just bench-baseline

# Create baseline with specific version
just bench-baseline v0.10.0

# Manual baseline creation
cp benchmarks/results/latest.json benchmarks/baselines/v0.10.0.json
```

#### Baseline Strategy

- **Release baselines**: Create on every release (v0.9.0, v0.10.0, etc.)
- **Feature baselines**: Optional baselines for major feature branches
- **Retention**: Keep 10 most recent baselines (configured in thresholds.yaml)
- **Location**: `benchmarks/baselines/v*.json` (committed to repo)

## Benchmark Categories

### 1. Parser Performance

Measures Perl code parsing speed across different file sizes and complexity levels.

| Benchmark | Description | Target | Alert Threshold |
|-----------|-------------|--------|-----------------|
| `parse_simple_script` | Basic variable declarations | < 50µs | 20% |
| `parse_complex_script` | Full module with OO code | < 500µs | 20% |
| `large_file_parsing` | 5000-line generated file | < 50ms | 20% |

### 2. Lexer Performance

Measures tokenization speed for various Perl input patterns.

| Benchmark | Description | Target | Alert Threshold |
|-----------|-------------|--------|-----------------|
| `simple_tokens` | Basic tokenization | < 10µs | 20% |
| `slash_disambiguation` | Context-aware `/` handling | < 50µs | 20% |
| `large_file` | 1000-line file tokenization | < 10ms | 20% |

### 3. LSP Response Times

**Critical path benchmarks** - affects editor responsiveness.

| Benchmark | Description | Target | Alert Threshold |
|-----------|-------------|--------|-----------------|
| `document_insertions` | Text edit operations | < 1ms | 10% (critical) |
| `position_conversions` | LSP position mapping | < 100µs | 10% (critical) |
| `incremental_edits` | Multiple small edits | < 5ms | 15% (critical) |

### 4. Workspace Indexing

Measures indexing performance (affects startup time and find-references).

| Benchmark | Description | Target | Alert Threshold |
|-----------|-------------|--------|-----------------|
| `initial_index_small` | 5 file workspace | < 100ms | 25% |
| `initial_index_medium` | 10 file workspace | < 200ms | 25% |
| `incremental_update` | Single file change | < 10ms | 15% |
| `symbol_lookup` | Definition lookup | < 1µs | 20% |

## Alert Examples

### Example 1: Warning Alert

```markdown
## Performance Benchmark Results

**Baseline:** v0.9.0 (abc123)
**Current:**  def456

### ⚡ Performance Warnings

| Benchmark | Baseline | Current | Change |
|-----------|----------|---------|--------|
| `parser/parse_complex_script` | 412µs | 458µs | +11.2% |

---

📊 **Summary:** 0 critical, 0 regressions, 1 warnings, 0 improvements
```

### Example 2: Regression Alert

```markdown
## Performance Benchmark Results

**Baseline:** v0.9.0 (abc123)
**Current:**  def456

### ⚠️ Performance Regressions

| Benchmark | Baseline | Current | Change | Status |
|-----------|----------|---------|--------|--------|
| `parser/large_file_parsing` | 42.3ms | 58.2ms | +37.6% | ⚠️ REGRESSION |

---

📊 **Summary:** 0 critical, 1 regressions, 0 warnings, 0 improvements
```

### Example 3: Critical Alert

```markdown
## Performance Benchmark Results

**Baseline:** v0.9.0 (abc123)
**Current:**  def456

### 🔴 Critical Regressions

| Benchmark | Baseline | Current | Change | Status |
|-----------|----------|---------|--------|--------|
| `lsp/document_insertions` | 0.8ms | 1.5ms | +87.5% | 🔴 CRITICAL |

### ⚠️ Performance Regressions

| Benchmark | Baseline | Current | Change | Status |
|-----------|----------|---------|--------|--------|
| `parser/parse_complex_script` | 412µs | 512µs | +24.3% | ⚠️ REGRESSION |

---

📊 **Summary:** 1 critical, 1 regressions, 0 warnings, 0 improvements

**⚠️ CRITICAL**: This PR introduces critical performance regressions that may impact user experience.
```

### Example 4: Improvement Alert

```markdown
## Performance Benchmark Results

**Baseline:** v0.9.0 (abc123)
**Current:**  def456

### ✅ Performance Improvements

| Benchmark | Baseline | Current | Change |
|-----------|----------|---------|--------|
| `parser/parse_simple_script` | 45.2µs | 38.1µs | -15.7% |
| `lsp/position_conversions` | 85µs | 72µs | -15.3% |

---

📊 **Summary:** 0 critical, 0 regressions, 0 warnings, 2 improvements

**✅ PASS**: Great job! This PR improves performance.
```

## Troubleshooting

### Alert Not Generated

**Symptom**: No PR comment posted even though benchmarks ran.

**Solutions**:
1. Check if `ci:bench` label is present
2. Verify baseline file exists: `benchmarks/baselines/v0.9.0.json`
3. Check workflow logs for `alert.py` errors
4. Ensure threshold changes don't exceed default thresholds

### False Positive Alerts

**Symptom**: Alerts for minor/insignificant changes.

**Solutions**:
1. Increase thresholds in `.ci/benchmark-thresholds.yaml`
2. Run benchmarks multiple times to verify consistency
3. Check for system load during benchmark runs
4. Add noisy benchmarks to exemptions list

### Noisy Benchmark Results

**Symptom**: High variance between runs (> 10% stddev).

**Solutions**:
1. Close other applications during benchmarking
2. Run with longer measurement time: `--measurement-time 10`
3. Increase warm-up iterations: `--warm-up-time 3`
4. Use dedicated benchmark machine (CI runners recommended)

### Missing Baseline

**Symptom**: "No baseline file found" error.

**Solutions**:
```bash
# Create baseline from current results
just bench
just bench-baseline v0.9.0

# Or manually copy
cp benchmarks/results/latest.json benchmarks/baselines/v0.9.0.json
```

### Critical Gate Blocking Merge

**Symptom**: PR cannot merge due to critical regression.

**Solutions**:
1. **Fix the regression**: Profile and optimize the slow code
2. **Increase threshold**: If the regression is acceptable, update `.ci/benchmark-thresholds.yaml`
3. **Disable gate**: Set `fail_on_critical: false` in thresholds.yaml (not recommended)
4. **Update baseline**: If the new performance is the new normal, create a new baseline

## Best Practices

### For Contributors

1. **Run benchmarks before submitting PR**:
   ```bash
   just bench
   just bench-compare
   ```

2. **Add `ci:bench` label for performance-sensitive changes**:
   - Parser modifications
   - LSP operation changes
   - Indexing algorithm updates

3. **Investigate warnings**:
   - Even warnings should be investigated
   - 10% slower is noticeable to users

4. **Document performance trade-offs**:
   - If performance degrades for good reasons (e.g., correctness), explain in PR

### For Maintainers

1. **Update baselines on releases**:
   ```bash
   just bench
   just bench-baseline v0.10.0
   git add benchmarks/baselines/v0.10.0.json
   git commit -m "chore: add v0.10.0 benchmark baseline"
   ```

2. **Review threshold configuration periodically**:
   - Adjust based on actual impact
   - Make critical path stricter
   - Relax non-critical benchmarks

3. **Monitor trend over time**:
   - Check nightly benchmark results
   - Look for gradual performance degradation
   - Investigate systematic slowdowns

4. **Gate critical regressions**:
   - Consider enabling `fail_on_critical: true`
   - Only for LSP critical path operations
   - Document escape hatch for exceptional cases

## Future Enhancements

### Planned Features

1. **Historical Trend Tracking**:
   - Store benchmark results over time
   - Generate trend charts for PR comments
   - Detect gradual performance degradation

2. **Performance SLO Enforcement**:
   - Define SLOs per benchmark (e.g., < 1ms for LSP ops)
   - Gate merge on SLO violations
   - Dashboard for SLO compliance

3. **Memory Regression Detection**:
   - Track memory usage alongside timing
   - Alert on memory bloat (> 10% increase)
   - Prevent memory leaks

4. **Comparison Against Main Branch**:
   - Compare PR against latest main, not just release baseline
   - Catch regressions introduced by other merged PRs
   - More accurate PR-specific alerts

5. **Multi-Platform Benchmarks**:
   - Run benchmarks on Linux, macOS, Windows
   - Detect platform-specific regressions
   - Ensure consistent performance across platforms

6. **Advanced Notifications**:
   - Slack webhook integration
   - Email alerts for critical regressions
   - GitHub Actions summary with trends

7. **Automated Bisection**:
   - Automatic bisection for large regressions
   - Identify exact commit causing slowdown
   - Reduce manual investigation time

## Integration with Development Workflow

### Pre-commit

```bash
# Optional pre-commit hook
#!/bin/bash
if git diff --cached --name-only | grep -E '(parser|lsp)/'; then
    echo "Parser/LSP changes detected, running benchmarks..."
    just bench-quick
    just bench-alert-check || {
        echo "Warning: Performance regression detected"
        read -p "Continue anyway? (y/n) " -n 1 -r
        echo
        [[ $REPLY =~ ^[Yy]$ ]]
    }
fi
```

### Pre-push

```bash
# Pre-push hook (recommended)
#!/bin/bash
just bench
just bench-alert-check
```

### CI Pipeline

```yaml
# .github/workflows/benchmark.yml
- name: Run benchmarks
  run: just bench

- name: Compare against baseline
  run: just bench-compare

- name: Generate alerts
  run: just bench-alert-md > alert.md

- name: Comment on PR
  uses: actions/github-script@v7
  with:
    script: |
      const fs = require('fs');
      const body = fs.readFileSync('alert.md', 'utf8');
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: body
      });
```

## See Also

- [benchmarks/README.md](../benchmarks/README.md) - Benchmark infrastructure overview
- [benchmarks/BENCHMARK_FRAMEWORK.md](../benchmarks/BENCHMARK_FRAMEWORK.md) - Detailed benchmark documentation
- [docs/ROADMAP.md](ROADMAP.md) - Performance milestones
- [.ci/benchmark-thresholds.yaml](../.ci/benchmark-thresholds.yaml) - Threshold configuration
- [Issue #278](https://github.com/user/perl-lsp/issues/278) - Original feature request
