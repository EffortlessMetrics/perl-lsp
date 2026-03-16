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

### 3. Mark ready

```bash
gh pr ready $NUMBER
```

### 4. Report

Output: "PR #$NUMBER marked ready -- CI will trigger."

Include the PR URL for convenience:
```bash
gh pr view $NUMBER --json url --template '{{.url}}'
```
