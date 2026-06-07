# Post-Wave Cleanup Checklist

Run after every agent wave completes. Prevents worktree and storage accumulation
that degrades subsequent waves.

Cross-reference:
- [docs/project/FRICTION_LOG.md](../project/FRICTION_LOG.md) — platform quirks, Windows path issues
- [ORCHESTRATION_ROLES.md](ORCHESTRATION_ROLES.md) — cleanup role definition

---

## Step 1 — Git status check

Verify no uncommitted changes remain in the main checkout or active worktrees.

```bash
git status --short --branch
```

Expected output: clean working tree, no staged or unstaged changes.
If files appear: identify the task that owns them. If the task is complete, discard:

```bash
git restore <file>
```

Never use `git stash` — the stash list is shared across all worktrees and will
contaminate other agents' sessions if popped in the wrong context.

---

## Step 2 — Worktree audit

List all worktrees and identify which are owned by completed tasks.

```bash
git worktree list
```

For each task-owned worktree that is no longer needed:

```bash
# Remove the worktree
git worktree remove <path>

# If remove fails due to dirty state, use force (after confirming no needed changes)
git worktree remove --force <path>

# Prune stale worktree metadata (safe to run always)
git worktree prune
```

**Locked worktree recovery (stale process):** If `git worktree remove` fails with
"is locked", a process is holding the worktree open. Recovery order:

1. Identify and kill the holding process (Task Manager on Windows, `kill` on Linux).
2. Then run `git worktree remove` — do not skip step 1 and try `--force` first.
   Force-removing a live worktree corrupts the git index.

Do not remove worktrees owned by other lanes or in-flight tasks. Check the
`lane:` label on the associated PR before removing.

---

## Step 3 — Delete task branches

After a PR is merged, delete the task branch locally and remotely when safe.

```bash
# Local
git branch -d <branch-name>

# Remote — only when the PR is confirmed merged and no dependent PRs use the branch as base
git push origin --delete <branch-name>
```

**When NOT to delete remote branches:**
- The PR is not yet merged.
- Another open PR uses this branch as its base.
- The branch name matches a pattern owned by another lane.

---

## Step 4 — Remove task-owned target/ and temp-receipt dirs

Generated output directories must not accumulate in the repo.

```bash
# Reconciliation output
rm -rf target/reconciliation/

# Any task-specific temp receipts (task-owned, not the permanent .receipts/ dir)
rm -rf target/temp-receipt-*/
```

Verify the permanent `.receipts/` directory is not touched — it is version-controlled
and contains release receipts. Only `target/` subdirectories are temporary.

---

## Step 5 — Check for stale cargo/rustc/xtask processes

Long-running cargo or xtask processes can hold file locks on Windows, preventing
worktree removal and branch switches.

```bash
# On Linux/macOS
ps aux | grep -E 'cargo|rustc|xtask' | grep -v grep

# On Windows (PowerShell)
Get-Process | Where-Object { $_.Name -match 'cargo|rustc|xtask' }
```

Kill any that are not part of an active task. On Windows, use Task Manager
or `Stop-Process -Name cargo -Force` (confirm before running).

---

## Step 6 — Storage doctor

Run the storage check and confirm no large repo-local `target/` directories remain.

```bash
./scripts/storage-doctor
```

Expected: no entries above the configured size threshold. If entries appear,
the agent that ran `cargo build/test` did not clean up. Trace the task via
`git log --oneline` and remove the left-behind directory.

---

## Platform Notes

### Windows: `core.longpaths` requirement

Nested worktrees on Windows can exceed the 260-character path limit, causing
silent failures in `git` and `cargo` operations. Verify this is set:

```bash
git config --global core.longpaths
# Expected output: true
```

If not set: `git config --global core.longpaths true`. This is a one-time setup
per machine. Agents running on fresh CI runners must apply this before any
worktree operations.

### Windows: locked-worktree recovery order

1. **Kill holding processes first** (Task Manager or PowerShell `Stop-Process`).
2. **Then** run `git worktree remove <path>` (without `--force`).
3. Only use `--force` if step 2 still fails after processes are confirmed dead.

Reversing this order (force before kill) can leave the git index in a partial
state that requires `git worktree prune` and manual ref cleanup to recover.

---

## Checklist Summary

```
[ ] git status --short --branch: clean
[ ] git worktree list: no task-owned worktrees for completed tasks
[ ] git worktree prune: ran successfully
[ ] Task branch deleted local + remote (where safe)
[ ] target/reconciliation/ removed
[ ] target/temp-receipt-*/ removed
[ ] No stale cargo/rustc/xtask processes
[ ] ./scripts/storage-doctor: no large target/ entries
[ ] core.longpaths=true (Windows only)
```
