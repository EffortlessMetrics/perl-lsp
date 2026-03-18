---
name: triage-prs
description: Triage duplicate/stale PRs — cluster by topic, compare, keep best, close rest
disable-model-invocation: true
user-invocable: true
argument-hint: "[optional: PR number range or 'all']"
---

# Triage PRs

Triage open PRs. Focus: **$ARGUMENTS**

## Steps

1. List all open PRs with freshness and branch metadata:
   `gh pr list --state open --json number,title,headRefName,mergeable,updatedAt --limit 100`
2. Build preliminary clusters by topic from title/head branch naming only.
   Do not assume "same files" from the list output alone.
3. For each possible duplicate cluster, fetch file-level detail before deciding:
   `gh pr view <num> --json body,files,updatedAt,mergeable,reviewDecision,headRefName`
4. Launch parallel explore agents only after file overlap is confirmed:
   ```
   Agent(subagent_type="Explore", prompt="Compare PRs #X, #Y, #Z.
   Run: gh pr diff <num> --stat && gh pr view <num> --json body,files,updatedAt,additions,deletions
   Return: which is most complete, which has unique good ideas, which to keep, and whether file overlap is real")
   ```
5. For each confirmed cluster, pick the best PR using:
   - changed-file overlap
   - recency (`updatedAt`)
   - review/merge state
   - implementation completeness
6. Treat a PR as stale only when it is both older and superseded by a newer PR in the same confirmed file/topic cluster.
   Do not close PRs based on age alone.
7. Close duplicates only after explicit comparison:
   `gh pr close <num> --comment "Closing — keeping #<best>"`
8. If the winner needs improvements from closed PRs, spawn a worktree agent to incorporate them first.
9. Report: how many clusters found, how many closed, which PRs remain blocked, and which PRs are ready to merge.
