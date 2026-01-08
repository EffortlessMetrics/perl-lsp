# DevLT Calibration Store

This directory contains calibration data for DevLT (Decision-weighted Developer Lieutenant Time) estimation.

## Files

| File | Purpose |
|------|---------|
| `devlt.csv` | Per-PR calibration records |

## Purpose

DevLT estimation improves over time through calibration against reported values. This store:

1. **Records estimates** - Every forensics pass appends an estimate
2. **Collects reported values** - Maintainers optionally record actual DevLT
3. **Enables weight tuning** - When systematic bias is detected, adjust weights
4. **Documents error bands** - Track estimation accuracy over time

## Usage

### Adding a New Entry

After running a forensics pass, append to `devlt.csv`:

```bash
echo "275,2026-01-08,60,90,,github_plus_agent_logs,devlt_est_v1:decision_weighted,Forensics tooling" >> devlt.csv
```

### Recording Reported DevLT

When a maintainer reports actual DevLT, update the `reported_min` column:

```csv
# Before
275,2026-01-08,60,90,,github_plus_agent_logs,...

# After (maintainer reported ~70 minutes)
275,2026-01-08,60,90,70,github_plus_agent_logs,...
```

### Calibration Analysis

To check calibration accuracy:

```bash
# Count PRs with reported values
grep -v '^#' devlt.csv | awk -F, 'NR>1 && $5!="" {count++} END {print count " PRs with reported values"}'

# Find estimates outside range
grep -v '^#' devlt.csv | awk -F, 'NR>1 && $5!="" && ($5<$3 || $5>$4) {print "Out of range: PR " $1}'
```

## Calibration Protocol

1. **Minimum sample**: 5 PRs with reported values before adjusting weights
2. **Error threshold**: Adjust weights if >50% of estimates miss range by >20%
3. **Weight adjustment**: Increase/decrease event weights by 20% per iteration
4. **Version tracking**: Update `method_id` when weights change (e.g., `devlt_est_v2:decision_weighted`)

## Current Status

| Metric | Value |
|--------|-------|
| PRs with estimates | 5 |
| PRs with reported values | 0 |
| Current method | devlt_est_v1:decision_weighted |
| Error band | ±30-50% (uncalibrated) |
| Target error band | ±20% (after calibration) |

## See Also

- [`../../../DEVLT_ESTIMATION.md`](../../DEVLT_ESTIMATION.md) - Estimation rubric and weights
- [`../../../FORENSICS_SCHEMA.md`](../../FORENSICS_SCHEMA.md) - Dossier format
