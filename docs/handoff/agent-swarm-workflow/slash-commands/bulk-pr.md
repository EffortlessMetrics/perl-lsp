---
description: PR all worktrees with uncommitted changes
argument-hint: "[--dry-run] [--filter <pattern>]"
---

# Bulk PR Worktrees

Scan all agent worktrees for uncommitted changes and create PRs for each. Context: **$ARGUMENTS**

## Steps

### 1. Scan worktrees
```bash
cd $PROJECT_ROOT/.claude/worktrees
for d in agent-*; do
  changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1)
  if [ -n "$changes" ]; then
    files=$(cd "$d" && git diff --name-only HEAD 2>/dev/null | tr '\n' ', ')
    echo "READY: $d | $changes | $files"
  fi
done
```

### 2. For each worktree with changes

Launch a parallel Agent (with `run_in_background: true`) for each worktree that:

1. cd to the worktree
2. Examines `git diff HEAD` to understand the changes
3. Runs `$FMT_CMD -- --check` (fix if needed)
4. Runs `$LINT_CMD` (fix if needed)
5. Runs `$TEST_CMD` for changed modules
6. Creates a descriptive feature branch
7. Commits with conventional commit message
8. Pushes and creates PR via `gh pr create`
9. Returns the PR URL

### 3. Report

Collect all PR URLs and report:

| Worktree | Branch | PR URL | Status |
|----------|--------|--------|--------|
| agent-xxx | fix/... | #1234 | Created |
| ... | | | |

### Tips
- Group small related changes (e.g., multiple test additions in the same module) if they are logically related
- Skip worktrees where the diff is just 1-2 lines of uncommitted debugging artifacts
- Use `--dry-run` to preview without creating PRs
- If `--filter <pattern>` is provided, only process worktrees whose names match
