# 2026-05-03 — GitHub Auto-Closes Dependent PRs on Base Branch Delete

**Lens**: When merging with `--delete-branch`, dependent PRs (PRs whose base is the deleted branch) are *closed*, not retargeted. Affects PR-chaining workflows.

## What we hit

During the v0.13.3 release pipeline, two PRs were chained:

- `#7870`: hardening fix on branch `fix/vscode-managed-install-013-3` → base `master`
- `#7871`: release-prep on branch `chore/prepare-v0.13.3` → base `fix/vscode-managed-install-013-3` (chained off #7870 so it could be reviewed in parallel)

Plan: merge `#7870` with `--delete-branch`, then `#7871` would auto-retarget to `master`.

Actual:

```bash
$ gh pr merge 7870 --repo <owner>/<repo> --squash --delete-branch
# (#7870 merged, branch deleted)

$ gh pr view 7871 --json state,baseRefName
{"state":"CLOSED","baseRefName":"fix/vscode-managed-install-013-3","mergeable":"CONFLICTING"}
```

`#7871` was *closed*, not retargeted. The base branch reference is preserved (now stale) and `mergeable` is `CONFLICTING` because the base no longer exists.

## What's actually documented

GitHub's documented behavior for branch deletion: open PRs that *target* the deleted branch as their base are automatically closed. PRs that have the deleted branch as their *head* (the source) are also closed.

This is consistent with how GitHub treats branch deletion as a structural change to the PR graph — there's no "auto-rebase to a sensible new base" semantic.

## Recovery

Cherry-pick the dependent PR's commit(s) onto a fresh branch from the new base, push, open a new PR:

```bash
git switch -c chore/release-prep-013-3 origin/master
git cherry-pick <commit-from-closed-pr>
git push -u origin chore/release-prep-013-3
gh pr create --base master --head chore/release-prep-013-3 --title "..." --body "..."
```

The closed PR cannot be reopened cleanly because its base no longer exists. Reference the closed PR in the new PR's body for trail (e.g., "supersedes #7871").

## How to avoid

Two options:

**Option A: don't chain release PRs.** Open both off `master` independently, even if one has to be rebased after the other lands. The rebase is cheap; the auto-close recovery is more painful.

**Option B: don't delete the base branch on first merge.** Merge `#7870` with `--squash` but *not* `--delete-branch`. Once `#7871` retargets manually to `master`, then delete the orphaned branch.

Option A is simpler. Option B requires remembering to clean up. The repo's release-merge default of `--delete-branch` is generally good (keeps the branch list clean); the chained-PR pattern is the exception.

## Detection signal

If you opened a PR with a non-master base branch, before merging the parent PR run:

```bash
gh pr list --search "base:<parent-branch>" --state open --json number,title,headRefName
```

If anything shows up, those PRs will be auto-closed on `--delete-branch`. Either retarget them first (`gh pr edit <N> --base master`) or skip `--delete-branch` on the parent merge.

## Related

- The actual recovery during the v0.13.3 closeout: `#7871` closed → `#7872` opened from a fresh branch.
- Articles: `../articles/RELEASES_FAIL_AT_SEAMS.md` (this is a workflow-tooling seam).
- Reference: `../reference/AGENT_HANDOFF_PROTOCOL.md` (Step-0 verify catches this when the executor checks PR state before assuming retargeting).
