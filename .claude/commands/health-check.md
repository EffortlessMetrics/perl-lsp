---
description: Quick codebase health scan — CI, PRs, tests, corpus, clippy, worktrees
argument-hint: ""
---

# Health Check: Quick Codebase Scan

Fast scan of overall codebase health. Outputs a formatted table to stdout (no GitHub issues).

## Checks

Run all of the following and report results in a summary table:

### 1. CI Status
```bash
gh run list --limit 3 --json status,conclusion,name,headBranch,createdAt
```

### 2. Open PRs
```bash
gh pr list --state open --json number,title,labels --limit 30 | jq length
```

### 3. Open Issues
```bash
gh issue list --state open --limit 100 --json number | jq length
```

### 4. Failing Tests
```bash
cargo test --workspace --lib --no-fail-fast 2>&1 | tail -5
```

### 5. Corpus Baseline
```bash
if [ -f .ci/parser-corpus-baseline.json ]; then
  cat .ci/parser-corpus-baseline.json | jq '.summary // .total // "present"'
else
  echo "NOT FOUND"
fi
```

### 6. Clippy Warnings
```bash
cargo clippy --workspace --lib 2>&1 | grep -c "^warning\[" || echo "0"
```

### 7. Active Worktrees
```bash
git worktree list | wc -l
```

### 8. Ignored Tests
```bash
grep -rc "#\[ignore" crates/*/tests/ crates/*/src/ --include="*.rs" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}'
```

### 9. Debt Budget
```bash
if [ -f .ci/debt-ledger.yaml ]; then
  echo "present"
else
  echo "NOT FOUND"
fi
```

### 10. Unused Dependencies
```bash
cargo machete 2>&1 | grep -c "unused" || echo "0"
```

### 11. Stale In-Build Issues
Detect issues labeled `in-build` that have no linked open PR and have not been updated in over 7 days.
These are evidence of a builder that never started, or a PR that merged without closing the issue.
Issues that also carry `structural-blocker` are excluded — those are legitimately stalled by design.

```bash
# Step 1: find candidate issues (age > 7 days, no structural-blocker label)
JQ_FILTER='[.[] | select((.labels | map(.name) | index("structural-blocker") == null) and ((now - (.updatedAt | fromdateiso8601)) > (7*86400)))] | map(.number)'
STALE_CANDIDATES=$(gh issue list --label "in-build" --state open --json number,updatedAt,labels --jq "$JQ_FILTER" 2>/dev/null || echo "[]")

# Step 2: for each candidate, verify no open PR references it
STALE_CONFIRMED=()
for NUM in $(echo "$STALE_CANDIDATES" | jq -r '.[]'  2>/dev/null); do
  PR_COUNT=$(gh pr list --state open --search "#${NUM} in:body" --limit 1 --json number --jq 'length' 2>/dev/null || echo "0")
  if [ "$PR_COUNT" = "0" ]; then
    STALE_CONFIRMED+=("$NUM")
  fi
done

if [ "${#STALE_CONFIRMED[@]}" -eq 0 ]; then
  echo "0"
else
  echo "${#STALE_CONFIRMED[@]}: issues ${STALE_CONFIRMED[*]}"
fi
```

Note: If `gh` is offline or rate-limited, the `|| echo` fallbacks ensure this check returns `0` rather than erroring.

## Output Format

Print a formatted table to stdout:

```
=== Health Check ===

| Check              | Status | Detail                    |
|--------------------|--------|---------------------------|
| CI (latest)        | OK/BAD | last 3 runs summary       |
| Open PRs           | <N>    | N open pull requests      |
| Open Issues         | <N>    | N open issues             |
| Tests              | OK/BAD | N pass, M fail            |
| Corpus baseline    | OK/BAD | present / not found       |
| Clippy warnings    | <N>    | N warnings                |
| Active worktrees   | <N>    | N worktrees               |
| Ignored tests      | <N>    | N ignored tests           |
| Debt ledger        | OK/BAD | present / not found       |
| Unused deps        | <N>    | N unused dependencies     |
| Stale in-build     | <N>    | N issues flagged (list #NN) |

Overall: OK / NEEDS ATTENTION (<list of BAD checks>)
```

## Thresholds

| Check | OK | NEEDS ATTENTION |
|-------|----|-----------------|
| CI | All recent runs succeeded | Any failure in last 3 runs |
| Open PRs | < 15 | >= 15 |
| Tests | All pass | Any failure |
| Corpus baseline | File exists | File missing |
| Clippy warnings | 0 | > 0 |
| Active worktrees | < 10 | >= 10 (cleanup needed) |
| Ignored tests | < 20 | >= 20 |
| Debt ledger | File exists | File missing |
| Unused deps | 0 | > 0 |
| Stale in-build | 0 | > 0 (builder never started or PR merged without closing) |

## When to Use

- Start of day: "Is everything OK before I start work?"
- Before a swarm cycle: "Is the baseline clean?"
- After a merge burst: "Did anything break?"
- Quick status check: faster than `/swarm-status` (no GitHub API calls for PRs/issues if offline)

## Notes

- This outputs to stdout only. No GitHub issues are created.
- For deeper investigation of any failing check, spawn a scout agent for that area.
- For full swarm state including PR details, use `/swarm-status`.
