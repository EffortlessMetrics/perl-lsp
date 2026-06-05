#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
IMPL="${REPO_ROOT}/scripts/cleanup-completed-worktrees.sh"

PASS=0
FAIL=0
TMPDIR_BASE=""

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" && -d "${TMPDIR_BASE}" ]]; then
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

pass() {
  printf 'PASS %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL %s\n' "$1"
  FAIL=$((FAIL + 1))
}

assert_contains() {
  local label="$1"
  local haystack="$2"
  local needle="$3"

  if grep -qF "$needle" <<<"$haystack"; then
    pass "$label"
  else
    fail "$label"
    printf 'missing: %s\noutput:\n%s\n' "$needle" "$haystack"
  fi
}

assert_not_contains() {
  local label="$1"
  local haystack="$2"
  local needle="$3"

  if grep -qF "$needle" <<<"$haystack"; then
    fail "$label"
    printf 'unexpected: %s\noutput:\n%s\n' "$needle" "$haystack"
  else
    pass "$label"
  fi
}

write_mock_git() {
  local mock_dir="$1"
  cat > "${mock_dir}/git" <<'MOCK_GIT'
#!/usr/bin/env bash
set -euo pipefail

printf 'git %s\n' "$*" >> "${MOCK_STATE}/git.log"

if [[ "${1:-}" == "-C" ]]; then
  shift
  _worktree_path="${1:-}"
  shift || true
  if [[ "${1:-}" == "status" ]]; then
    if [[ -f "${MOCK_STATE}/dirty" ]]; then
      cat "${MOCK_STATE}/dirty"
    fi
    exit 0
  fi
fi

case "${1:-}" in
  rev-parse)
    if [[ "$*" == *"--path-format=absolute --git-common-dir"* ]]; then
      printf '%s/.git\n' "${MOCK_REPO_ROOT}"
      exit 0
    fi
    if [[ "$*" == *"--abbrev-ref HEAD"* ]]; then
      cat "${MOCK_STATE}/current-branch"
      exit 0
    fi
    ;;
  worktree)
    case "${2:-}" in
      prune)
        exit 0
        ;;
      list)
        cat "${MOCK_STATE}/worktree-list"
        exit 0
        ;;
      remove)
        printf 'DESTRUCTIVE git worktree remove %s\n' "$*" >> "${MOCK_STATE}/destructive.log"
        exit 0
        ;;
    esac
    ;;
  branch)
    if [[ "${2:-}" == "--merged" ]]; then
      printf '%s\n' "${3:-}" >> "${MOCK_STATE}/merged-queries.log"
      cat "${MOCK_STATE}/merged-branches"
      exit 0
    fi
    if [[ "${2:-}" == "-D" ]]; then
      printf 'DESTRUCTIVE git branch -D %s\n' "${3:-}" >> "${MOCK_STATE}/destructive.log"
      exit 0
    fi
    ;;
  config)
    if [[ "${2:-}" == "--get" ]]; then
      if [[ -s "${MOCK_STATE}/remote-merge" ]]; then
        cat "${MOCK_STATE}/remote-merge"
        exit 0
      fi
      exit 1
    fi
    ;;
  rev-list)
    if [[ -s "${MOCK_STATE}/ahead-count" ]]; then
      cat "${MOCK_STATE}/ahead-count"
    else
      printf '0\n'
    fi
    exit 0
    ;;
esac

printf 'unexpected git command: %s\n' "$*" >&2
exit 97
MOCK_GIT
  chmod +x "${mock_dir}/git"
}

write_mock_gh() {
  local mock_dir="$1"
  cat > "${mock_dir}/gh" <<'MOCK_GH'
#!/usr/bin/env bash
set -euo pipefail

printf 'gh %s\n' "$*" >> "${MOCK_STATE}/gh.log"

if [[ "$*" == pr\ list* ]]; then
  if [[ -s "${MOCK_STATE}/pr-number" ]]; then
    cat "${MOCK_STATE}/pr-number"
  fi
  exit 0
fi

printf 'unexpected gh command: %s\n' "$*" >&2
exit 98
MOCK_GH
  chmod +x "${mock_dir}/gh"
}

