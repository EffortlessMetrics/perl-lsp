---
description: Ops step 2 — merge a batch of up to 3 PRs
user-invocable: false
---

# Ops Merge Batch

Merge up to 3 PRs from the candidates identified in step 1.

## Steps

1. Pick up to 3 PRs from the MERGE NOW list.
   Respect dependency order:
   - If PR B depends on PR A (same files), merge A first
   - Parser fixes before corpus ratchets
   - Infrastructure before features

2. **Fresh green check** — immediately before each merge, verify live state:
   ```bash
   gh pr view <number> --json isDraft,mergeable,mergeStateStatus,labels,headRefOid,reviewDecision,statusCheckRollup
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", owner, repo, pullNumber:<number>)` for isDraft, mergeable, mergeStateStatus, labels, headRefOid, reviewDecision fields; additionally call `mcp__github__pull_request_read(method:"get_check_runs", pullNumber:<number>)` for CI status rollup.
   All of these must be true AT MERGE TIME (not remembered from earlier):
   - Not draft
   - mergeStateStatus = CLEAN (NOT UNSTABLE, NOT UNKNOWN, NOT DIRTY)
   - CI checks green on the current HEAD SHA using **latest-per-check filter** (per `feedback_status_check_rollup_stale_entries.md`)
   - No blocking review comments
   - **NO active `needs-*` label** (per the 2026-04-26 sign-off-as-routing rule: presence of `needs-builder-fix` / `needs-ci-fix` / `needs-diff-fix` / `needs-spec-fix` / `needs-red-tdd-fix` MUST block merge regardless of `merge-ready`). Sign-off is one of the routing decisions; if any gate ALSO bounced, the PR is not actually signed off.
   - **Workspace-wide CI checks SUCCESS** (Compile All Targets, PR Smoke including workspace fmt, Windows Guardrails compile/module-separator/sandbox), not just per-crate. Per the 2026-04-26 master-green directive: per-crate gates miss workspace drift.

3. **Policy gate (defense-in-depth)** — run the scripted pre-merge guard:
   ```bash
   just pre-merge-check <number>
   # or: bash scripts/pre-merge-check.sh <number>
   ```
   This codifies the policy:
   - non-docs PRs need `deep-reviewed`
   - docs-only PRs may merge with `merge-ready` alone
   - draft/title/label mistakes still fail loud
   If it fails, **skip this PR** with the script's reason.

4. **Build a good commit message** for each PR:
   ```bash
   # Get the PR title and body
   gh pr view <number> --json title,body
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", owner, repo, pullNumber:<number>)` → `.title`, `.body` fields.
   The squash commit message should be: `<PR title> (#<number>)` as the first line,
   followed by a blank line and a 1-3 sentence summary of WHAT changed and WHY.
   Future readers should understand the change without opening the PR.

5. Merge each PR with squash:
   ```bash
   gh pr merge <number> --squash --subject "<title> (#<number>)" --body "<summary>"
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__merge_pull_request(owner, repo, pullNumber:<number>, merge_method:"squash", commit_title:"<title> (#<number>)", commit_message:"<summary>")` — full parity including squash and custom commit message.

6. After each merge, verify it landed and clean up labels:
   ```bash
   gh pr view <number> --json state --jq .state
   # Remove merge-ready from the now-merged PR
   gh pr edit <number> --remove-label "merge-ready"
   # Remove deep-reviewed from the now-merged PR
   gh pr edit <number> --remove-label "deep-reviewed"
   # Remove in-build from the linked issue (if any)
   CLOSING_ISSUE=$(gh pr view <number> --json closingIssuesReferences --jq '.closingIssuesReferences[0].number // empty')
   if [ -n "$CLOSING_ISSUE" ]; then
     gh issue edit "$CLOSING_ISSUE" --remove-label "in-build"
   fi
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", pullNumber:<number>)` → `.state` for merge verification. For label removal: read current labels with `pull_request_read`, then write back the filtered list with `mcp__github__issue_write(method:"update", issue_number:<number>, labels:[...current minus removed label])`. Note: `issue_write` labels field replaces the full list — always read current labels first before writing.
   Label cleanup prevents stale `merge-ready`, `deep-reviewed`, and `in-build` labels from
   misleading future orchestrator queries.

7. If a merge fails or pre-check fails:
   - CONFLICTING → skip, note "needs rebase"
   - CI red or pending → skip, note "CI not green on current HEAD"
   - CI green on old SHA → skip, note "stale CI — needs rerun"
   - Draft → skip, note "still in review"
   - Missing `deep-reviewed` on a non-docs PR → skip, note "missing deep review signal"
   - **Active `needs-*` label** → skip, note "sign-off contradicted by needs-* routing label; gate has not actually cleared"
   - mergeStateStatus = UNSTABLE → skip, note "non-required check failing or in flight; verify which before forcing"

8. **After each batch of 3** — verify master is genuinely green BEFORE starting the next batch:
   ```bash
   gh run list --workflow=CI --branch=master --limit=3 --json conclusion,headSha,event,name
   ```
   > **MCP sessions:** `gh run list` / `gh run view` have no MCP equivalent. Treat as best-effort: in MCP sessions, workflow run listing is unavailable. Use `mcp__github__pull_request_read(method:"get_status")` on any recently-merged PR against master as a proxy signal, or note the limitation and proceed with caution.
   Required: latest master CI run on the merged SHA = SUCCESS. If master goes red post-merge:
   - Halt the queue immediately
   - Report which merge introduced the regression (compare master CI logs to recent merges)
   - Dispatch a master-fix path (narrow fix PR, admin-merge, cascade-update queued PRs)
   - Do NOT continue merging until master is verified green

## Rules

- NEVER use `--admin` or `--force`
- NEVER merge more than 3 in one batch
- If merge fails twice, skip and move to next PR
- Note which PRs contained parser fixes (for corpus ratchet)

## Output

Record in your task:
```
Merged: #NNN, #NNN, #NNN
Skipped: #NNN (reason)
Parser fixes merged: yes/no (for ratchet decision)
```
