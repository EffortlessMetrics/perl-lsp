# Research Findings — work-714b8764

## Issue Summary
The issue claims `.claude/` edits made in a worktree "leak" to the main checkout via symlinks. Investigation shows `.claude/` is NOT a symlink — it's a regular directory. The described symptom (main checkout shows old versions while `git show` shows correct) is normal git worktree behavior when branches differ. However, a REAL issue exists: git stash (43 entries found) is shared across all worktrees, creating cross-contamination risk.

## Relevant Codebase Areas
- `scripts/agent-preflight.sh` — pre-flight checks for worktree isolation (branch, cwd, stash)
- `scripts/worktree-manager.py` — worktree lifecycle management
- `.claude/commands/agent-preflight.md` — documentation for preflight checks
- `.claude/commands/control-plane-lock.md` — advisory lock for `.claude/` files
- `pre-tool-use.sh` hook — blocks `git stash` at execution time

## Key Findings
1. **`.claude/` is NOT a symlink** — the symlink hypothesis is not confirmed
2. **Worktree isolation (git-dir check) works correctly** — preflight Check 2 passes
3. **cwd isolation (Check 4) exists and works** — detects if agent runs from main repo root
4. **Git stash IS shared across worktrees** — 43 stash entries found, this is real contamination risk
5. **The issue description has contradictions** — "edits appear in main checkout" vs "reading shows pre-change versions" are mutually exclusive

## Proposed Approach
Address the REAL issues: (1) clear git stash contamination, (2) ensure preflight runs reliably before agents edit, (3) document that main checkout not reflecting worktree changes is NORMAL behavior, not a bug. Do NOT change worktree creation (not a symlink issue).

## Top Risks
1. **Misdiagnosis** — issue title/description are contradictory; may be misunderstanding rather than bug
2. **Stash clear loses work** — 43 entries exist; need to verify before clearing
3. **Agent cwd misconfiguration** — if agents spawn with wrong cwd, writes go to main checkout

## Scope
- Covers: git stash contamination, preflight reliability, documentation of expected behavior
- Does NOT cover: changing worktree creation (no symlink found), changing git worktree mechanics
