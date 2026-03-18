#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/agent-dashboard.sh [--verbose] [--no-prs]

Render a compact dashboard for active agent worktrees under .claude/worktrees/.

Options:
  --verbose   Show commit lists and git status for each active worktree
  --no-prs    Skip GitHub PR lookup even if gh/jq are available
  -h, --help  Show this help text
USAGE
}

VERBOSE=0
FETCH_PRS=1

while (($# > 0)); do
  case "$1" in
    --verbose)
      VERBOSE=1
      shift
      ;;
    --no-prs)
      FETCH_PRS=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -d .git ]]; then
  echo "❌ Run this script from the repository root" >&2
  exit 1
fi

if [[ -t 1 ]]; then
  BOLD=$(printf '\033[1m')
  DIM=$(printf '\033[2m')
  RESET=$(printf '\033[0m')
else
  BOLD=''
  DIM=''
  RESET=''
fi

repeat_char() {
  local char="$1"
  local count="$2"
  local out=''
  for ((i = 0; i < count; i++)); do
    out+="$char"
  done
  printf '%s' "$out"
}

metric_bar() {
  local value="$1"
  local max="$2"
  local width="${3:-8}"
  local filled=0
  if (( max > 0 )); then
    filled=$(( value * width / max ))
  fi
  if (( filled > width )); then
    filled=$width
  fi
  local empty=$(( width - filled ))
  printf '%s%s' "$(repeat_char '█' "$filled")" "$(repeat_char '░' "$empty")"
}

safe_git() {
  git "$@" 2>/dev/null || true
}

current_branch=$(safe_git rev-parse --abbrev-ref HEAD)
if [[ -z "$current_branch" ]]; then
  current_branch="unknown"
fi

