#!/usr/bin/env bash
# PreToolUse hook: block dangerous bash commands before execution
# Exit 2 = block with feedback
# Exit 0 = allow

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if echo "$CMD" | grep -qE 'git push --force|git push -f |git checkout \.|git reset --hard|rm -rf /|cargo publish|git clean -fd'; then
  echo "Blocked: dangerous command '$CMD'. Use safer alternatives." >&2
  exit 2
fi

# Block git stash commands -- stash is shared across all worktrees and causes cross-contamination.
# Use git restore <file> to discard changes, or git commit -m wip to save work in progress.
if echo "$CMD" | grep -qE 'git stash( |$)'; then
  echo "Blocked: git stash is shared across all worktrees and risks cross-contamination." >&2
  echo "Use git restore <file> to discard changes or git commit -m wip to save work." >&2
  exit 2
fi

# Worktree guard (#4464): when CWD is inside a linked worktree, block branch-mutating
# commands that anchor to the wrong location and cause nested-worktree contamination.
# Detection: git-dir != git-common-dir means we're inside a linked worktree, not main.
git_dir=$(git rev-parse --git-dir 2>/dev/null)
common_dir=$(git rev-parse --git-common-dir 2>/dev/null)
if [ -n "$git_dir" ] && [ -n "$common_dir" ] && [ "$git_dir" != "$common_dir" ] && [ "$git_dir" != ".git" ]; then
  # In a linked worktree. Block the subset of git ops that anchor wrong.

  # git worktree add: always block (creates nested worktrees)
  if echo "$CMD" | grep -qE 'git +worktree +add( |$)'; then
    echo "Blocked: 'git worktree add' inside a linked worktree creates nested worktrees." >&2
    echo "Recovery: cd to main checkout first, then re-run. Main checkout: $(git rev-parse --git-common-dir)/.." >&2
    echo "See #4456 for context." >&2
    exit 2
  fi

  # git switch <branch> (without -c/-C): block branch-switch form
  if echo "$CMD" | grep -qE 'git +switch( +-[-[:alnum:]]+)* +[^-]'; then
    # Allow -c/-C (create) forms
    if ! echo "$CMD" | grep -qE 'git +switch +(-c|-C|--create|--force-create)( |$)'; then
      echo "Blocked: 'git switch <branch>' inside a linked worktree changes the worktree's branch." >&2
      echo "Recovery: cd to main checkout first, or use 'git switch -c <new-branch>' to create from current." >&2
      echo "See #4456 for context." >&2
      exit 2
    fi
  fi

  # git checkout <branch>: block branch-switch form (not -b/-B, not -- <file>, not --ours/--theirs)
  if echo "$CMD" | grep -qE 'git +checkout( |$)'; then
    # Allow safe forms:
    #   git checkout -b / -B / --force        → create-and-switch
    #   git checkout -- <path>                → restore file
    #   git checkout --ours / --theirs        → rebase conflict
    #   git checkout HEAD -- <path>           → restore from HEAD
    if ! echo "$CMD" | grep -qE 'git +checkout +(-b|-B|--force|-f |--ours|--theirs|--detach|-- +|[a-f0-9]+ +-- +|HEAD +-- +)'; then
      echo "Blocked: 'git checkout <branch>' inside a linked worktree changes the worktree's branch." >&2
      echo "Recovery: cd to main checkout first. Allowed here: 'git checkout -b/-B <new>', 'git checkout -- <file>', 'git checkout --ours/--theirs'." >&2
      echo "See #4456 for context." >&2
      exit 2
    fi
  fi
fi

exit 0
