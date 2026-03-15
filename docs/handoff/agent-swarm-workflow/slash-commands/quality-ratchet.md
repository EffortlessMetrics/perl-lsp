---
description: Run quality checks, compare against baseline, update if improved
argument-hint: "[--check | --update] [--metric <name>]"
---

# Quality Ratchet

Run quality metrics and ratchet the baseline forward. Mode: **$ARGUMENTS**

## Concept

A quality ratchet ensures that metrics can only improve. After agents make
improvements, you update the baseline. Future runs fail if metrics drop below
the new floor.

## Metrics to track

Adapt these to your project. Store baselines in `.ci/` or a similar directory.

### Test count
```bash
# Measure current test count
# Rust:
test_count=$(cargo test --workspace --lib -- --list 2>/dev/null | grep -c "test$" || echo 0)
# Python:
test_count=$(pytest --collect-only -q 2>/dev/null | tail -1 | grep -oP '\d+(?= test)')
# TypeScript:
test_count=$(npx jest --listTests 2>/dev/null | wc -l)
# Go:
test_count=$(go test ./... -list '.*' 2>/dev/null | grep -c '^Test')

echo "Current test count: $test_count"
```

### Lint warnings
```bash
# Should be zero; ratchet prevents regressions
# Rust:
warnings=$($LINT_CMD 2>&1 | grep -c "warning\[" || echo 0)
# Python:
warnings=$($LINT_CMD 2>&1 | wc -l)

echo "Current warning count: $warnings"
```

### Build / type-check cleanliness
```bash
$CHECK_CMD 2>&1
echo "Exit code: $?"
```

## Check against baseline

```bash
baseline_file=".ci/quality-baseline.json"
if [ -f "$baseline_file" ]; then
    echo "Comparing against baseline..."
    # Read baseline values and compare
    # Fail if any metric regressed
else
    echo "No baseline found. Run with --update to create one."
fi
```

## Update baseline after improvements

```bash
# 1. Run all metrics to see current state
# 2. If metrics improved, update baseline

cat > .ci/quality-baseline.json <<'BASELINEEOF'
{
  "test_count": <current>,
  "lint_warnings": <current>,
  "updated_at": "<date>"
}
BASELINEEOF

# 3. Verify the ratchet holds
# (re-run check to make sure baseline matches reality)

# 4. Commit
git add .ci/quality-baseline.json
git commit -m "ci: ratchet quality baseline (tests: <N>, warnings: <N>)"
```

## Integration with agent swarm

Typical flow:
1. `/wave bug-fixes` -- agents fix bugs
2. `/bulk-pr` -- create PRs
3. Merge PRs
4. `/quality-ratchet --update` -- lock in improvements
5. `/wave test-coverage` -- agents add tests
6. `/bulk-pr` -- create PRs
7. Merge PRs
8. `/quality-ratchet --update` -- lock in higher test count

The ratchet prevents future work from accidentally undoing improvements.
