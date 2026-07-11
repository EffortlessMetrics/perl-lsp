#!/usr/bin/env bash
# PreToolUse hook: block dangerous bash commands before execution
# Exit 2 = block with feedback
# Exit 0 = allow

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
# agent_type is the confirmed PreToolUse subagent field (captured live,
# 2026-07-11: a real diff-auditor persona's payload carried a populated
# agent_type="diff-auditor" with subagent_type null) -- it stays PRIMARY.
# subagent_type is read as a defensive fallback only, in case a future
# harness version or a different event shape (e.g. an Agent-Teams teammate
# session) populates that field instead. Absent both -> "main" (top-level
# orchestrator), matching the confirmed top-level payload shape.
AGENT_TYPE=$(echo "$INPUT" | jq -r '.agent_type // .subagent_type // "main"')

# Shared regex fragments (#3763 deep-review hardening, 2026-07-11): the
# publish-boundary and read-only-shell blocks below both match a git/gh
# subcommand immediately after the binary name. Two bypasses were found and
# fixed here for both blocks:
#   1. A bare `([[:space:]]|$)` terminator lets a shell separator ride
#      straight through: `git push;echo`, `git push&&x`, `(git push)` all
#      matched "push" followed by a non-whitespace, non-EOL character that
#      the old class didn't cover. CMD_TERMINATOR adds `; & | < >` (shell
#      separators, mirroring the `rm -rf` guard further down this file) and
#      `)` (a subshell-wrapped command still needs to be caught).
#   2. Requiring the subcommand immediately after `git`/`gh` lets global
#      options hide it: `git -C /repo push`, `git -c user.name=x push`,
#      `git --no-pager push`, `git --git-dir=... push`, and
#      `gh --repo owner/name pr merge 42` all bypassed. GIT_GLOBAL_OPT /
#      GH_GLOBAL_OPT tolerate a repeated run of the common global flags
#      between the binary and its subcommand.
GIT_GLOBAL_OPT='(-C[[:space:]]+[^[:space:]]+|-c[[:space:]]+[^[:space:]]+|--git-dir=[^[:space:]]+|--work-tree=[^[:space:]]+|--no-pager|--paginate|-p)'
GH_GLOBAL_OPT='(--repo[[:space:]]+[^[:space:]]+|-R[[:space:]]+[^[:space:]]+|--hostname[[:space:]]+[^[:space:]]+)'
CMD_TERMINATOR='([[:space:];&|<>)]|$)'

# M4b publication boundary (#3763, "publication-boundary moves DEFER -> BUILD"):
# the PreToolUse hook payload self-identifies the calling subagent via
# `agent_type` when Claude Code invokes a hook inside a subagent (absent at
# the top level, which is why the jq filter above defaults to "main"). This
# lets us deny direct, routine `git push` forms (including the tested
# global-option and shell-separator variants below) and `gh pr merge` for
# review/audit personas specifically, with no env var required.
#
# Named review/audit personas already carry a read-only `tools:` allowlist
# that excludes Edit/Write/Agent (`.claude/agents/*.md`, enforced by
# `cargo xtask check-agent-capabilities` / #3771) -- their only residual
# write surface is `Bash`. This block closes that surface for the direct
# publish forms; shell-indirection through `Bash` (see "Known, accepted
# limitations" below) remains a documented, accepted gap, not something
# this hook claims to close.
#
# REVIEW_AUDIT_AGENT_TYPES MUST stay in sync with
# `xtask/src/tasks/agent_capability_policy.rs`'s `REVIEW_AUDIT_AGENTS` (the
# same cohort already excluded from Edit/Write by #3771). Do not hand-edit
# one without the other --
# `.claude/hooks/tests/test_review_audit_agents_sync.sh` fails CI the
# moment the two lists disagree.
REVIEW_AUDIT_AGENT_TYPES="reviewer reviewer-deep diff-auditor maintainer-pr maintainer-issue architecture-reviewer advocatus-diaboli accuracy-scout research-verifier oppositional-planner plan-reviewer spec-test-code-match scout-find-ci-ops-gaps scout-find-dap-gaps scout-find-docs-receipt-drift scout-find-lsp-gaps scout-find-parser-gaps scout-find-robustness-gaps"

is_review_audit_agent() {
  case " $REVIEW_AUDIT_AGENT_TYPES " in
    *" $1 "*) return 0 ;;
    *) return 1 ;;
  esac
}

# Known, accepted limitations (this is a GUARDRAIL against the routine,
# unobfuscated publish forms a review/audit persona would actually type --
# NOT an adversarial sandbox, and not a claim that no shell can ever push or
# merge from here):
#   - Global-option tolerance above covers the common documented forms, not
#     every git/gh global flag that could ever precede the subcommand.
#   - Shell indirection always bypasses a regex guard: `sh -c "git push"`,
#     `eval "git push"`, decode-and-pipe (base64 | sh), or writing a script
#     to disk and executing it separately are NOT caught here -- closing
#     that class would require a real sandboxed shell, not a PreToolUse
#     regex. This is the same category of accepted gap the `rm -rf` guard
#     documents further down this file (quoted-path evasion there).
# Defense-in-depth: this hook is one layer alongside the tool-allowlist
# boundary (#3771, no Edit/Write/NotebookEdit/Agent for the review/audit
# cohort) -- it is not the only control.
if is_review_audit_agent "$AGENT_TYPE" && [ -n "$CMD" ]; then
  if echo "$CMD" | grep -qE "(^|[^[:alnum:]_])git([[:space:]]+${GIT_GLOBAL_OPT})*[[:space:]]+push${CMD_TERMINATOR}"; then
    echo "Blocked (publish boundary, #3763 M4b): agent_type=$AGENT_TYPE is a review/audit persona and may not 'git push'. Review/audit agents return findings; a writer, publisher, or the orchestrator performs the push." >&2
    exit 2
  fi
  if echo "$CMD" | grep -qE "(^|[^[:alnum:]_])gh([[:space:]]+${GH_GLOBAL_OPT})*[[:space:]]+pr[[:space:]]+merge${CMD_TERMINATOR}"; then
    echo "Blocked (publish boundary, #3763 M4b): agent_type=$AGENT_TYPE is a review/audit persona and may not 'gh pr merge'. Merging is an ops/orchestrator responsibility." >&2
    exit 2
  fi
