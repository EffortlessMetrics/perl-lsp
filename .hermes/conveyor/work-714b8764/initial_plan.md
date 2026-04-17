# Initial Plan — work-714b8764

## Issue
`worktree: .claude/ edits leak to main checkout, bypass worktree isolation (#4235)`

## Problem Statement

The issue as filed claims that `.claude/` edits made in a worktree "leak" to the main checkout, bypassing worktree isolation. The issue author hypothesizes `.claude/` is symlinked. However:

1. **`.claude/` is NOT a symlink** — investigation confirms it is a regular directory in worktrees
2. **The symptom described (main checkout shows old versions, `git show` shows correct) is NORMAL git worktree behavior** when branches differ
3. **There IS a real issue:** git stash entries are shared across all worktrees (43 entries found in preflight check), creating cross-contamination risk
4. **There MAY be a real issue:** if agents accidentally run with cwd = main repo root (preflight Check 4), writes would go to wrong location

## Approach

The approach is to address the **actual root causes**, not the misleading symlink hypothesis, because the symlink hypothesis was disproven by direct investigation. The approach focuses on:

### 1. Clear Git Stash Contamination (Real Issue)

Git stash is shared across all worktrees via the common `.git` directory. Any stash entries risk cross-worktree contamination.

**Tasks:**
- Run `git stash clear` to clear the 43 existing stash entries
- Add a pre-commit hook or preflight enhancement to warn if stash has entries before agent work

### 2. Enhance Preflight Reliability (Preventative)

Preflight Check 4 (cwd isolation) already exists but may not run reliably before agents start work.

**Tasks:**
- Document that preflight must pass before any agent makes edits
- Consider adding preflight exit code 4 as a hard blocker in agent spawn logic
- Verify that the conveyor system runs preflight before spawning edit-capable agents

### 3. Document Expected Worktree Behavior (Education)

The issue author appears to have misinterpreted normal git behavior.

**Tasks:**
- Add documentation to `agent-preflight.md` clarifying that:
  - Main checkout NOT seeing worktree changes is NORMAL, not a bug
  - Only `git fetch` + `git checkout` / merge would bring worktree changes to main checkout
  - Worktree isolation means git-dir is separate; it does NOT mean main checkout reflects worktree state

### 4. Investigate Agent Spawn Logic (If Needed)

If the issue persists after steps 1-3, the conveyor/agent-spawn system may have a bug where agents are incorrectly given main repo path instead of worktree path.

**Tasks:**
- Audit how `worktree_path` is passed to agents during spawn
- Verify that `isolation: worktree` in agent definitions correctly sets the agent's working directory
- Check if there's a path resolution bug in the Hermes/conveyor system

## Risks

### Risk 1: Misdiagnosis
The issue title and description are contradictory and may indicate a misunderstanding rather than a real bug. If we fix the wrong thing, the issue will persist.

**Mitigation:** Focus on clearing stash contamination and ensuring preflight reliability first. These are real issues regardless of whether the "symlink leak" hypothesis is correct.

### Risk 2: Breaking Agent Productivity
If we make preflight too strict or add blocking checks, agents may be unable to work when they need to.

**Mitigation:** Keep preflight as advisory checks that can be bypassed with force flags if needed.

### Risk 3: Stash Clear Loses Work
Clearing 43 stash entries could lose work-in-progress if any contain valuable changes.

**Mitigation:** Before clearing, verify with `git stash list` that none of the entries are from active in-progress work.

## Task Breakdown

### Phase 1: Immediate Cleanup (Verification Agent can verify)
1. Run `git stash list` to inspect the 43 entries before clearing
2. If entries are not needed: `git stash clear`
3. Document findings in the issue

### Phase 2: Preflight Enhancement (Implementation)
4. Add warning in preflight output about stash risk
5. Document that preflight Check 4 (cwd) must pass before edits

### Phase 3: Documentation (Verification Agent)
6. Add clarifying section to `agent-preflight.md` about expected worktree behavior
7. Close issue as "working as designed" if investigation confirms no actual bug

### Phase 4: Agent Spawn Audit (If Needed)
8. Audit conveyor/agent-spawn to verify worktree path is correctly passed
9. Fix any path resolution bugs if found

## Verification Criteria

1. `git stash list` shows 0 entries after Phase 1
2. `bash scripts/agent-preflight.sh` exits 0 in an active worktree
3. Documentation clarifies expected vs. unexpected worktree behavior
4. No agent reports ".claude/ edits appearing in wrong location" after fix
