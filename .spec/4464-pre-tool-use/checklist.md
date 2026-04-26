# Implementation Checklist — Pre-Tool-Use Hook Worktree Guard (#4464)

## Step 1: Create Test Script
**File**: `.claude/hooks/tests/test_pre_tool_use_worktree.sh`
**Action**: CREATE — new file with 10 test cases covering all block/pass scenarios
**Dependencies**: None (test script is self-contained)
**Verify**: `bash .claude/hooks/tests/test_pre_tool_use_worktree.sh` — must run from inside a linked worktree (any path under `.claude/worktrees/`)

```bash
# Key content: 10 test functions (assert_blocks, assert_passes)
# - assert_blocks "git worktree add ../foo master"
# - assert_blocks "git checkout master"
# - assert_blocks "git switch master"
# - assert_passes "git checkout -b impl/123-foo origin/master"
# - assert_passes "git switch -c impl/123-foo"
# - assert_passes "git checkout -- file.rs"
# - assert_passes "git checkout --ours Cargo.lock"
# - assert_passes "git checkout --theirs Cargo.lock"
# - assert_passes "git commit -m wip"
# - assert_passes "git push origin HEAD"
```

**Estimated lines**: ~60 lines

---

## Step 2: Edit Pre-Tool-Use Hook
**File**: `.claude/hooks/pre-tool-use.sh`
**Action**: ADD worktree guard block
**Dependencies**: None (hook is already structured to support additional guards)
**Insert location**: After line 20 (end of stash block), before line 22 (final `exit 0`)
**Verify**: `cargo test --lib` does not regress; existing guards (hard-reset, stash) still fire

Key regex blocks to add:

1. `git worktree add` block (always block)
2. `git switch <branch>` block with pass-through for `-[cC]` (branch creation)
3. `git checkout <branch>` block with pass-throughs for `-[bB]`, `--`, `--ours`, `--theirs`

Detection header:
```bash
_git_dir=$(git rev-parse --git-dir 2>/dev/null)
_common_dir=$(git rev-parse --git-common-dir 2>/dev/null)
if [ -n "$_git_dir" ] && [ "$_git_dir" != "$_common_dir" ]; then
  # Inside a linked worktree — apply guards
  ...
fi
```

**Estimated lines**: ~25 lines

---

## Step 3: Verify Existing Guards Still Work
**File**: None (verification only)
**Action**: Test that the hook's existing guards (hard-reset on line 9, stash block on lines 11-20) still fire correctly
**Dependencies**: Step 1 and 2 complete
**Verify**:
```bash
echo '{"tool_input":{"command":"git push --force origin master"}}' | bash .claude/hooks/pre-tool-use.sh
echo $?  # Must be 2 (blocked by hard-reset guard)

echo '{"tool_input":{"command":"git stash"}}' | bash .claude/hooks/pre-tool-use.sh
echo $?  # Must be 2 (blocked by stash guard)
```

**Estimated lines**: 0 (verification only)

---

## Step 4: Test from Inside a Linked Worktree
**File**: None (execution only)
**Action**: Run test script from inside a real linked worktree to verify all 10 cases
**Dependencies**: Steps 1-3 complete
**Verify**:
```bash
cd /h/Code/Rust/perl-lsp/.claude/worktrees/<any-agent>
bash /h/Code/Rust/perl-lsp/.claude/hooks/tests/test_pre_tool_use_worktree.sh
# Output: "=== Results: 10 passed, 0 failed ==="
```

**Estimated lines**: 0 (execution only)

---

## Step 5: Test from Main Checkout (Regression)
**File**: None (execution only)
**Action**: Run test script from main checkout (not inside a worktree) to confirm no false positives
**Dependencies**: Steps 1-4 complete
**Verify**:
```bash
cd /h/Code/Rust/perl-lsp
bash .claude/hooks/tests/test_pre_tool_use_worktree.sh
# Expect: All 10 tests pass (guard logic detects main checkout and skips filtering)
# Note: git_dir == common_dir, so [ "$_git_dir" != "$_common_dir" ] is false, skips guard
```

**Estimated lines**: 0 (execution only)

---

## Summary

| File | Action | Lines | Status |
|------|--------|-------|--------|
| `.claude/hooks/tests/test_pre_tool_use_worktree.sh` | CREATE | ~60 | Code |
| `.claude/hooks/pre-tool-use.sh` | ADD worktree guard | ~25 | Code |
| Verify existing guards | Test | 0 | Gate |
| Test from worktree | Test | 0 | Gate |
| Test from main checkout | Test | 0 | Gate |

**Compilation gates** (all at each step):
- No Rust code changes, so `cargo build/check` not applicable
- Hook is bash; validate syntax: `bash -n .claude/hooks/pre-tool-use.sh`
- Verify hook still runs and parses JSON correctly: existing guards test above
- All tests pass: Steps 4-5 complete

**Total scope**: ~85 lines across 2 files
**Builder next**: Implement Steps 1-2, execute Steps 3-5, commit and push.
