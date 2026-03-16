#!/usr/bin/env bash
# Safe git pull that handles untracked file conflicts and stale branch tracking.
#
# Problem: `git pull` fails when the remote adds files that already exist locally
# as untracked files (common with generated files, worktree leftovers, etc.).
# Additionally, `@{u}` (upstream tracking ref) can be stale or missing, causing
# scripts that rely on it to fail silently.
#
# Usage:
#   scripts/safe-pull.sh              # pull from origin/master
#   scripts/safe-pull.sh my-branch    # pull from origin/my-branch
set -euo pipefail

BRANCH="${1:-master}"
REMOTE="origin"

# Ensure we are inside a git repository
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "ERROR: Not inside a git repository."
  exit 1
fi

echo "==> Fetching ${REMOTE}..."
if ! git fetch "${REMOTE}" "${BRANCH}"; then
  echo "ERROR: Failed to fetch ${REMOTE}/${BRANCH} (network issue or remote not configured)."
  exit 1
fi

# Use explicit remote ref instead of @{u} to avoid stale tracking issues
LOCAL_HEAD=$(git rev-parse HEAD)
REMOTE_HEAD=$(git rev-parse "${REMOTE}/${BRANCH}")

if [ "${LOCAL_HEAD}" = "${REMOTE_HEAD}" ]; then
  echo "Already up to date."
  exit 0
fi

# Show what would change
BEHIND=$(git rev-list HEAD.."${REMOTE}/${BRANCH}" --count)
echo "==> ${BEHIND} commit(s) behind ${REMOTE}/${BRANCH}"

# Try a merge, capturing both stdout and stderr in a single attempt.
# IMPORTANT: We do NOT attempt the merge twice. The first failed merge can leave
# git in a merge-in-progress state (for content conflicts), so re-running merge
# would get "You have not concluded your current merge" instead of the real error.
MERGE_OUTPUT=$(git merge "${REMOTE}/${BRANCH}" 2>&1) && {
  echo "Pull succeeded."
  exit 0
}

# Merge failed — check what kind of failure
if echo "${MERGE_OUTPUT}" | grep -q "would be overwritten by merge"; then
  # Untracked file conflicts: git refused to start the merge, so no cleanup needed.
  # Extract conflicting file paths from the error message.
  # Git formats these as indented file paths between the error header and footer.
  CONFLICTING_FILES=$(echo "${MERGE_OUTPUT}" \
    | grep -E '^\t' \
    | sed 's/^\t//')

  if [ -z "${CONFLICTING_FILES}" ]; then
    echo "ERROR: Detected untracked file conflict but could not parse file list."
    echo "Raw output:"
    echo "${MERGE_OUTPUT}"
    exit 1
  fi

  echo "==> Removing conflicting untracked files:"
  while IFS= read -r f; do
    if [ -n "${f}" ]; then
      echo "  rm ${f}"
      rm -f "${f}"
    fi
  done <<< "${CONFLICTING_FILES}"

  # Retry the merge after removing conflicts
  echo "==> Retrying merge..."
  git merge "${REMOTE}/${BRANCH}"
  echo "Pull succeeded after removing untracked conflicts."
  exit 0
fi

# Content conflict or other merge failure — abort the in-progress merge if any
if git rev-parse --verify MERGE_HEAD >/dev/null 2>&1; then
  git merge --abort 2>/dev/null || true
fi

# Surface the error
echo "ERROR: Merge failed for an unexpected reason:"
echo "${MERGE_OUTPUT}"
exit 1
