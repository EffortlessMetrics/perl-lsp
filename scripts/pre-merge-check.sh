#!/usr/bin/env bash
# pre-merge-check.sh — Pre-merge guard for ops agents
#
# Checks that a PR is safe to merge:
#   1. Not in draft state
#   2. Has the merge-ready label
#   3. Title contains an issue reference (#NNN)
#   4. Has deep-review coverage, unless the PR is docs-only
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

json_read() {
    local filter="$1"
    printf '%s' "$PR_JSON" | jq -r "$filter" | tr -d '\r'
}

# ── Fetch PR metadata ─────────────────────────────────────────────────────────

PR_JSON="$(gh pr view "$PR" --json isDraft,labels,title,files)"

IS_DRAFT="$(json_read '.isDraft')"
HAS_MERGE_READY="$(json_read '[.labels[].name] | any(. == "merge-ready")')"
HAS_REVIEWED_DEEP="$(json_read '[.labels[].name] | any(. == "reviewed-deep")')"
TITLE="$(json_read '.title')"

is_docs_only_path() {
    local path="$1"
    case "$path" in
        docs/*) return 0 ;;
        *.md|*.mdx|*.txt|*.rst|*.adoc) return 0 ;;
        *) return 1 ;;
    esac
}

DOCS_ONLY=true
FILE_COUNT=0
while IFS= read -r changed_path; do
    [[ -z "$changed_path" ]] && continue
    FILE_COUNT=$((FILE_COUNT + 1))
    if ! is_docs_only_path "$changed_path"; then
        DOCS_ONLY=false
        break
    fi
done < <(json_read '.files[]?.path // empty')

if [[ "$FILE_COUNT" -eq 0 ]]; then
    DOCS_ONLY=false
fi

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

# Check 4: Non-docs PRs require reviewed-deep
if [[ "$HAS_REVIEWED_DEEP" != "true" && "$DOCS_ONLY" != "true" ]]; then
    echo "FAIL PR #$PR: missing 'reviewed-deep' label on a non-docs PR — route through reviewer-deep first" >&2
    FAILED=1
fi

# ── Result ────────────────────────────────────────────────────────────────────

if [[ "$FAILED" -eq 0 ]]; then
    if [[ "$DOCS_ONLY" == "true" && "$HAS_REVIEWED_DEEP" != "true" ]]; then
        echo "OK   PR #$PR: pre-merge checks passed (docs-only fast track, merge-ready label present, issue ref in title)"
    else
        echo "OK   PR #$PR: pre-merge checks passed (not draft, merge-ready label present, issue ref in title, deep review covered)"
    fi
    exit 0
else
    exit 1
fi
