---
description: Compatibility shim for legacy bulk PR sweeps; prefer /swarm-driven builder/reviewer flow
argument-hint: "[--dry-run] [--filter <pattern>]"
disable-model-invocation: true
---

# Bulk PR Worktrees (Compatibility Shim)

Scan all agent worktrees for uncommitted changes and create PRs for each. Context: **$ARGUMENTS**

This command is a legacy/manual sweep entrypoint. The live swarm model prefers
one PR-shaped worktree per worker, with PR creation and review handled through
the `/swarm` control plane. Use `/bulk-pr` only when cleaning up older waves or
manual worktree batches.

## Steps

### 1. Scan worktrees
```bash
cd /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees
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
3. Runs `cargo fmt --all -- --check` (fix if needed)
4. Runs `cargo clippy --workspace --lib` (fix if needed)
5. Runs appropriate `cargo test` for changed crates
6. Creates a descriptive feature branch
7. Commits with conventional commit message
8. Pushes and creates draft PR via `gh pr create --draft`
9. Returns the PR URL

### 3. Report

Collect all PR URLs and report:

| Worktree | Branch | PR URL | Status |
|----------|--------|--------|--------|
| agent-xxx | fix/... | #1234 | Created |
| ... | | | |

### Tips
- Group small related changes (e.g., multiple `mod.rs` test additions) if they're logically related
- Skip worktrees where the diff is just 1-2 lines of uncommitted debugging artifacts
- Use `--dry-run` to preview without creating PRs
