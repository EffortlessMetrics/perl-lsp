# CI Audit Documentation

> **Purpose**: This document describes the CI audit infrastructure that validates parser quality, feature coverage, and test health for the perl-lsp project.

---

## Overview

The CI audit system provides automated validation of:

1. **Corpus Audit**: NodeKind coverage, GA feature alignment, and timeout risk detection
2. **Parse Error Baseline**: Ratchet mechanism ensuring parse error counts only decrease
3. **Ignored Test Tracking**: Categorized tracking of test debt with baseline enforcement
4. **Feature Catalog Audit**: Single source of truth validation via `features.toml`

All audit gates run locally via `just ci-gate` before CI. The repo is local-first by design.

---

## Corpus Audit

### What It Checks

The corpus audit analyzes test corpus files to ensure comprehensive parser coverage:

- **NodeKind Coverage**: Tracks which AST node types are exercised by corpus files
- **GA Feature Alignment**: Validates that GA (Generally Available) LSP features have corresponding test coverage
- **Parse Outcomes**: Categorizes files by parse result (ok, error, timeout, panic)
- **Timeout Risks**: Detects constructs that may cause parsing performance issues

### Running the Audit

```bash
# Via canonical gate (recommended)
just ci-gate  # Includes corpus audit

# Standalone corpus audit
cargo run -p xtask --no-default-features -q -- corpus-audit --fresh

# Specify custom corpus path
cargo run -p xtask --no-default-features -q -- corpus-audit \
  --corpus-path ./test_corpus \
  --output corpus_audit_report.json

# Check mode (CI validation)
cargo run -p xtask --no-default-features -q -- corpus-audit --check
```

### Report Output

The audit generates `corpus_audit_report.json` with the following structure:

```json
{
  "metadata": {
    "generated_at": "2026-01-11T18:16:01Z",
    "version": "0.1.0",
    "duration_secs": 0
  },
  "inventory": {
    "total_files": 10,
    "files_by_layer": [{"layer": "TestCorpus", "count": 10}],
    "total_size_bytes": 26542,
    "total_line_count": 1204
  },
  "parse_outcomes": {
    "total": 10,
    "ok": 5,
    "error": 5,
    "timeout": 0,
    "panic": 0,
    "error_by_category": {
      "ModernFeature": 4,
      "Subroutine": 1
    },
    "failing_files": [...]
  },
  "nodekind_coverage": {
    "total_count": 87,
    "covered_count": 3,
    "coverage_percentage": 3.45,
    "never_seen": ["Prototype", "Die", "Subroutine", ...],
    "at_risk": [],
    "frequency": {"Statement": 5, "Identifier": 5}
  },
  "ga_coverage": {
    "total_count": 12,
    "covered_count": 12,
    "coverage_percentage": 100.0,
    "features": [...],
    "uncovered_critical": [],
    "uncovered_partial": []
  },
  "timeout_risks": []
}
```

### Interpreting Results

**Parse Outcomes**:
- `ok`: File parsed successfully without errors
- `error`: Parse failed with categorized error (e.g., ModernFeature, Subroutine)
- `timeout`: Parse exceeded time limit (30s default)
- `panic`: Parser panicked during execution

**NodeKind Coverage**:
- **total_count**: Total number of AST node types defined
- **covered_count**: Number of node types seen in corpus
- **never_seen**: Node types not exercised (potential coverage gaps)
- **at_risk**: Node types with low frequency (fragile coverage)

**GA Coverage**:
- **features**: List of GA (Generally Available) LSP features
- **covering_files**: Which corpus files exercise each feature
- **uncovered_critical**: P0/P1 features missing coverage (gate failure)

---

## Parse Error Baseline

### Location

The parse error baseline is tracked at:
```
ci/parse_errors_baseline.txt
```

This file contains a single integer representing the maximum allowed parse error count.

### Ratchet Behavior

The parse error count can **only decrease, never increase**:

- ✅ **Allowed**: Reducing parse errors (parser improvements)
- ❌ **Blocked**: Increasing parse errors (regressions)
- ℹ️ **Neutral**: Maintaining current error count

### Updating the Baseline

When you successfully reduce parse errors:

```bash
# After fixing parser issues, update baseline
echo 3 > ci/parse_errors_baseline.txt  # Replace 3 with new count

# Verify improvement
bash ci/check_parse_errors.sh
```

### How It Works

The `ci/check_parse_errors.sh` script:
1. Runs corpus audit to generate fresh report
2. Extracts current parse error count from `corpus_audit_report.json`
3. Compares against baseline in `ci/parse_errors_baseline.txt`
4. Fails CI if error count increased
5. Suggests updating baseline if error count decreased

**Example output**:
```
Running corpus audit...

Parse errors in test corpus: 3
Baseline: 5

IMPROVEMENT: 2 fewer parse errors!
Consider updating baseline: echo 3 > ci/parse_errors_baseline.txt

Check passed: parse error count is within acceptable range
```

---

## Ignored Test Tracking

### Script Location

```bash
scripts/ignored-test-count.sh
```

### Categories

Ignored tests are categorized by reason:

| Category | Description | Examples |
|----------|-------------|----------|
| **bug** | Known bugs waiting to be fixed | Parser limitations, incorrect behavior |
| **feature** | Feature-gated/not implemented | WIP features, pending implementations |
| **manual** | Manual helper tests | Snapshot regeneration, helper utilities |
| **infra** | Infrastructure/setup requirements | Environment dependencies, configuration |
| **stress** | Stress tests (run with `--ignored`) | Memory stress, stack overflow tests |
| **protocol** | Protocol compliance issues | LSP/DAP specification edge cases |
| **brokenpipe** | Transport/flake issues | BrokenPipe errors, transport flakes |
| **bare** | No reason given | Should be categorized or removed |
| **other** | Uncategorized | Catch-all for unrecognized patterns |

### Running the Script

```bash
# Show current counts with delta from baseline
./scripts/ignored-test-count.sh

# Update baseline (after justifiable changes)
./scripts/ignored-test-count.sh --update

# CI gate mode (exit 1 if total increased)
./scripts/ignored-test-count.sh --check

# Verbose output with detailed breakdown
VERBOSE=1 ./scripts/ignored-test-count.sh
```

### Baseline Management

The baseline is stored in:
```
scripts/.ignored-baseline
```

**Format**:
```
# Ignored test baseline - 2026-01-11T18:00:00Z
# Updated by: ignored-test-count.sh --update
brokenpipe=0
feature=1
infra=0
protocol=0
manual=1
stress=0
bug=8
bare=0
other=0
total=10
```

**CI Enforcement**:
- Total ignored test count can only decrease or stay the same
- Increases require justification and explicit baseline update
- Run `--check` in CI to enforce this policy

---

## Feature Catalog Audit

### Single Source of Truth

The `features.toml` file at the repository root defines all LSP features:

```bash
features.toml  # Canonical LSP feature definitions
```

### Maturity Levels

Features are classified by maturity:

| Level | Description | Advertised | CI Required |
|-------|-------------|------------|-------------|
| **planned** | Future work, not implemented | No | No |
| **experimental** | In development, unstable | No | No |
| **preview** | Testing phase, may change | Limited | Optional |
| **ga** | Generally Available, stable | Yes | Yes |

### How Coverage Is Calculated

LSP coverage percentage is computed as:

```
LSP Coverage = (advertised_ga / trackable) × 100%
```

Where:
- **advertised_ga**: Features with `maturity = "ga"` and `advertised = true`
- **trackable**: All features except those marked `maturity = "planned"`

**Example from `features.toml`**:
```toml
[[feature]]
id = "lsp.completion"
spec = "LSP 3.0"
area = "text_document"
maturity = "ga"              # GA maturity level
advertised = true            # Counts toward coverage
tests = ["tests/lsp_completion_tests.rs"]
description = "Code completion with 150+ built-in functions"
```