fi

# M4b read-only shell (#3763): review/audit agents are mechanically read-only.
# Their tool allowlist already excludes Edit/Write/NotebookEdit/Agent (see
# `.claude/agents/*.md` `tools:` + `cargo xtask check-agent-capabilities`).
# This block closes the residual mutating-Bash vector so the boundary is a
# TRUE read-only shell: when the invoking agent is spawned with
# CLAUDE_AGENT_READONLY=1, mutating git/gh/filesystem commands are rejected
# BEFORE execution, while read-only inspection (git diff/log/show/status,
# gh pr view/diff/list/checks, cargo check, cat, grep) passes. Review/audit
# agents return findings to an orchestrator; a writer/publisher performs the
# writes. Enable by exporting CLAUDE_AGENT_READONLY=1 in the review-agent spawn.
if [ "${CLAUDE_AGENT_READONLY:-0}" = "1" ] && [ -n "$CMD" ]; then
  # Same terminator + global-option hardening as the publish-boundary block
  # above (CMD_TERMINATOR / GIT_GLOBAL_OPT / GH_GLOBAL_OPT) -- this is the
  # same one-liner shape (binary, immediate subcommand, weak terminator)
  # that had the same bypasses (`git -C /x commit`, `git push;echo`, etc.).
  if echo "$CMD" | grep -qE "(^|[^[:alnum:]_])git([[:space:]]+${GIT_GLOBAL_OPT})*[[:space:]]+(commit|push|merge|rebase|reset|revert|cherry-pick|am|apply|restore|stash|clean|add|rm|mv)${CMD_TERMINATOR}"; then
    echo "Blocked (read-only agent): mutating git command. Review/audit agents return findings; a writer/publisher commits. See #3763 (M4b)." >&2
    exit 2
  fi
  if echo "$CMD" | grep -qE '(^|[^[:alnum:]_])git[[:space:]]+(checkout[[:space:]]+(-[bB]|--)|switch[[:space:]]+-[cC]|branch[[:space:]]+(-[dDmMcC]|[^-[:space:]])|worktree[[:space:]]+add|tag[[:space:]]+[^-[:space:]])'; then
    echo "Blocked (read-only agent): mutating git command (branch/worktree/tag). See #3763 (M4b)." >&2
    exit 2
  fi
  if echo "$CMD" | grep -qE "(^|[^[:alnum:]_])gh([[:space:]]+${GH_GLOBAL_OPT})*[[:space:]]+(pr[[:space:]]+(merge|comment|review|edit|close|create|ready|lock|reopen)|issue[[:space:]]+(create|edit|comment|close|delete|lock|reopen|pin|transfer|develop)|label[[:space:]]+(create|edit|delete|clone)|release[[:space:]]+(create|edit|delete|upload)|secret|variable|cache[[:space:]]+delete|workflow[[:space:]]+(run|enable|disable)|run[[:space:]]+(rerun|cancel|delete))${CMD_TERMINATOR}"; then
    echo "Blocked (read-only agent): mutating gh command. Review/audit agents post no comments/labels; a publisher/ops does. See #3763 (M4b)." >&2
    exit 2
  fi
  if echo "$CMD" | grep -qE '(^|[^[:alnum:]_])gh[[:space:]]+.*--(add|remove)-label'; then
    echo "Blocked (read-only agent): gh label mutation. See #3763 (M4b)." >&2
    exit 2
  fi
  if echo "$CMD" | grep -qE '(^|[^[:alnum:]_])gh[[:space:]]+api[[:space:]]+.*(-X[[:space:]]*|--method[[:space:]]+)(POST|PATCH|PUT|DELETE)'; then
    echo "Blocked (read-only agent): gh api write request. See #3763 (M4b)." >&2
    exit 2
  fi
  # Filesystem writes: redirection (`>`, `>>` — but not `2>`/`2>&1`), and
  # mutating coreutils. Read-only agents do not modify the filesystem.
  if echo "$CMD" | grep -qE '>>|[^0-9&>]>[^&>]|(^|[^[:alnum:]_])(tee|dd|truncate|install)[[:space:]]|(^|[^[:alnum:]_])(cp|mv|touch|mkdir|rmdir|ln)[[:space:]]|(^|[^[:alnum:]_])sed[[:space:]]+-[^[:space:]]*i'; then
    echo "Blocked (read-only agent): file-writing command. Review/audit agents do not modify the filesystem. See #3763 (M4b)." >&2
    exit 2
  fi
fi

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
