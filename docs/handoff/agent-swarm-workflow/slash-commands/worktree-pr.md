---
description: PR a worktree's changes (validate, branch, commit, push, PR)
argument-hint: "<worktree-path> [commit message]"
---

# Worktree PR

Create a PR from a worktree's uncommitted changes. Context: **$ARGUMENTS**

## Steps

1. **Identify the worktree** -- parse $ARGUMENTS for the worktree path. If not provided, list available worktrees with changes:
```bash
cd $PROJECT_ROOT/.claude/worktrees && for d in agent-*; do changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1); [ -n "$changes" ] && echo "$d: $changes"; done
```

2. **Understand the changes** -- cd to the worktree and examine:
```bash
git diff --stat HEAD
git diff HEAD
```

3. **Validate** -- run checks in the worktree:
```bash
$FMT_CMD -- --check    # or equivalent dry-run mode
$LINT_CMD 2>&1 | tail -5
$TEST_CMD 2>&1 | tail -10
```
Fix any issues before proceeding.

4. **Branch** -- create a descriptive feature branch:
- `fix/...` for bug fixes
- `feat/...` for new features
- `test/...` for test additions
- `docs/...` for documentation
- `chore/...` for cleanup
```bash
git checkout -b <branch-name>
```

5. **Commit** -- stage relevant files and commit with conventional commit message:
```bash
git add <files>
git commit -m "$(cat <<'EOF'
<type>(<scope>): <description>
EOF
)"
```

6. **Push and PR**:
```bash
git push -u origin <branch-name>
gh pr create --title "<type>(<scope>): <description>" --body "$(cat <<'EOF'
## Summary
<what and why>

## Evidence
- `$TEST_CMD` -- passes
- `$LINT_CMD` -- clean
- `$FMT_CMD` -- clean
EOF
)"
```

7. **Return the PR URL**.