max_ahead=0
active_count=0
dirty_worktrees=0
total_commits_ahead=0
worktree_rows=()
verbose_sections=()

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  wt_path=$(awk '{print $1}' <<<"$line")
  [[ "$wt_path" != *"/.claude/worktrees/"* && "$wt_path" != .claude/worktrees/* ]] && continue

  branch=$(sed -n 's/.*\[\(.*\)\].*/\1/p' <<<"$line")
  if [[ -z "$branch" ]]; then
    branch=$(basename "$wt_path")
  fi

  head_ref="origin/master"
  if ! git -C "$wt_path" rev-parse --verify "$head_ref" >/dev/null 2>&1; then
    head_ref="master"
  fi
  if ! git -C "$wt_path" rev-parse --verify "$head_ref" >/dev/null 2>&1; then
    head_ref="HEAD"
  fi

  last_commit_time=$(safe_git -C "$wt_path" log -1 --format='%cr')
  last_commit_msg=$(safe_git -C "$wt_path" log -1 --format='%s')
  dirty_count=$(safe_git -C "$wt_path" status --porcelain | wc -l | tr -d ' ')
  ahead_count=$(safe_git -C "$wt_path" rev-list "$head_ref"..HEAD --count)

  [[ -z "$last_commit_time" ]] && last_commit_time="unknown"
  [[ -z "$last_commit_msg" ]] && last_commit_msg="no commits"
  [[ -z "$dirty_count" ]] && dirty_count=0
  [[ -z "$ahead_count" ]] && ahead_count=0

  if (( dirty_count > 0 )); then
    dirty_worktrees=$((dirty_worktrees + 1))
    dirty_badge="⚠ ${dirty_count}"
  else
    dirty_badge="✅ 0"
  fi

  activity_badge="🕒 ${last_commit_time}"
  ahead_badge="↑ ${ahead_count}"
  worktree_rows+=("$branch|$ahead_count|$dirty_count|$ahead_badge|$dirty_badge|$activity_badge|$last_commit_msg|$wt_path")

  active_count=$((active_count + 1))
  total_commits_ahead=$((total_commits_ahead + ahead_count))
  if (( ahead_count > max_ahead )); then
    max_ahead=$ahead_count
  fi

  if (( VERBOSE == 1 )); then
    commits=$(safe_git -C "$wt_path" log "$head_ref"..HEAD --oneline --decorate)
    status=$(safe_git -C "$wt_path" status --short)
    [[ -z "$commits" ]] && commits="  (no commits ahead)"
    [[ -z "$status" ]] && status="  (clean worktree)"
    verbose_sections+=("$branch|$wt_path|$commits|$status")
  fi
done < <(safe_git worktree list --porcelain | awk '/^worktree /{wt=$2} /^branch /{br=$2; print wt " [" br "]"} /^detached/{print wt " [detached]"}')

printf '%sAgent Dashboard%s — %s\n' "$BOLD" "$RESET" "$(date -u '+%Y-%m-%d %H:%M UTC')"
printf '%sRepository%s: %s (%s)\n' "$BOLD" "$RESET" "$(basename "$(pwd)")" "$current_branch"

if (( active_count == 0 )); then
  echo
  echo "No active agent worktrees found under .claude/worktrees/."
  exit 0
fi

echo
printf '%sSummary%s\n' "$BOLD" "$RESET"
printf '  • Active worktrees      %s\n' "$active_count"
printf '  • Dirty worktrees       %s\n' "$dirty_worktrees"
printf '  • Commits ahead total   %s\n' "$total_commits_ahead"
printf '  • Progress density      %s\n' "$(metric_bar "$total_commits_ahead" $(( max_ahead > 0 ? active_count * max_ahead : active_count )) 12)"

echo
printf '%sActive Worktrees%s\n' "$BOLD" "$RESET"
printf '%-32s %-14s %-10s %-18s %s\n' 'Branch' 'Ahead' 'Dirty' 'Last activity' 'Last commit'
printf '%-32s %-14s %-10s %-18s %s\n' '--------------------------------' '--------------' '----------' '------------------' '------------------------------'
for row in "${worktree_rows[@]}"; do
  IFS='|' read -r branch ahead_count dirty_count ahead_badge dirty_badge activity_badge last_commit_msg wt_path <<<"$row"
  ahead_visual=$(metric_bar "$ahead_count" "$max_ahead" 8)
  printf '%-32s %-14s %-10s %-18s %s\n' \
    "$branch" \
    "$ahead_badge $ahead_visual" \
    "$dirty_badge" \
    "$activity_badge" \
    "$last_commit_msg"
done

pr_count=0
pr_rows=()
if (( FETCH_PRS == 1 )) && command -v gh >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
  while IFS= read -r pr_line; do
    [[ -z "$pr_line" ]] && continue
    pr_rows+=("$pr_line")
    pr_count=$((pr_count + 1))
  done < <(
    gh pr list --state open --json number,title,headRefName,statusCheckRollup,isDraft --limit 100 2>/dev/null \
      | jq -r '.[] | [
          (.number|tostring),
          .headRefName,
          .title,
          (if .isDraft then "draft" else (((.statusCheckRollup // []) | map(.conclusion // .status // "pending") | map(select(. != ""))[0]) // "pending") end)
        ] | @tsv'
  )

  if (( pr_count > 0 )); then
    echo
    printf '%sOpen PRs%s\n' "$BOLD" "$RESET"
    printf '%-8s %-32s %-12s %s\n' 'PR' 'Branch' 'CI' 'Title'
    printf '%-8s %-32s %-12s %s\n' '--------' '--------------------------------' '------------' '------------------------------'
    for row in "${pr_rows[@]}"; do
      IFS=$'\t' read -r number branch title status <<<"$row"
      case "$status" in
        success) status_badge='✅ success' ;;
        failure|failed|error) status_badge='❌ failure' ;;
        draft) status_badge='📝 draft' ;;
        *) status_badge='⏳ pending' ;;
      esac
      printf '%-8s %-32s %-12s %s\n' "#$number" "$branch" "$status_badge" "$title"
    done
  fi
else
  echo
  printf '%sOpen PRs%s\n' "$BOLD" "$RESET"
  echo "  • Skipped PR lookup (install gh + jq, or omit --no-prs)"
fi

echo
printf '%sHighlights%s\n' "$BOLD" "$RESET"
most_ahead=$(printf '%s\n' "${worktree_rows[@]}" | sort -t'|' -k2,2nr | head -n1)
if [[ -n "$most_ahead" ]]; then
  IFS='|' read -r branch ahead_count _rest <<<"$most_ahead"
  printf '  • Most progress: %s (%s commits ahead)\n' "$branch" "$ahead_count"
fi
if (( dirty_worktrees > 0 )); then
  printf '  • Needs cleanup: %s worktree(s) have uncommitted changes\n' "$dirty_worktrees"
else
  echo '  • Hygiene: all agent worktrees are clean'
fi
if (( pr_count > 0 )); then
  printf '  • Review queue: %s open PR(s) tied to active work\n' "$pr_count"
fi

if (( VERBOSE == 1 )); then
  echo
  printf '%sVerbose Details%s\n' "$BOLD" "$RESET"
  for section in "${verbose_sections[@]}"; do
    IFS='|' read -r branch wt_path commits status <<<"$section"
    echo
    printf '%s%s%s\n' "$BOLD" "$branch" "$RESET"
    printf '%sPath%s: %s\n' "$DIM" "$RESET" "$wt_path"
    printf '%sCommits%s\n%s\n' "$DIM" "$RESET" "$commits"
    printf '%sStatus%s\n%s\n' "$DIM" "$RESET" "$status"
  done
fi
