---
description: Check master branch CI status and block if red
argument-hint: "[--block] [--verbose]"
user-invocable: false
---

# Verify Master Green

Check whether master branch CI is green before proceeding with operations. Context: **$ARGUMENTS**

> **Branch naming:** the default branch of this repo is `main`. "Master" in this doc refers to it; all commands below use `main` (a stale `master` ref caused a ~2h CI stall — see AGENT_CATALOG "Base ref is `origin/main`").

## Steps

### 1. Fetch latest master status
```bash
git fetch origin main
```

### 2. Check CI status on master
```bash
gh run list --branch main --limit 5 --json status,conclusion,name,headSha,createdAt
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__actions_list(method:"list_workflow_runs", owner, repo, workflow_runs_filter:{branch:"main"}, per_page:5)` — full parity; each run carries `status`, `conclusion`, `head_sha`, and workflow name. See [docs/reference/GH_MCP_FALLBACK.md](../../docs/reference/GH_MCP_FALLBACK.md).

### 3. Classify master health

- **Green**: Most recent `CI Gate` workflow has `conclusion: success` -> safe to proceed
- **Pending**: Most recent run has `status: in_progress` -> wait or warn
- **Red**: Most recent `CI Gate` has `conclusion: failure` -> block and diagnose

### 4. If red, diagnose the cause

Identify the breaking commit:
```bash
gh run list --branch main --limit 5 --json headSha,conclusion,name,createdAt
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__actions_list(method:"list_workflow_runs", owner, repo, workflow_runs_filter:{branch:"main"}, per_page:5)` — compare `head_sha` and `conclusion` across runs to find the first red SHA.

Check which PR was most recently merged:
```bash
gh pr list --state merged --base main --limit 5 --json number,title,mergedAt,headRefName
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__list_pull_requests(owner, repo, state:"closed", base:"main", perPage:5)` then filter for merged status in agent code (check `.merged_at` field is non-null).

Cross-reference the failing commit SHA with the merged PR to identify the culprit.

Check the failing run's logs:
```bash
gh run view <run-id> --log-failed
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__get_job_logs(owner, repo, run_id:<run-id>, failed_only:true, return_content:true, tail_lines:500)` — returns the failing jobs' log tails directly. For a single job: `mcp__github__get_job_logs(owner, repo, job_id:<job-id>, return_content:true)`.

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
