# Documentation Truth System

## Problem

Manual documentation maintenance leads to inevitable drift:
- Test counts become stale ("1,384 tests" vs actual 1,649)
- Component tallies don't reconcile (668 ≠ 1,384)
- Pass rates lack clear denominators (99.6% of what?)
- Version numbers hardcoded across multiple files
- Performance claims ("5000x") lack receipts

**Root cause**: Hardcoded numbers = fragile truth that drifts by design.

## Solution: Self-Healing Documentation

Generate canonical receipts → Render docs from receipts → Guard with CI.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Source of Truth                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Cargo.toml   │  │ cargo test   │  │ cargo doc    │      │
│  │   version    │  │   output     │  │   warnings   │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
└─────────┼──────────────────┼──────────────────┼──────────────┘
          │                  │                  │
          v                  v                  v
    ┌─────────────────────────────────────────────────────┐
    │        scripts/generate-receipts.sh                  │
    │                                                       │
    │  1. Run cargo test --workspace --exclude xtask       │
    │  2. Parse "test result: ok. X passed; Y failed..."   │
    │  3. Run cargo doc and count warnings                 │
    │  4. Extract version from perl-parser/Cargo.toml      │
    │  5. Generate JSON receipts in artifacts/             │
    └─────────────────────┬───────────────────────────────┘
                          │
                          v
            ┌──────────────────────────────┐
            │  artifacts/state.json         │
            │  {                            │
            │    "version": "0.8.8",        │
            │    "tests": {                 │
            │      "passed": 828,           │
            │      "failed": 3,             │
            │      "ignored": 818,          │
            │      "pass_rate_active": 99.6 │
            │    },                         │
            │    "docs": {                  │
            │      "missing_docs": 484      │
            │    }                          │
            │  }                            │
            └────────────┬─────────────────┘
                         │
                         v
         ┌───────────────────────────────────────┐
         │  scripts/render-docs.sh                │
         │                                        │
         │  Replace tokens in templates:          │
         │    0.8.8          → 0.8.8        │
         │    0      → 828          │
         │    0.0 → 99.6         │
         │    484     → 484          │
         └────────────┬──────────────────────────┘
                      │
                      v
            ┌──────────────────────┐
            │  CLAUDE.md            │
            │  README.md            │
            │  (rendered docs)      │
            └──────────┬───────────┘
                       │
                       v
          ┌────────────────────────────────┐
          │  .github/workflows/docs-truth  │
          │                                 │
          │  CI guard ensures:              │
          │  ✓ Receipts regenerate cleanly  │
          │  ✓ Rendered docs match commits  │
          │  ✓ No manual number drift       │
          └─────────────────────────────────┘
```

## Usage

### Generate Receipts

```bash
# Full receipts (tests + docs + version)
./scripts/generate-receipts.sh

# Quick receipts (docs + version only, faster)
./scripts/quick-receipts.sh
```

This creates:
- `artifacts/test-output.txt` - Raw test output
- `artifacts/test-summary.json` - Parsed test metrics
- `artifacts/doc-summary.json` - Doc warning counts
- `artifacts/state.json` - Consolidated truth

### Render Documentation

```bash
./scripts/render-docs.sh
```

Replaces template tokens with receipt values:
- `0.8.8` → Version from perl-parser/Cargo.toml
- `0`, `0`, `0` → Test counts
- `0.0` → Pass rate for active tests (excluding ignored)
- `484` → Count of missing documentation warnings

### CI Enforcement

The `docs-truth` workflow runs on every PR touching documentation:
1. Regenerates receipts from scratch
2. Renders documentation from receipts
3. Fails if committed docs differ from rendered docs
4. Validates claimed counts match receipts

## Template Token Reference

| Token | Source | Example |
|-------|--------|---------|
| `0.8.8` | perl-parser/Cargo.toml | 0.8.8 |
| `0` | cargo test summary | 828 |
| `0` | cargo test summary | 3 |
| `0` | cargo test summary | 818 |
| `0` | passed + failed | 831 |
| `0` | active + ignored | 1649 |
| `0.0` | (passed / active) × 100 | 99.6 |
| `0.0` | (passed / total) × 100 | 50.2 |
| `484` | rustdoc warning count | 484 |

## Receipt Format

### test-summary.json

```json
{
  "passed": 828,
  "failed": 3,
  "ignored": 818,
  "active_tests": 831,
  "total_all_tests": 1649,
  "pass_rate_active": 99.6,
  "pass_rate_total": 50.2
}
```

### doc-summary.json

```json
{
  "missing_docs": 484
}
```

### state.json (consolidated)

```json
{
  "version": "0.8.8",
  "tests": {
    "passed": 828,
    "failed": 3,
    "ignored": 818,
    "active_tests": 831,
    "total_all_tests": 1649,
    "pass_rate_active": 99.6,
    "pass_rate_total": 50.2
  },
  "docs": {
    "missing_docs": 484
  },
  "generated_at": "2025-10-23T03:41:17Z"
}
```

## Migration Guide

### Converting Hardcoded Numbers to Tokens

Before:
```markdown
99.6% test pass rate (828 passing, 3 failing, 818 ignored)
```

After (template):
```markdown
0.0% test pass rate (0 passing, 0 failing, 0 ignored)
```

Rendered:
```markdown
99.6% test pass rate (828 passing, 3 failing, 818 ignored)
```

### When to Use Which Pass Rate

- **`pass_rate_active`**: Pass rate excluding ignored tests (preferred for quality metrics)
  - Formula: `(passed / (passed + failed)) × 100`
  - Example: `(828 / 831) × 100 = 99.6%`
  - Use when: Reporting test suite health

- **`pass_rate_total`**: Pass rate including ignored tests (use for coverage)
  - Formula: `(passed / total_all_tests) × 100`
  - Example: `(828 / 1649) × 100 = 50.2%`
  - Use when: Showing proportion of total test base

## Benefits

1. **No more manual number updates** - Numbers come from receipts, not memory
2. **Automatic reconciliation** - All tallies derived from same source
3. **CI-enforced truth** - Drift caught before merge
4. **Clear provenance** - Every number traceable to receipt
5. **Reproducible** - Anyone can regenerate receipts and verify

## Future Enhancements

1. **Per-crate breakdowns** - Track test counts by crate
2. **Benchmark receipts** - Store criterion results as JSON
3. **Historical tracking** - Track metrics over time in `docs/reports/`
4. **Snapshot updates** - Require explicit label to change baselines
5. **Template validation** - Catch unused tokens or missing receipts

## Troubleshooting

### Receipts not generated

```bash
# Check if xtask compilation is interfering
cargo test --workspace --exclude xtask --no-fail-fast

# Use quick receipts for docs-only
./scripts/quick-receipts.sh
```

### Numbers don't match

```bash
# Regenerate from scratch
rm -rf artifacts/
./scripts/generate-receipts.sh

# Compare with committed state
diff artifacts/state.json docs/reports/state-*.json
```

### CI fails on docs-truth

The docs were manually edited without updating receipts.

Fix:
```bash
./scripts/generate-receipts.sh
./scripts/render-docs.sh
git add artifacts/ CLAUDE.md docs/
git commit -m "sync: regenerate docs from receipts"
```

## Related

- `.github/workflows/docs-truth.yml` - CI enforcement
- `scripts/generate-receipts.sh` - Receipt generation
- `scripts/render-docs.sh` - Template rendering
- `artifacts/state.json` - Current truth
