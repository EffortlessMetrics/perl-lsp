---
description: Diff auditor step 1 — review the complete PR diff for coherence and cleanliness
user-invocable: false
---

# Diff Audit: Check

Review the cumulative diff from all agents.

## Steps

1. Get the PR file list and diff using the GitHub API (NEVER use `gh pr diff` — it shows
   branch-vs-current-master and produces false contamination claims on stale-base PRs):
   ```bash
   # Authoritative PR file list (PR-authored only):
   REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
   gh api repos/$REPO/pulls/<number>/files --jq '.[].filename'
   gh api repos/$REPO/pulls/<number>/files --jq '.[] | {filename, patch: (.patch // "(binary)")}'

   # Full authored diff (three-dot — only what the PR added, not inherited state):
   git diff $(git merge-base origin/master HEAD)..HEAD --stat
   git diff $(git merge-base origin/master HEAD)..HEAD
   ```
   Before flagging any file as cross-PR contamination: confirm it appears in the `pulls/N/files`
   API response (PR-authored). If it only appears in a `gh pr diff` (branch-vs-master snapshot)
   it is inherited base state — NOT scope drift. This self-check is mandatory before any
   SCOPE DRIFT or CONTAMINATION verdict.

2. Read the spec:
   ```bash
   gh pr checkout <number>
   cat .spec/*/acceptance.md 2>/dev/null
   ```

3. Check each acceptance criterion against the diff — is it implemented?

4. Search for leftover artifacts:
   ```bash
   git diff origin/master..HEAD | grep -iE "TODO|FIXME|HACK|XXX|dbg!|println!|eprintln!|#\[allow"
   ```

5. Check commit history coherence:
   ```bash
   git log origin/master..HEAD --oneline
   ```

6. Verify tests still pass (catch late-commit regressions):
   ```bash
   cargo test -p <crate>
   ```

7. Check PR metadata:
   ```bash
   gh pr view <number> --json title,isDraft,labels
   ```
