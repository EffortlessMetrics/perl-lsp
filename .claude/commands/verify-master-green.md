---
description: Check master branch CI status and block if red
argument-hint: "[--block] [--verbose]"
user-invocable: false
---

# Verify Master Green

Check whether master branch CI is green before proceeding with operations. Context: **$ARGUMENTS**

## Steps

### 1. Fetch latest master status
```bash
git fetch origin master
```

### 2. Check CI status on master
```bash
gh run list --branch master --limit 5 --json status,conclusion,name,headSha,createdAt
```
> **MCP sessions:** `gh run list` has no MCP equivalent. In MCP sessions, workflow run listing is unavailable — classify master health as UNKNOWN and proceed with caution. Use `mcp__github__pull_request_read(method:"get_status")` on a recently-merged PR against master as a proxy if available.

### 3. Classify master health

- **Green**: Most recent `CI Gate` workflow has `conclusion: success` -> safe to proceed
- **Pending**: Most recent run has `status: in_progress` -> wait or warn
- **Red**: Most recent `CI Gate` has `conclusion: failure` -> block and diagnose

### 4. If red, diagnose the cause

Identify the breaking commit:
```bash
gh run list --branch master --limit 5 --json headSha,conclusion,name,createdAt
```
> **MCP sessions:** `gh run list` has no MCP equivalent. In MCP sessions, workflow run listing is unavailable — treat this step as best-effort.

Check which PR was most recently merged:
```bash
gh pr list --state merged --base master --limit 5 --json number,title,mergedAt,headRefName
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__list_pull_requests(owner, repo, state:"closed", base:"master", perPage:5)` then filter for merged status in agent code (check `.merged_at` field is non-null).

Cross-reference the failing commit SHA with the merged PR to identify the culprit.

Check the failing run's logs:
```bash
gh run view <run-id> --log-failed
```
> **MCP sessions:** `gh run view` has no MCP equivalent. Workflow run log access is unavailable in MCP sessions. Link to the GitHub Actions UI for manual inspection.

### 5. Report

Output status summary:

```
### Master CI Status
- **Status**: GREEN / RED / PENDING
- **Last commit**: <sha> (<message>)
- **Last CI run**: <url> (<conclusion>)
- **Diagnosis** (if red): <cause>
- **Suggested fix** (if red): <action>
```

### 6. Block decision

If `--block` is specified and master is RED:
- Do NOT proceed with any merge, rebase, or deploy operations
- Output: "BLOCKED: Master is red. Fix master before continuing."

Suggested fixes when red:
- If a recent merge broke it: suggest reverting that PR
- If it's a flaky test: suggest re-running the workflow
- If it's a dependency issue: suggest checking `Cargo.lock` or `deny.toml`
