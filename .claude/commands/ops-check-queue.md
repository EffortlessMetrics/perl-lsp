---
description: Ops step 1 — find merge-ready PRs in the queue
user-invocable: false
---

# Ops Check Queue

Find PRs that are ready to merge.

## Steps

### Step 0 — Sweep stale in-build claims

Before checking the merge queue, clear noise from stale builder claims.

Issues with `in-build` but no linked open PR for more than 7 days are routing dead weight.
Each one adds latency to every orchestrator dispatch decision.

```bash
gh issue list --label "in-build" --state open --json number,title,updatedAt --jq '.[] | select((now - (.updatedAt | fromdateiso8601)) > (7*86400)) | "#\(.number) \(.title)"'
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__list_issues(owner, repo, labels:["in-build"], state:"OPEN")` → filter by `updatedAt` in agent logic for >7 days stale (the `since` parameter filters issues updated *after* a date, not before — apply the stale-age filter client-side).

For each result, check if a linked open PR exists:

```bash
gh pr list --search "closes #<number>" --state open --json number,title
```
> **MCP alternative (web/no-gh sessions):** `mcp__github__search_pull_requests(query:"closes #<number> is:open repo:effortlessmetrics/perl-lsp-swarm")`

Classify and act:
- **Has open PR**: skip — builder is active.
- **No open PR, > 7 days stale**: remove `in-build` label and add a comment: `in-build label removed — no linked PR after 7 days; issue returned to queue`.

1. List all open PRs with their merge state:
   ```bash
   gh pr list --state open --limit 50 --json number,title,mergeable,mergeStateStatus,isDraft,reviewDecision --jq '.[] | "\(.number)\t\(.mergeable)/\(.mergeStateStatus)\tdraft:\(.isDraft)\treview:\(.reviewDecision)\t\(.title)"'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__list_pull_requests(owner, repo, state:"open", perPage:50)` — `mergeable`, `mergeStateStatus`, `isDraft`, `reviewDecision` fields available on each PR object.

2. Filter for merge candidates:
   - **mergeStateStatus = CLEAN** (NOT UNSTABLE, NOT UNKNOWN, NOT DIRTY). UNSTABLE means non-required check failing or in flight — wait, don't merge.
   - Not a draft (or promote with `gh pr ready` if appropriate)
   - reviewDecision: APPROVED or no review required
   - **No active `needs-*` routing label** (per the 2026-04-26 sign-off-as-routing rule). Filter out PRs with any of: `needs-builder-fix`, `needs-ci-fix`, `needs-diff-fix`, `needs-spec-fix`, `needs-red-tdd-fix`. Sign-off and routing labels are mutually exclusive at the same gate; presence of `needs-*` means the gate has not actually cleared.

   Filter command (post-Step-1 list narrowing):
   ```bash
   gh pr list --state open --label merge-ready --limit 50 --json number,labels,mergeStateStatus,isDraft -q '[.[] | select(.isDraft | not) | select(.mergeStateStatus == "CLEAN") | select(.labels | map(.name) | (contains(["needs-builder-fix"]) or contains(["needs-ci-fix"]) or contains(["needs-diff-fix"]) or contains(["needs-spec-fix"]) or contains(["needs-red-tdd-fix"])) | not)] | .[].number'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__search_pull_requests(query:"is:open is:pr label:merge-ready repo:effortlessmetrics/perl-lsp-swarm")` then filter `needs-*` labels in agent code after fetching.

3. Check CI on each candidate using **latest-per-check filter** (per `feedback_status_check_rollup_stale_entries.md`):
   ```bash
   gh pr view <number> --json statusCheckRollup --jq '.statusCheckRollup | group_by(.name // .context) | map(sort_by(.completedAt // .startedAt) | last) | [.[] | select(.conclusion == "FAILURE") | (.context // .name)]'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get_check_runs", owner, repo, pullNumber:<number>)` → apply the same group-by-name, sort-by-completedAt, take-last logic in agent code rather than jq.

4. Classify:
   - **MERGE NOW**: CLEAN + latest CI all green + no `needs-*` + `just pre-merge-check <number>` passes
   - **WAIT**: CI still running (mergeStateStatus = UNSTABLE / UNKNOWN)
   - **BLOCKED**: CI failures on latest run — note which check failed
   - **NEEDS REBASE**: CONFLICTING / DIRTY
   - **CONTRADICTORY**: has `merge-ready` AND a `needs-*` label — strip `merge-ready` and route to the appropriate fixer

## Output

Record in your task:
```
Merge candidates: #NNN, #NNN, #NNN
Blocked: #NNN (reason), #NNN (reason)
Waiting: #NNN (CI running)
```
