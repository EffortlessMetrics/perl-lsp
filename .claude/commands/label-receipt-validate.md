---
description: Validate a pipeline label's freshness against the current artifact version
argument-hint: "<pr|issue> <number> <label>"
---

# Label Receipt Validate

Check whether a pipeline label is still fresh (bound to the current version of the
artifact) or stale (artifact has changed since the label was set).

Context: **$ARGUMENTS**

## Steps

### 1. Parse arguments

Extract from $ARGUMENTS:
- `artifact_type`: `pr` or `issue`
- `number`: PR or issue number
- `label`: the pipeline label to validate (e.g., `merge-ready`, `in-build`)

If arguments are missing, report usage and stop:
```
Usage: /label-receipt-validate <pr|issue> <number> <label>
Example: /label-receipt-validate pr 2645 merge-ready
```

### 2. Get current artifact version

For PRs:
```bash
CURRENT_SHA=$(gh pr view $NUMBER --json headRefOid --jq '.headRefOid')
CURRENT_UPDATED=$(gh pr view $NUMBER --json updatedAt --jq '.updatedAt')
```

For issues:
```bash
CURRENT_UPDATED=$(gh issue view $NUMBER --json updatedAt --jq '.updatedAt')
CURRENT_SHA="n/a"
```

### 3. Find the receipt comment

Use the `issues` endpoint for both PRs and issues -- on GitHub's API, receipt comments
created via `gh pr comment` are issue-type comments, and `pulls/.../comments` only returns
line-level review comments.

```bash
RECEIPT_BODY=$(gh api "repos/{owner}/{repo}/issues/$NUMBER/comments" \
  --jq '[.[] | select(.body | contains("<!-- LABEL_RECEIPT_v1 -->"))] | last | .body')
```

### 4. Check for receipt existence

**If no receipt comment found:**
```
Result: NO_RECEIPT
  Label "$LABEL" on $ARTIFACT_TYPE #$NUMBER has no receipt.
  The label may have been set before receipts were enabled.
  Recommendation: treat as STALE and re-evaluate.
```

### 5. Extract and validate binding

Parse the JSON from between the `<!-- LABEL_RECEIPT_v1 -->` markers.
Find the binding entry where `label` matches and `valid` is `true`.

**If no binding found for this label:**
```
Result: NO_BINDING
  No receipt binding for "$LABEL" on $ARTIFACT_TYPE #$NUMBER.
  The label was not recorded via /label-receipt-write.
  Recommendation: treat as STALE and re-evaluate.
```

### 6. Compare versions

**For PRs:**
Compare `bound_at_version` (the SHA when label was set) against `CURRENT_SHA` (current HEAD).

- If they match: the label was set against the current code.
- If they differ: the code has changed since the label was set.

**For issues:**
Compare `bound_at_timestamp` against the issue's `updated_at`.

- If `bound_at_timestamp` >= issue `updated_at`: label was set after last update.
- If `bound_at_timestamp` < issue `updated_at`: issue was updated after label was set.

### 7. Report

**FRESH** (safe to trust):
```
Result: FRESH
  Label "$LABEL" on $ARTIFACT_TYPE #$NUMBER is current.
  Bound at: $BOUND_SHA ($BOUND_TIMESTAMP)
  Current:  $CURRENT_SHA ($CURRENT_UPDATED)
  Agent: $BOUND_BY_AGENT
  Recommendation: safe to route based on this label.
```

**STALE** (artifact changed):
```
Result: STALE
  Label "$LABEL" on $ARTIFACT_TYPE #$NUMBER is outdated.
  Bound at: $BOUND_SHA ($BOUND_TIMESTAMP)
  Current:  $CURRENT_SHA ($CURRENT_UPDATED)
  Agent: $BOUND_BY_AGENT
  Recommendation: re-evaluate before routing. The artifact has changed since this label was set.
```

## Freshness Rules

| Artifact Type | Fresh When | Stale When |
|--------------|-----------|-----------|
| Pull Request | `bound_at_version` == current HEAD SHA | HEAD SHA changed since binding |
| Issue | `bound_at_timestamp` >= `updated_at` | Issue updated after binding |

## Notes

- This skill only reads and reports; it does not modify labels or receipts
- `NO_RECEIPT` and `NO_BINDING` are distinct from `STALE` -- they indicate missing data, not outdated data
- Agents should call this before trusting labels for routing decisions
- The ops-merge-batch skill already does its own fresh CI check at merge time, so `merge-ready` staleness is less critical than `plan-reviewed` or `builder-ready` staleness
