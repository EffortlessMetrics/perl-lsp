---
description: Check and restore the correct git identity (Steven Zimmerman, CPA) if it's been corrupted to "test"
---

# Fix Git Identity

Verifies git user.name and user.email are set correctly, and restores them if they've been corrupted (commonly to `test / test@test.com` by sandbox initialization or an agent violating the "NEVER update git config" rule).

See `memory/feedback_git_config_test_identity_leak.md` for background.

## Correct identity

```
user.name  = Steven Zimmerman, CPA
user.email = 15812269+EffortlessSteven@users.noreply.github.com
```

## Steps

### 1. Check current state (main checkout)

```bash
git config --get user.name
git config --get user.email
git config --global --get user.name
git config --global --get user.email
```

If any of these print `test` or `test@test.com`, proceed to step 2.

### 2. Restore global config

```bash
git config --global user.name "Steven Zimmerman, CPA"
git config --global user.email "15812269+EffortlessSteven@users.noreply.github.com"
```

### 3. Clear local overrides

Local config overrides global. Clear any stale local entries:

```bash
git config --local --unset user.name 2>/dev/null || true
git config --local --unset user.email 2>/dev/null || true
```

### 4. Check existing worktrees

Worktrees inherit from the local config of the parent repo **at creation time** — they don't re-read global config when it changes. Walk every active worktree and unset local overrides there too:

```bash
git worktree list | awk '{print $1}' | while read wt; do
  echo "=== $wt ==="
  git -C "$wt" config --local --unset user.name 2>/dev/null
  git -C "$wt" config --local --unset user.email 2>/dev/null
  echo "name:  $(git -C "$wt" config --get user.name)"
  echo "email: $(git -C "$wt" config --get user.email)"
done
```

### 5. Verify recent commits

```bash
git log --oneline --format="%h %an <%ae>" -10
```

If recent commits show `test <test@test.com>`, they have already been pushed with the wrong identity. **Do not rewrite history** unless the user explicitly authorizes `git push --force` — retroactive re-authoring requires force-push, which overwrites published history.

### 6. Report

- State before (local + global)
- Actions taken
- State after
- Count of recent commits with wrong author (informational, not fixed)

## When NOT to use this skill

- If `user.name` is already correct, do nothing. Don't re-set global config when it already matches.
- If the user is on a different machine with a different preferred identity, verify with them first — the values in this skill are hard-coded for the primary maintainer's machine.

## Prevention

- CLAUDE.md has a top-level rule: **NEVER update the git config**. Violating agents set bad values.
- If you're about to make a commit and `git config --get user.name` returns `test`, STOP and run this skill first.
- Consider adding a pre-commit hook that refuses commits when `user.name == "test"`.
