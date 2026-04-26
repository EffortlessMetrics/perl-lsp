# Acceptance Criteria — Pre-Tool-Use Hook Worktree Guard (#4464)

## Behavioral Assertions (Grid-Complete)

- [ ] **Block git worktree add from worktree** | BLOCK (exit 2) when `git worktree add ../foo master` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 23-27, error message "must be run from main checkout" | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 1
- [ ] **Block git checkout <branch> from worktree** | BLOCK (exit 2) when `git checkout master` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 36-40, error message about cross-contamination | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 2
- [ ] **Block git switch <branch> from worktree** | BLOCK (exit 2) when `git switch master` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 32-35, error message about cross-contamination | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 3
- [ ] **Allow git checkout -b from worktree** | PASS (exit 0) when `git checkout -b impl/123-foo origin/master` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 36-40 pass-through regex for `-[bB]` | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 4
- [ ] **Allow git switch -c from worktree** | PASS (exit 0) when `git switch -c impl/123-foo` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 32-35 pass-through regex for `-[cC]` | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 5
- [ ] **Allow git checkout -- <file>** | PASS (exit 0) when `git checkout -- file.rs` runs inside a linked worktree | `.claude/hooks/pre-tool-use.sh` lines 42-45 pass-through regex for `--` separator | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 6
- [ ] **Allow git checkout --ours** | PASS (exit 0) when `git checkout --ours Cargo.lock` runs inside a linked worktree (rebase conflict resolution) | `.claude/hooks/pre-tool-use.sh` lines 42-45 pass-through regex for `--ours` | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 7
- [ ] **Allow git checkout --theirs** | PASS (exit 0) when `git checkout --theirs Cargo.lock` runs inside a linked worktree (rebase conflict resolution) | `.claude/hooks/pre-tool-use.sh` lines 42-45 pass-through regex for `--theirs` | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test case 8
- [ ] **Unaffected operations (commit, push)** | PASS (exit 0) when `git commit -m wip` and `git push origin HEAD` run inside a linked worktree | `.claude/hooks/pre-tool-use.sh` worktree guard does not block these commands (only checkout/switch/worktree add) | `.claude/hooks/tests/test_pre_tool_use_worktree.sh` test cases 9-10
- [ ] **Main checkout no false positives** | PASS (exit 0) when all blocked commands run from main checkout (git_dir == common_dir detection skips guard) | `.claude/hooks/pre-tool-use.sh` detection `[ "$_git_dir" != "$_common_dir" ]` line 21 | verification via manual test from main checkout
- [ ] **Outside repo no false positives** | PASS (exit 0) when commands run outside any git repo (git_dir is empty, detection skips) | `.claude/hooks/pre-tool-use.sh` detection `[ -n "$_git_dir" ]` line 21 | verification via manual test outside repo

## Structural Assertions (Non-Grid)

- [ ] No duplicate hard-reset block added (already globally blocked on line 9 of existing hook — spec excludes this)
- [ ] Exit code is 2 (hook convention for "block with feedback"), not 1
- [ ] Detection logic: `git rev-parse --git-dir` vs `git rev-parse --git-common-dir` (clean worktree indicator)
- [ ] Inserted _before_ final `exit 0` on line 22, _after_ stash block ending at line 20

## Gates (Pre-Verify Checklist)

- `bash .claude/hooks/tests/test_pre_tool_use_worktree.sh` from inside a linked worktree passes all 10 cases
- `echo '{"tool_input":{"command":"git push --force origin master"}}' | bash .claude/hooks/pre-tool-use.sh && echo "exit: $?"` returns exit code 2 (existing global hard-reset block still fires)
- `echo '{"tool_input":{"command":"git stash"}}' | bash .claude/hooks/pre-tool-use.sh && echo "exit: $?"` returns exit code 2 (existing stash block still fires)

## Context

Refined per plan-reviewer wrapup comment: two implementation corrections were needed to avoid breaking the build pipeline.

1. `git reset --hard` is already globally blocked on line 9 — no worktree-scoped version needed (dead code risk).
2. `git checkout -b impl/...` and `git checkout --ours/--theirs` must pass through — builders and agents use these forms routinely from inside worktrees.

Detection mechanism validated live: `git_dir` vs `git-common-dir` has zero false positives on both main checkout (both are `.git`) and outside-repo (both empty).
