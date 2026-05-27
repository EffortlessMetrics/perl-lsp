# Coverage gate notes

The parser coverage lane already runs branch-aware coverage with `cargo-llvm-cov --branch`.

This directory holds the checked-in policy snapshot that keeps the gate practical:

- `.ci/coverage-baseline.txt` stores the current branch and line coverage baseline
- `scripts/check-coverage-baseline.sh` compares a fresh `lcov.info` snapshot against that baseline
- `scripts/update-coverage-baseline.sh` refreshes the baseline after an intentional improvement

## Local commands

```bash
rtk just coverage-summary
rtk just coverage-branch-gate
rtk just coverage-baseline-refresh
```

## Policy shape

- The gate is a ratchet, not a hard 80% blocker on day one
- `allowed_drop_percentage` is the regression budget
- `target_branch_coverage` documents the long-term target
- Codecov patch status is the front-door PR coverage policy at `95%` with `0%` threshold
- Codecov project status advertises the final `95%` target but remains informational during burn-down

In GitHub Actions, the gate script also writes a step summary so branch coverage, line coverage, baseline, and pass/fail status are visible without digging through raw logs.

## When to refresh the baseline

Refresh `.ci/coverage-baseline.txt` only when the new parser coverage snapshot is intentional and accepted.

Typical flow:

1. Improve tests or coverage scope
2. Run `rtk just coverage-branch-gate` to confirm the lane is green
3. Run `rtk just coverage-baseline-refresh`
4. Commit the updated baseline with the related test/coverage change

Do not refresh the baseline to hide an accidental regression.
