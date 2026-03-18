---
name: triage-prs
description: Triage duplicate/stale PRs — cluster by topic, compare, keep best, close rest
user-invocable: true
argument-hint: "[optional: PR number range or 'all']"
---

# Triage PRs

Triage open PRs. Focus: **$ARGUMENTS**

## Steps

1. List all open PRs: `gh pr list --state open --json number,title,mergeable`
2. Cluster by topic (same files or same feature name)
3. For each cluster with >1 PR, launch parallel explore agents to compare:
   ```
   Agent(subagent_type="Explore", prompt="Compare PRs #X, #Y, #Z.
   Run: gh pr diff <num> --stat && gh pr view <num> --json body,additions,deletions
   Return: which is most complete, which has unique good ideas, which to keep")
   ```
4. For each cluster, pick the best PR
5. Close duplicates: `gh pr close <num> --comment "Closing — keeping #<best>"`
6. If winner needs improvements from closed PRs, spawn a worktree agent to incorporate them
7. Report: how many clusters found, how many closed, which PRs to merge
