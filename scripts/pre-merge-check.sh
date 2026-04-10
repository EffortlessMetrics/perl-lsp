#!/usr/bin/env bash
# pre-merge-check.sh — Pre-merge guard for ops agents
#
# Checks that a PR is safe to merge:
#   1. Not in draft state
#   2. Has the merge-ready label
#   3. Title contains an issue reference (#NNN)
#
# Usage:
#   scripts/pre-merge-check.sh <pr-number>
#
# Exit codes:
#   0  All checks passed — safe to merge
#   1  One or more checks failed — skip this PR
#
# Designed to be called by ops-merge-batch before each merge attempt.
# A failure skips the individual PR with a clear message; it does not abort
# the whole batch.

set -euo pipefail

PR="${1:?usage: $0 <pr-number>}"

# ── Fetch PR metadata ─────────────────────────────────────────────────────────

PR_JSON="$(gh pr view "$PR" --json isDraft,labels,title)"

IS_DRAFT="$(printf '%s' "$PR_JSON" | jq -r '.isDraft')"
HAS_MERGE_READY="$(printf '%s' "$PR_JSON" | jq -r '[.labels[].name] | any(. == "merge-ready")')"
TITLE="$(printf '%s' "$PR_JSON" | jq -r '.title')"

# ── Run checks ────────────────────────────────────────────────────────────────

FAILED=0

# Check 1: Not a draft
if [[ "$IS_DRAFT" == "true" ]]; then
    echo "FAIL PR #$PR: still in draft state — mark as ready for review first" >&2
    FAILED=1
fi

# Check 2: Has merge-ready label
if [[ "$HAS_MERGE_READY" != "true" ]]; then
    echo "FAIL PR #$PR: missing 'merge-ready' label — route through reviewer → /pr-ready first" >&2
    FAILED=1
fi

# Check 3: Title contains issue reference (#NNN)
if ! printf '%s' "$TITLE" | grep -qE '\(#[0-9]+\)'; then
    echo "FAIL PR #$PR: title missing issue reference — add (#NNN) to the PR title" >&2
    echo "     Current title: $TITLE" >&2
    FAILED=1
fi

# ── Result ────────────────────────────────────────────────────────────────────

if [[ "$FAILED" -eq 0 ]]; then
    echo "OK   PR #$PR: pre-merge checks passed (not draft, merge-ready label present, issue ref in title)"
    exit 0
else
    exit 1
fi
