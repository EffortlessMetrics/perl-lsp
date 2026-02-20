# Technical Debt Tracking

This document describes the technical debt tracking system used to maintain useful CI gates while acknowledging imperfection.

## Overview

The debt ledger system provides:

1. **Quarantined Tests** - Flaky tests excluded from merge-gate but still run for visibility
2. **Known Issues** - Acknowledged problems that don't fail gates but are tracked
3. **Technical Debt** - Architectural and code quality items for future cleanup
4. **Budgets** - Limits on each category to prevent debt accumulation
5. **Expiration** - Quarantines have shelf lives and must be resolved or renewed

## Quick Reference

```bash
# Show current debt status
just debt-report

# CI gate check (fails if over budget)
just debt-check

# Show expired quarantines only
just debt-expired

# JSON output for tooling
just debt-json

# PR comment summary
just debt-pr-summary
```

## Debt Ledger

The debt ledger lives at `.ci/debt-ledger.yaml` and is the single source of truth for tracked debt.

### Structure

```yaml
# Budget limits
budgets:
  max_quarantined_tests: 10
  max_known_issues: 20
  max_technical_debt: 30
  warning_threshold_percent: 80
  critical_threshold_percent: 95

# Flaky tests in quarantine
flaky_tests:
  - name: "lsp::test_completion_timeout"
    added: "2026-01-24"
    issue: "#198"
    tier: "quarantine"
    quarantine_days: 14
    expires: "2026-02-07"
    notes: "Timing-dependent, needs server init fix"

# Acknowledged issues
known_issues:
  - name: "clippy::cognitive_complexity on parser.rs"
    added: "2026-01-20"
    issue: "#205"
    status: "accepted"
    notes: "Complex but correct, refactor post-v0.9.x (Production-Ready)"

# Cleanup work
technical_debt:
  - area: "error_handling"
    description: "Some unwrap() in test code"
    priority: "low"
    target: "v1.1"
```

## Quarantine Mechanism

### What Quarantine Does

- Test is still **run** as part of the gate
- Test results are **reported** in logs and receipts
- Test failures **do not block** the merge-gate
- Clear expiration date forces resolution

### When to Quarantine

Use quarantine for tests that:

1. Fail intermittently (flaky) due to timing, resources, or environment
2. Have a known root cause that can't be fixed immediately
3. Are blocking development while investigation continues

Do NOT quarantine tests that:

1. Fail consistently (fix the test or the code)
2. Fail because of a real bug (fix the bug)
3. Were never working (delete or fix them)

### How to Quarantine a Test

1. **Create an issue** tracking the flaky test

2. **Add entry to debt ledger**:
   ```yaml
   flaky_tests:
     - name: "lsp::test_completion_timeout"
       added: "2026-01-24"
       issue: "#198"
       tier: "quarantine"
       quarantine_days: 14
       expires: "2026-02-07"
       owner: "your-username"
       notes: "Describe the failure pattern"
       failure_pattern: "timeout waiting for completion"
       affected_platforms:
         - "windows"
         - "wsl"
   ```

3. **Verify with debt report**:
   ```bash
   just debt-report
   ```

### How to Un-Quarantine

When the root cause is fixed:

1. **Remove from flaky_tests section**

2. **Add to history.resolved**:
   ```yaml
   history:
     resolved:
       - type: "flaky_test"
         name: "lsp::test_completion_timeout"
         resolved: "2026-02-05"
         resolution: "Fixed server initialization race"
         pr: "#210"
         days_in_quarantine: 12
   ```

3. **Close the tracking issue**

### Quarantine Expiration

Quarantines have a shelf life (`quarantine_days`). When a quarantine expires:

1. The debt check (`just debt-check`) will **fail**
2. An alert is generated
3. The test must be either:
   - **Fixed** and un-quarantined
   - **Renewed** with a new expiration date and justification
   - **Disabled** (rare, requires strong justification)

To renew a quarantine, update the `expires` date and add a note explaining why more time is needed.

## Budget System

### Budget Categories

| Category | Default Budget | Purpose |
|----------|----------------|---------|
| Quarantined Tests | 10 | Flaky tests excluded from gate |
| Known Issues | 20 | Acknowledged but not fixed |
| Technical Debt | 30 | Cleanup and improvement work |