### Auditing Features

```bash
# View current LSP coverage metrics
just status-check

# Update computed metrics in CURRENT_STATUS.md
just status-update

# Verify features.toml integrity
cargo run -p xtask -- validate-features
```

### Adding New Features

When implementing a new LSP feature:

1. Add entry to `features.toml`:
   ```toml
   [[feature]]
   id = "lsp.new_feature"
   spec = "LSP 3.18"
   area = "text_document"
   maturity = "experimental"  # Start as experimental
   advertised = false
   tests = ["tests/lsp_new_feature_tests.rs"]
   description = "New feature description"
   ```

2. Implement feature with test coverage

3. Promote to GA when stable:
   ```toml
   maturity = "ga"
   advertised = true
   ```

4. Update metrics:
   ```bash
   just status-update
   ```

---

## Integration with CI Gate

All audit checks run as part of the canonical gate:

```bash
# Run all gates (required before push)
just ci-gate
```

**Gate stages**:
1. Format check (`cargo fmt --all -- --check`)
2. Clippy lints (`cargo clippy --workspace -- -D warnings`)
3. **Corpus audit** (via `ci/check_parse_errors.sh`)
4. **Parser matrix check** (validates NodeKind coverage)
5. **Ignored test gate** (via `ci/check_ignored.sh`)
6. Library tests (`cargo test --workspace --lib`)
7. Integration tests (`cargo test -p perl-lsp`)

**Pre-push hook**:
```bash
# Install hook to run gate before push
bash scripts/install-githooks.sh
```

---

## Troubleshooting

### Parse Error Regression

**Symptom**: `ci/check_parse_errors.sh` fails with increased error count

**Solutions**:
1. Run corpus audit to see failing files:
   ```bash
   cargo run -p xtask -- corpus-audit --fresh
   cat corpus_audit_report.json | jq '.parse_outcomes.failing_files'
   ```
2. Fix parser to handle new constructs
3. If intentional regression, update baseline with justification

### Ignored Test Count Increased

**Symptom**: `ci/check_ignored.sh` fails with increased total

**Solutions**:
1. Review new ignores with verbose output:
   ```bash
   VERBOSE=1 ./scripts/ignored-test-count.sh
   ```
2. Categorize ignores properly with reason comments
3. Fix underlying issues instead of ignoring tests
4. If justified, update baseline:
   ```bash
   ./scripts/ignored-test-count.sh --update
   ```

### Missing GA Feature Coverage

**Symptom**: Corpus audit reports uncovered critical features

**Solutions**:
1. Check GA coverage details:
   ```bash
   cat corpus_audit_report.json | jq '.ga_coverage.uncovered_critical'
   ```
2. Add test corpus files exercising the feature
3. Re-run audit to verify coverage:
   ```bash
   cargo run -p xtask -- corpus-audit --fresh
   ```

### NodeKind Coverage Gaps

**Symptom**: Low `covered_count` in nodekind_coverage

**Solutions**:
1. Review never-seen node kinds:
   ```bash
   cat corpus_audit_report.json | jq '.nodekind_coverage.never_seen'
   ```
2. Add corpus files with missing constructs
3. Consider if some node kinds are deprecated/unused

---

## Best Practices

1. **Run `just ci-gate` before every commit** to catch issues early
2. **Update baselines only with justification** and document in commit messages
3. **Categorize ignored tests properly** using explicit reason comments
4. **Maintain `features.toml` accuracy** as single source of truth
5. **Review corpus audit reports** when adding new Perl language features
6. **Use local-first workflow** - don't rely on CI for validation

---

## Related Documentation

- [CURRENT_STATUS.md](CURRENT_STATUS.md) - Computed metrics and project health
- [COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md) - Full command catalog
- [features.toml](../features.toml) - Canonical LSP feature definitions
- [ROADMAP.md](ROADMAP.md) - Milestones and release planning

---

*Last Updated: 2026-01-11*
*Canonical source: This document is authoritative for CI audit procedures*
