# Known Pitfalls

Accumulated lessons from fixer agents and failed builds. Scouts and builders should read this before starting work to avoid repeating known mistakes.

This file is append-only during swarm operation. The janitor consolidates it periodically.

## Format

Each entry:
```
### <date> — <category>
**Source**: <branch or PR that discovered this>
**Pitfall**: <what went wrong>
**Fix**: <what the correct approach is>
**Affected crates**: <list>
```

## Entries

<!-- Agents append new entries below this line -->

### 2026-03-19 — Worktree Base Drift

**Source**: Cycle 4 session learnings
**Pitfall**: Worktrees created early in a session drift from master as other PRs merge. By the time a PR is created, the worktree may be many commits behind, causing rebase conflicts or stale code references.
**Fix**: Before creating a PR from a worktree, rebase onto current master and re-run verification. If the worktree is very stale (10+ commits behind), consider starting fresh.
**Affected crates**: all

### 2026-03-19 — CI Cancellation Cascades

**Source**: Cycle 4 merge waves
**Pitfall**: Merging PRs in rapid succession causes GitHub Actions to cancel in-progress CI runs for the master branch. Each new push to master cancels the previous run, so only the last merge in a rapid batch gets a full CI pass.
**Fix**: Merge in batches of 3, then wait for the CI run to complete before merging the next batch. Use `gh run list --branch master --limit 5 --json status,conclusion` to check.
**Affected crates**: all

### 2026-03-19 — Clippy Gate After Merge Waves

**Source**: Cycle 4 post-merge failures
**Pitfall**: Individual PRs pass clippy in isolation, but the merge commit combining multiple PRs can introduce new clippy warnings (unused imports from removed functions, new lints from dependency updates).
**Fix**: After merging a wave of PRs, check the master CI run. If clippy fails, spawn a fixer agent. Run `cargo clippy --workspace --lib -- -D warnings` locally after pulling merged master.
**Affected crates**: all

### 2026-03-19 — Draft PR Cleanup

**Source**: Cycle 4 PR queue bloat
**Pitfall**: Draft PRs accumulate when builders create them but reviewers do not pick them up. The PR list grows unwieldy and stale drafts cause confusion about work status.
**Fix**: Triage draft PRs at session start. Close stale drafts (no updates in 2+ days). For active drafts, mark ready or note blockers.
**Affected crates**: all

### 2026-03-19 — Status Update After Test Changes

**Source**: Cycle 4 CI failures
**Pitfall**: Adding or modifying tests changes computed metrics in `docs/project/CURRENT_STATUS.md`. If `just status-update && just status-check` is not run, the `policy_checks` CI gate fails.
**Fix**: Always run `just status-update && just status-check` after modifying test files, before committing.
**Affected crates**: all

### 2026-03-19 — Rebase Ours/Theirs Inversion

**Source**: Multiple cycle 3-4 rebase failures
**Pitfall**: During `git rebase`, `--ours` and `--theirs` are inverted compared to `git merge`. In rebase, `--ours` = upstream (master), `--theirs` = your commits being replayed.
**Fix**: During rebase, `--theirs` keeps YOUR changes. When in doubt, resolve conflicts manually.
**Affected crates**: all