### Budget Thresholds

- **Warning (80%)**: Approaching limit, consider cleanup
- **Critical (95%)**: Near limit, prioritize debt reduction

### What Happens When Over Budget

- `just debt-check` fails with exit code 1
- PR comments show budget status
- Receipts include debt status

To reduce debt:

1. Fix flaky tests and remove from quarantine
2. Resolve known issues
3. Address technical debt items
4. Review if items are still relevant

## Known Issues Tracking

For issues that are acknowledged but not immediately fixed:

```yaml
known_issues:
  - name: "Slow first parse on large files"
    added: "2026-01-15"
    issue: "#189"
    status: "deferred"
    category: "performance"
    notes: "Acceptable for current use cases"
    target_version: "v1.2"
```

### Status Values

| Status | Meaning |
|--------|---------|
| accepted | Intentional, won't fix before target version |
| deferred | Will fix, but not blocking current work |
| monitoring | Watching for impact, may escalate |
| wontfix | Not a bug or out of scope (rare) |

## Technical Debt Tracking

For architectural debt and cleanup work:

```yaml
technical_debt:
  - area: "error_handling"
    description: "Some unwrap() in test code"
    priority: "low"
    target: "v1.1"
    issue: "#143"
    notes: "Test code uses #[allow], should migrate"
```

### Priority Values

| Priority | Meaning |
|----------|---------|
| critical | Must fix before next major release |
| high | Should fix in next few releases |
| medium | Nice to have, opportunistic |
| low | Cosmetic or minor improvement |

### Categories

- `architecture` - Design decisions limiting future work
- `error_handling` - Unwrap/expect, poor error messages
- `testing` - Missing tests, flaky infrastructure
- `performance` - Known slow paths
- `documentation` - Missing or outdated docs
- `security` - Non-critical security improvements
- `dependencies` - Outdated or problematic deps

## Integration with Gates

### Receipt Integration

Gate receipts include debt status:

```json
{
  "summary": {
    "debt_status": {
      "overall_status": "ok",
      "quarantined_tests": {
        "count": 2,
        "budget": 10,
        "percent": 20.0,
        "expired": 0
      }
    }
  }
}
```

### Gate Policy Integration

The gate policy (`.ci/gate-policy.yaml`) references the debt ledger:

```yaml
flake_policy:
  quarantined_gates: []
  # Populated from debt-ledger.yaml flaky_tests
```

### PR Comments

When configured, PRs show debt impact:

```markdown
## Technical Debt Status

| Category | Count | Budget | Status |
|----------|-------|--------|--------|
| Quarantined Tests | 2 | 10 | ok |
| Known Issues | 5 | 20 | ok |
| Technical Debt | 8 | 30 | ok |
```

## Weekly Trend Tracking

The debt ledger includes weekly summaries for trend analysis:

```yaml
history:
  weekly_summaries:
    - week: "2026-W04"
      quarantined_tests: 2
      known_issues: 5
      technical_debt: 8
      added: 1
      resolved: 3
      notes: "Resolved DAP race condition issues"
```

This enables tracking debt trends over time and identifying patterns.

## Best Practices

1. **Create issues first** - Every quarantined item needs a tracking issue
2. **Set realistic expiration** - 7-14 days for most flaky tests
3. **Document failure patterns** - Help others recognize the flakiness
4. **Review regularly** - Check debt report weekly
5. **Celebrate resolution** - Acknowledge when debt is paid down
6. **Don't abuse quarantine** - It's not a way to hide broken tests

## Troubleshooting

### "PyYAML not installed"

Install PyYAML for full functionality:

```bash
pip install pyyaml
```

The script includes a fallback parser but full YAML support is recommended.

### Quarantine not taking effect

1. Verify the test name matches exactly
2. Check that `tier: "quarantine"` is set
3. Run `just debt-report` to verify the entry is loaded

### Budget check failing unexpectedly

1. Run `just debt-report` to see current status
2. Check for expired quarantines
3. Review if budget limits need adjustment (rare)

## Related Documentation

- [Gate Policy](../.ci/gate-policy.yaml) - CI gate configuration
- [Receipt Schema](../.ci/receipt.schema.json) - Receipt JSON structure
- [CLAUDE.md](../CLAUDE.md) - Development workflow
