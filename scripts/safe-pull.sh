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

echo "==> Fetching ${REMOTE}..."
git fetch "${REMOTE}" "${BRANCH}"

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

# Try a normal merge first
if git merge "${REMOTE}/${BRANCH}" 2>/dev/null; then
  echo "Pull succeeded."
  exit 0
fi

# If merge failed, capture the error to check for untracked file conflicts
MERGE_OUTPUT=$(git merge "${REMOTE}/${BRANCH}" 2>&1 || true)

if echo "${MERGE_OUTPUT}" | grep -q "would be overwritten by merge"; then
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

# Some other merge failure — surface it
echo "ERROR: Merge failed for an unexpected reason:"
echo "${MERGE_OUTPUT}"
exit 1
