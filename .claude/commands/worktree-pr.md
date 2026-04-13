---
description: PR a worktree's changes (validate → branch → commit → push → PR)
argument-hint: "<worktree-path> [commit message]"
---

# Worktree PR

Create a PR from a worktree's uncommitted changes. Context: **$ARGUMENTS**

## Steps

1. **Identify the worktree** — parse $ARGUMENTS for the worktree path. If not provided, list available worktrees with changes:
```bash
cd /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees && for d in agent-*; do changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1); [ -n "$changes" ] && echo "$d: $changes"; done
```

2. **Understand the changes** — cd to the worktree and examine:
```bash
git diff --stat HEAD
git diff HEAD
```

3. **Validate** — run checks in the worktree:
```bash
cargo xtask fmt --check
cargo clippy --workspace --lib 2>&1 | tail -5
cargo test --workspace --lib 2>&1 | tail -10
```
Fix any issues before proceeding.

4. **Branch** — create a descriptive feature branch:
- `fix/...` for parser fixes
- `test/...` for test additions
- `docs/...` for documentation
- `chore/...` for cleanup
```bash
git checkout -b <branch-name>
```

5. **Commit hygiene check** — verify only intended files are staged:
```bash
git diff --cached --name-only
```
**NEVER** use `git add -A` or `git add .`. Always add specific files.

Reject any of these from the staged set (unless they are the point of the PR):
- `Cargo.lock` — unless your change modifies dependencies; worktree drift causes false conflicts
- `.claude/` infrastructure files
- `docs/project/CURRENT_STATUS.md` — auto-generated
- `scripts/.ignored-baseline` — auto-generated

If unintended files are staged, unstage them:
```bash
git reset HEAD <file>
```

6. **Commit** — stage relevant files and commit with conventional commit message:
```bash
git add <specific-files>
git commit -m "$(cat <<'EOF'
<type>(<scope>): <description>
EOF
)"
```

6. **Push and PR** (always as draft):
```bash
git push -u origin <branch-name>
gh pr create --draft --title "<type>(<scope>): <description>" --body "$(cat <<'EOF'
## Summary
<what and why>

## Evidence
- `cargo test` — passes
- `cargo clippy` — clean
- `cargo fmt` — clean
EOF
)"
```

> **Note**: PRs open as draft. After review agent fixes issues, mark ready with `/pr-ready`.

7. **Return the PR URL**.
