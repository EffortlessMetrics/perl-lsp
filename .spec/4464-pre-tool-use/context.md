# Context — Pre-Tool-Use Hook Worktree Guard (#4464)

## Problem Statement

Agents that work from inside git linked worktrees (worktree isolation is mandatory per CLAUDE.md) can accidentally switch branches or perform other operations that contaminate the main checkout's state. The pre-tool-use hook is the enforcement mechanism to prevent this.

**Root cause** (confirmed by plan-reviewer): The hook lacks a worktree-specific guard. Dangerous operations like `git checkout <branch>` succeed from inside a worktree and silently corrupt the main checkout.

**Detection mechanism** (validated live): `git rev-parse --git-dir` (worktree-specific: `.git/worktrees/<name>`) vs `git rev-parse --git-common-dir` (always main: `.git`). They differ only inside linked worktrees.

## Key Decisions

### 1. Which Operations to Block

**Blocked** (always exit 2):
- `git worktree add` — must originate from main checkout, never from inside a worktree
- `git checkout <branch>` — switching branches from inside a worktree corrupts the main checkout
- `git switch <branch>` — same risk as checkout

**Pass-through** (exit 0, allow):
- `git checkout -b/-B <branch>` — branch _creation_ is safe and necessary (builders create impl branches from inside worktrees)
- `git switch -c/-C <branch>` — branch _creation_ is safe
- `git checkout -- <file>` — file restore via `--` separator is safe (canonical form per `git restore` design)
- `git checkout --ours <file>` — rebase conflict resolution is safe (builders use this in rebase-pr.md workflows)
- `git checkout --theirs <file>` — rebase conflict resolution is safe
- All other git commands (commit, push, rebase, etc.) — unaffected by this guard

### 2. Why Not Block `git checkout` Entirely

The naive approach would be `[ checkin command starts with 'git checkout' ] && block`. This breaks builders:
- Spec-planner, red-tdd, green-tdd, builder, all agents run `git checkout -b impl/<issue#>-<slug> origin/master` from inside worktrees to create new branches
- Blocking this would prevent the entire build pipeline from functioning

**Solution**: The regex check `! echo "$CMD" | grep -qE '^git checkout -[bB]'` allows branch creation (`-b`/`-B` flags) while blocking branch switching (bare branch names).

### 3. Exit Code: 2, Not 1

The existing hook uses exit code 2 for "block with feedback" (Claude Code's semantic for "user saw an error message"). The issue body said exit 1, but the plan-reviewer corrected this to 2 to match hook convention.

### 4. Why Not Add a Second Hard-Reset Block

The issue body suggested adding `git reset --hard` to the worktree guard. However, line 9 of the existing hook already blocks it globally:
```bash
if echo "$CMD" | grep -qE '^git reset --hard'; then
  exit 2
fi
```

Adding a second block inside the worktree guard would create dead code. The global block fires first. The spec omits it.

## Alternatives Considered and Rejected

| Alternative | Why rejected |
|-------------|-------------|
| Worktree stash prevention instead of branch blocking | Stash is already globally blocked (line 11-20); worktree stash contamination is orthogonal to branch switching |
| Allow `git checkout <branch>` but warn loudly | Warnings are easily ignored; the risk is too high; block is correct |
| Detect worktree via `GIT_WORKTREE_LOCK` env var | Not reliable; git sets it inconsistently. `git_dir` vs `git-common-dir` is the canonical stable method |
| Run guards only when specific env vars are set | Makes the hook unreliable; guards should be uniform across all runs |

## Testing Strategy

The test script (`test_pre_tool_use_worktree.sh`) must run from inside a real linked worktree because:
- `git rev-parse --git-dir` returns different values depending on CWD
- Mocking the values in unit tests is fragile
- A real worktree is created by the agent spawning system and is available during the builder's run

The fallback: if the builder cannot easily run from inside a worktree during development, they can manually test by:
1. Creating a temporary linked worktree: `git worktree add ../temp-test master`
2. Entering it: `cd ../temp-test`
3. Running the test script
4. Cleanup: `cd .. && git worktree remove temp-test`

## Verification Path

1. `bash -n .claude/hooks/pre-tool-use.sh` — check syntax
2. `bash .claude/hooks/tests/test_pre_tool_use_worktree.sh` — run all 10 cases from inside a worktree
3. Existing guards still fire (hard-reset, stash) — regression check
4. Main checkout (git_dir == common_dir) — no false positives
5. Outside repo (both git_dir and git-common-dir empty) — no false positives

## Scope Exclusions

- Does NOT change the global hard-reset or stash guards (already working, plan-reviewer ruled them out)
- Does NOT add `lower_is_better` detection or tolerance bands (different issue, #4105)
- Does NOT modify the JSON parsing or event dispatch logic (hook's core is unchanged)
- Does NOT add new commands to the blocked list (only what's necessary to prevent main-checkout corruption)

## Downstream Impact

This hook fires on every command issued by every agent (via `.claude/hooks/pre-tool-use.sh` being wired to agent-definition CLaude Code runtime). Changes are low-risk because:
- Only adds new guard blocks (pass-through is the default)
- New rules are narrowly scoped to worktree operations
- All builders and agents already use `git checkout -b` (will continue to work)
- All builders and agents never use blocked commands from worktrees (command blocker adds safety)

Risk vectors:
- If regex patterns are overly broad, they'll block legitimate commands (mitigated by 10-case test)
- If detection logic is faulty, false positives on main checkout (mitigated by acceptance test case 9)
- If false negatives occur, corruption isn't prevented (mitigated by live validation in plan-review)

## Timing and Dependencies

- Standalone issue; no upstream dependencies
- Affects all downstream agents (spec-planner, red-tdd, builder, etc.)
- Can land independently
- Should land before any major build waves to prevent worker-node branch corruption

## Related Issues

- #4456 (discovery of the worktree stash contamination pattern that motivated this)
- #4077 (merged, reason for method_inheritance regression guard in #4464 context)
- Workflow: `.claude/hooks/pre-tool-use.sh` (hook source)
- Policy: CLAUDE.md worktree stash prohibition + git safety constraints

## Open Questions Resolved

- Q: Should `git checkout -- <file>` be allowed?
  A: Yes. It's the canonical form for file restore (vs the risky `git checkout <branch>`). Builders use it in rebase workflows.

- Q: Should exit code be 1 or 2?
  A: 2. Matches existing hook convention for "block with feedback."

- Q: Should hard-reset be blocked from worktrees too?
  A: No. It's already globally blocked. Double-blocking creates dead code.

- Q: Can the test run on CI, or must it run locally?
  A: Must run locally (from inside a real worktree). CI worktrees are ephemeral and test-scoped; the test verifies behavior inside a real agent worktree.
