#!/usr/bin/env bash
# PreToolUse hook: block dangerous bash commands before execution
# Exit 2 = block with feedback
# Exit 0 = allow

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if echo "$CMD" | grep -qE 'git push --force($|[[:space:]])|git push -f |git checkout \.|git reset --hard|cargo publish|git clean -fd'; then
  echo "Blocked: dangerous command '$CMD'. Use safer alternatives." >&2
  exit 2
fi

# Block recursive deletes that target the filesystem root or a whole top-level
# system directory. Subpath deletes (e.g. `rm -rf /tmp/foo`, `rm -rf /home/user/proj/target`)
# are allowed — the previous bare `rm -rf /` substring match blocked every absolute path.
# Matched: `rm -rf /`, `rm -rf /*`, `rm -rf /etc`, `rm -rf /home/`, `rm -rf /usr/*`,
#          `rm -rf /run`, `rm -rf /tmp`, `rm -rf /tmp/*` (whole shared-temp wipe removes
#          active build files/sockets in container sessions — per Codex review on PR #1952),
#          separator-terminated forms `rm -rf /etc;`, `rm -rf /etc&`, `rm -rf /etc||`,
#          and a dangerous dir at ANY argument position, e.g. `rm -rf /home/foo /etc`
#          (both per factory-droid review on PR #1952).
# Not matched: `rm -rf /tmp/x`, `rm -rf /home/user/...`, `rm -rf ./target`, `rm -rf /etc-backup`.
# The dir alternation enumerates the FHS / common-Linux top-levels worth protecting (incl.
# /run, /media, /libexec, /snap, /lost+found — added per factory-droid review on PR #1952).
# It is a curated allow-list, not byte-for-byte parity with the old substring match: the goal
# is to catch whole-directory wipes of system top-levels, not every string the old rule caught.
# The `([^ ]+ +)*` prefix lets any earlier whitespace-delimited argument be skipped so the
# dangerous dir is found wherever it appears; the trailing class (whitespace, EOL, or a
# shell separator ; & | < >) ensures only a *whole* dir is matched, not a deeper subpath.
#
# Known, accepted limitations (maintainer-acknowledged on PR #1952):
#   - Quoted dangerous paths are NOT matched: `rm -rf "/etc"` slips through, because the
#     hook does no shell-quote stripping. The old substring match missed this too.
#   - The unanchored `rm ` match can false-positive on command names ending in `rm`:
#     `git rm /etc` is blocked. Anchoring `^rm ` was rejected because it would lose
#     coverage for chained commands like `cmd && rm /etc`. A rare false-block is preferred
#     over a coverage hole for a safety guard.
#   - Subpath deletes under a system dir are intentionally allowed (e.g. `rm -rf /run/docker.sock`):
#     this guard is a coarse net against whole-dir/root wipes, not a per-file protector;
#     re-blocking subpaths would reintroduce the over-blocking this change fixes.
if echo "$CMD" | grep -qE 'rm +([^ ]+ +)*/(\*?|(bin|boot|dev|etc|home|lib|lib32|lib64|libexec|lost\+found|media|mnt|opt|proc|root|run|sbin|snap|srv|sys|tmp|usr|var)/?\*?)([[:space:];&|<>]|$)'; then
  echo "Blocked: refusing to recursively delete the filesystem root or a whole system directory." >&2
  echo "Deleting a specific subpath (e.g. /tmp/foo, ./target) is allowed." >&2
  exit 2
fi

# Block refspec force-push forms: `git push <remote> +branch`, `git push origin +refs/heads/main`,
# `git push <remote> +HEAD:branch`. The `+` prefix on a refspec bypasses non-fast-forward checks
# the same way --force does.
if echo "$CMD" | grep -qE 'git +push( +[^ ]+)*[[:space:]]\+[^[:space:]]'; then
  echo "Blocked: refspec '+<ref>' in git push forces non-fast-forward the same as --force." >&2
  echo "Remove the leading '+' or use a safer alternative." >&2
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