new_case() {
  local name="$1"
  local case_dir="${TMPDIR_BASE}/${name}"
  mkdir -p "${case_dir}/bin" "${case_dir}/repo/.git" "${case_dir}/repo/.claude/worktrees"
  : > "${case_dir}/merged-branches"
  : > "${case_dir}/remote-merge"
  : > "${case_dir}/ahead-count"
  : > "${case_dir}/pr-number"
  : > "${case_dir}/destructive.log"
  : > "${case_dir}/git.log"
  : > "${case_dir}/gh.log"
  printf 'main\n' > "${case_dir}/current-branch"
  write_mock_git "${case_dir}/bin"
  write_mock_gh "${case_dir}/bin"
  printf '%s\n' "$case_dir"
}

write_worktree_list() {
  local case_dir="$1"
  local branch="$2"
  local worktree_path="${case_dir}/repo/.claude/worktrees/${branch//\//-}"
  mkdir -p "$worktree_path"
  printf '%s abc123 [%s]\n' "$worktree_path" "$branch" > "${case_dir}/worktree-list"
}

run_cleanup_dry_run() {
  local case_dir="$1"
  PATH="${case_dir}/bin:${PATH}" \
    MOCK_STATE="$case_dir" \
    MOCK_REPO_ROOT="${case_dir}/repo" \
    bash "$IMPL" --dry-run 2>&1
}

assert_no_destructive_commands() {
  local label="$1"
  local case_dir="$2"

  if [[ -s "${case_dir}/destructive.log" ]]; then
    fail "$label"
    cat "${case_dir}/destructive.log"
  else
    pass "$label"
  fi
}

test_merged_branch_uses_main_by_default() {
  local case_dir output
  case_dir="$(new_case merged-main)"
  write_worktree_list "$case_dir" "feature/merged"
  printf '  feature/merged\n' > "${case_dir}/merged-branches"

  output="$(run_cleanup_dry_run "$case_dir")"

  assert_contains "dry-run announces main as the default cleanup base" "$output" "Base branch: main"
  assert_contains "merged branch is marked for removal in dry-run" "$output" "feature/merged"
  assert_contains "merged branch removal is reported" "$output" "merged"
  assert_contains "merged branch action is REMOVE" "$output" "REMOVE"
  assert_contains "merged query uses main, not legacy master" "$(cat "${case_dir}/merged-queries.log")" "main"
  assert_no_destructive_commands "dry-run does not remove merged worktrees" "$case_dir"
}

test_open_pr_branch_is_kept() {
  local case_dir output
  case_dir="$(new_case open-pr)"
  write_worktree_list "$case_dir" "feature/open-pr"
  printf '123\n' > "${case_dir}/pr-number"

  output="$(run_cleanup_dry_run "$case_dir")"

  assert_contains "open PR branch is kept" "$output" "open-pr:#123"
  assert_contains "open PR action is KEEP" "$output" "KEEP"
  assert_no_destructive_commands "dry-run does not remove open PR worktrees" "$case_dir"
}

test_unpushed_branch_is_kept() {
  local case_dir output
  case_dir="$(new_case unpushed)"
  write_worktree_list "$case_dir" "feature/unpushed"
  printf '2\n' > "${case_dir}/ahead-count"

  output="$(run_cleanup_dry_run "$case_dir")"

  assert_contains "unpushed branch is kept" "$output" "unpushed"
  assert_contains "unpushed action is KEEP" "$output" "KEEP"
  assert_no_destructive_commands "dry-run does not remove unpushed worktrees" "$case_dir"
}

test_abandoned_branch_is_reported_without_removal_in_dry_run() {
  local case_dir output
  case_dir="$(new_case abandoned)"
  write_worktree_list "$case_dir" "feature/abandoned"
  printf '0\n' > "${case_dir}/ahead-count"

  output="$(run_cleanup_dry_run "$case_dir")"

  assert_contains "abandoned branch is reported" "$output" "abandoned"
  assert_contains "abandoned action is REMOVE" "$output" "REMOVE"
  assert_not_contains "abandoned dry-run does not claim actual deletion" "$output" "git branch -D"
  assert_no_destructive_commands "dry-run does not remove abandoned worktrees" "$case_dir"
}

echo "=== cleanup-completed-worktrees dry-run test suite ==="
echo ""

if [[ ! -f "$IMPL" ]]; then
  echo "ERROR: cleanup-completed-worktrees.sh not found at ${IMPL}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"

test_merged_branch_uses_main_by_default
test_open_pr_branch_is_kept
test_unpushed_branch_is_kept
test_abandoned_branch_is_reported_without_removal_in_dry_run

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
