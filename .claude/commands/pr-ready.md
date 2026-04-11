---
description: Mark a reviewed draft PR as ready for CI
argument-hint: "<PR number>"
---

# Mark PR Ready

Mark a reviewed draft PR as ready for merge. This triggers CI. Context: **$ARGUMENTS**

## Steps

### 1. Parse PR number

Extract the PR number from $ARGUMENTS. If not provided, list open draft PRs:
```bash
gh pr list --state open --draft --json number,title,headRefName --template '{{range .}}#{{.number}} {{.title}} ({{.headRefName}}){{"\n"}}{{end}}'
```

### 2. Verify PR exists and is a draft

```bash
gh pr view $NUMBER --json isDraft,title,state
```

If the PR is not a draft, report: "PR #N is already marked ready" and stop.
If the PR is not open, report the current state and stop.

### 3. Verify review coverage

```bash
gh pr view $NUMBER --json labels,files
```

Policy:
- If the PR has the `reviewed-deep` label, proceed.
- If it does not have `reviewed-deep`, it may proceed **only** when every changed file is docs-only (`docs/**` or doc-text files such as `.md`, `.mdx`, `.txt`, `.rst`, `.adoc`).
- Otherwise: **STOP.** Report: "PR #$NUMBER cannot be marked ready — missing `reviewed-deep` on a non-docs PR. Route to reviewer-deep first."

Optionally validate receipt freshness when `reviewed-deep` is present to ensure the deep review covers the current HEAD:
```
/label-receipt-validate pr $NUMBER reviewed-deep
```

### 4. Mark ready and signal merge-readiness

```bash
gh pr ready $NUMBER
gh pr edit $NUMBER --add-label "merge-ready"
```

The `merge-ready` label signals the ops agent that this PR has passed review and is cleared for merge pickup.

### 5. Write version-bound receipt

Record the label binding against the current HEAD SHA so the orchestrator can detect staleness:

```
/label-receipt-write pr $NUMBER merge-ready pr-ready
```

### 6. Report

Output: "PR #$NUMBER marked ready -- CI will trigger. Labeled merge-ready for ops pickup."

Include the PR URL for convenience:
```bash
gh pr view $NUMBER --json url --template '{{.url}}'
```
