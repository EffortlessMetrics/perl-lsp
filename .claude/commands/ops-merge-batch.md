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
   gh pr view <number> --json isDraft,mergeable,mergeStateStatus,labels,headRefOid,reviewRequests,reviewDecision,statusCheckRollup
   scripts/ci/check-pr-review-convergence <number>
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__pull_request_read(method:"get", owner, repo, pullNumber:<number>)` for isDraft, mergeable, mergeStateStatus, labels, headRefOid, reviewRequests, reviewDecision fields; additionally call `mcp__github__pull_request_read(method:"get_check_runs", pullNumber:<number>)` for CI status rollup. Review convergence still requires the canonical script — there is no MCP equivalent that pages both `latestReviews` and `reviewThreads` correctly.
   All of these must be true AT MERGE TIME (not remembered from earlier):
   - Not draft
   - mergeStateStatus = CLEAN (NOT UNSTABLE, NOT UNKNOWN, NOT DIRTY)
   - CI checks green on the current HEAD SHA using **latest-per-check filter** (per `feedback_status_check_rollup_stale_entries.md`)
   - No blocking review comments
   - **Review convergence** — `scripts/ci/check-pr-review-convergence <number>` exits `0`
     (see [.claude/reference/review-convergence.md](../reference/review-convergence.md)
     for the contract). Do not reproduce or modify its query locally. Never
     merge, and never enable/retain auto-merge, while it exits non-zero —
     that means a requested reviewer is still pending on the current HEAD SHA
     or an active thread remains unresolved.
   - **NO active `needs-*` label** (per the 2026-04-26 sign-off-as-routing rule: presence of `needs-builder-fix` / `needs-ci-fix` / `needs-diff-fix` / `needs-spec-fix` / `needs-red-tdd-fix` MUST block merge regardless of `merge-ready`). Sign-off is one of the routing decisions; if any gate ALSO bounced, the PR is not actually signed off.
   - **Workspace-wide CI checks SUCCESS** (Compile All Targets, PR Smoke including workspace fmt, Windows Guardrails compile/module-separator/sandbox), not just per-crate. Per the 2026-04-26 main-green directive: per-crate gates miss workspace drift.

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
   - mergeStateStatus = UNSTABLE → verify which check is red: if BOTH required checks (`Perl LSP Rust Small Result`, `ripr+ New Gap Gate`) are green on the current HEAD SHA, UNSTABLE from a non-required check is mergeable (per CLAUDE.md "Merge with UNSTABLE is OK"). Skip only if a required check is failing or still in flight.

8. **After each batch of 3** — verify main is genuinely green BEFORE starting the next batch:
   ```bash
   gh run list --workflow=CI --branch=main --limit=3 --json conclusion,headSha,event,name
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__actions_list(method:"list_workflow_runs", owner, repo, workflow_runs_filter:{branch:"main"}, per_page:5)` — full parity (each run carries `conclusion`, `head_sha`, `event`, workflow name). For failing-run logs use `mcp__github__get_job_logs(owner, repo, run_id:<id>, failed_only:true, return_content:true)`. See [docs/reference/GH_MCP_FALLBACK.md](../../docs/reference/GH_MCP_FALLBACK.md).
   Required: latest main CI run on the merged SHA = SUCCESS. If main goes red post-merge:
   - Halt the queue immediately
   - Report which merge introduced the regression (compare main CI logs to recent merges)
   - Dispatch a main-fix path (narrow fix PR, admin-merge, cascade-update queued PRs)
   - Do NOT continue merging until main is verified green

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
