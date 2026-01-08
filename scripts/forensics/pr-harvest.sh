#!/usr/bin/env bash
# PR Harvest: Extract raw fact bundles from GitHub and git for forensics analysis
#
# Pulls all raw facts needed for PR forensics analysis including:
# - GitHub PR metadata
# - Commits and full messages
# - Files changed
# - PR thread (comments and reviews)
# - Git diff stats
# - Commit timeline
#
# Usage:
#   ./scripts/forensics/pr-harvest.sh <PR_NUMBER>
#   ./scripts/forensics/pr-harvest.sh <PR_NUMBER> -o ./custom-output
#
# Output directory structure:
#   <output_dir>/pr-<number>/
#     metadata.json       # PR metadata from gh
#     commits.json        # Commit list
#     files.json          # Files changed
#     comments.json       # PR thread (comments + reviews)
#     diff.stat           # git diff --stat
#     diff.numstat        # git diff --numstat
#     timeline.txt        # commit timestamps for temporal analysis
#     full_messages.txt   # full commit messages
#
# See: docs/FORENSICS_SCHEMA.md, docs/DEVLT_ESTIMATION.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# -----------------------------------------------------------------------------
# Usage
# -----------------------------------------------------------------------------
usage() {
    cat <<EOF
Usage: $(basename "$0") <PR_NUMBER> [-o <output_dir>]

Extract raw fact bundle from a PR for forensics analysis.

Arguments:
    PR_NUMBER           GitHub PR number (required)
    -o, --output <dir>  Output directory (default: ./forensics-output)

Output structure:
    <output_dir>/pr-<number>/
      metadata.json       PR metadata from gh
      commits.json        Commit list with SHAs and messages
      files.json          Files changed with additions/deletions
      comments.json       PR comments and reviews
      diff.stat           git diff --stat histogram
      diff.numstat        git diff --numstat precise counts
      timeline.txt        Commit timestamps for temporal analysis
      full_messages.txt   Full commit messages for decision event extraction

Examples:
    $(basename "$0") 259
    $(basename "$0") 259 -o ./pr-analysis

Exit codes:
    0   Success
    1   Invalid arguments or prerequisites not met
    2   PR not found or cannot be fetched
EOF
    exit 1
}

# -----------------------------------------------------------------------------
# Logging (all to stderr)
# -----------------------------------------------------------------------------
log() {
    echo "[pr-harvest] $*" >&2
}

log_progress() {
    echo "[pr-harvest] >>> $*" >&2
}

die() {
    echo "[pr-harvest] ERROR: $*" >&2
    exit 2
}

# -----------------------------------------------------------------------------
# Parse arguments
# -----------------------------------------------------------------------------
PR_NUMBER=""
OUTPUT_DIR="./forensics-output"

while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--output)
            if [[ -z "${2:-}" ]]; then
                die "--output requires a directory path"
            fi
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        -*)
            die "Unknown option: $1"
            ;;
        *)
            if [[ -z "$PR_NUMBER" ]]; then
                PR_NUMBER="$1"
            else
                die "Unexpected argument: $1"
            fi
            shift
            ;;
    esac
done

[[ -z "$PR_NUMBER" ]] && usage

# Validate PR number is numeric
if ! [[ "$PR_NUMBER" =~ ^[0-9]+$ ]]; then
    die "PR number must be numeric: $PR_NUMBER"
fi

# -----------------------------------------------------------------------------
# Verify prerequisites
# -----------------------------------------------------------------------------
log_progress "Checking prerequisites..."

if ! command -v gh >/dev/null 2>&1; then
    die "gh CLI is required (install: brew install gh / apt install gh)"
fi

if ! command -v jq >/dev/null 2>&1; then
    die "jq is required (install: apt install jq / brew install jq)"
fi

if ! command -v git >/dev/null 2>&1; then
    die "git is required"
fi

# Check gh authentication
if ! gh auth status >/dev/null 2>&1; then
    die "gh not authenticated (run: gh auth login)"
fi

log "Prerequisites OK"

# -----------------------------------------------------------------------------
# Create output directory
# -----------------------------------------------------------------------------
PR_DIR="${OUTPUT_DIR}/pr-${PR_NUMBER}"
mkdir -p "$PR_DIR"
log "Output directory: $PR_DIR"

# -----------------------------------------------------------------------------
# 1. Fetch PR metadata from GitHub
# -----------------------------------------------------------------------------
log_progress "Fetching PR metadata..."

# Note: baseRefOid/headRefOid may not be available in older gh versions
# We fetch commits to extract head SHA, and compute base from merge commit parent
PR_METADATA=$(gh pr view "$PR_NUMBER" --json \
    number,title,url,state,createdAt,mergedAt,\
author,labels,body,\
baseRefName,headRefName,\
additions,deletions,changedFiles,\
mergeCommit,commits 2>/dev/null) || die "Failed to fetch PR #$PR_NUMBER (does it exist?)"

# Validate PR exists and has required fields
PR_STATE=$(echo "$PR_METADATA" | jq -r '.state')
if [[ "$PR_STATE" == "null" || -z "$PR_STATE" ]]; then
    die "PR #$PR_NUMBER not found or has invalid state"
fi

# Write metadata
echo "$PR_METADATA" | jq '.' > "${PR_DIR}/metadata.json"
log "Saved: metadata.json"

# Extract SHAs for git operations
# Try baseRefOid/headRefOid first (newer gh), fallback to commits/mergeCommit (older gh)
BASE_SHA=$(echo "$PR_METADATA" | jq -r '.baseRefOid // empty')
HEAD_SHA=$(echo "$PR_METADATA" | jq -r '.headRefOid // empty')
MERGE_COMMIT=$(echo "$PR_METADATA" | jq -r '.mergeCommit.oid // empty')

# Fallback: extract HEAD_SHA from last commit in PR
if [[ -z "$HEAD_SHA" || "$HEAD_SHA" == "null" ]]; then
    HEAD_SHA=$(echo "$PR_METADATA" | jq -r '.commits[-1].oid // empty')
fi

# Fallback: compute BASE_SHA from merge commit parent
if [[ -z "$BASE_SHA" || "$BASE_SHA" == "null" ]]; then
    if [[ -n "$MERGE_COMMIT" && "$MERGE_COMMIT" != "null" ]] && git cat-file -t "$MERGE_COMMIT" >/dev/null 2>&1; then
        BASE_SHA=$(git rev-parse "${MERGE_COMMIT}^1" 2>/dev/null || echo "")
    fi
fi
TITLE=$(echo "$PR_METADATA" | jq -r '.title')
MERGED_AT=$(echo "$PR_METADATA" | jq -r '.mergedAt // "not merged"')

log "PR: $TITLE"
log "State: $PR_STATE | Merged: $MERGED_AT"

# -----------------------------------------------------------------------------
# 2. Fetch commits
# -----------------------------------------------------------------------------
log_progress "Fetching commits..."

COMMITS_JSON=$(gh pr view "$PR_NUMBER" --json commits 2>/dev/null) || {
    log "Warning: Could not fetch commits from gh, will use git log"
    COMMITS_JSON='{"commits": []}'
}

echo "$COMMITS_JSON" | jq '.' > "${PR_DIR}/commits.json"
log "Saved: commits.json"

# -----------------------------------------------------------------------------
# 3. Fetch files changed
# -----------------------------------------------------------------------------
log_progress "Fetching files changed..."

FILES_JSON=$(gh pr view "$PR_NUMBER" --json files 2>/dev/null) || {
    log "Warning: Could not fetch files list"
    FILES_JSON='{"files": []}'
}

echo "$FILES_JSON" | jq '.' > "${PR_DIR}/files.json"
log "Saved: files.json"

# -----------------------------------------------------------------------------
# 4. Fetch PR thread (comments and reviews)
# -----------------------------------------------------------------------------
log_progress "Fetching PR thread..."

# Fetch comments
COMMENTS_JSON=$(gh pr view "$PR_NUMBER" --json comments,reviews,reviewRequests 2>/dev/null) || {
    log "Warning: Could not fetch PR thread"
    COMMENTS_JSON='{"comments": [], "reviews": [], "reviewRequests": []}'
}

echo "$COMMENTS_JSON" | jq '.' > "${PR_DIR}/comments.json"
log "Saved: comments.json"

# -----------------------------------------------------------------------------
# 5. Git diff stats (requires SHAs)
# -----------------------------------------------------------------------------
log_progress "Computing git diff stats..."

cd "$PROJECT_ROOT"

# Determine the range for git operations
# Prefer: base_sha..head_sha, fallback to merge_commit parents
DIFF_RANGE=""
DIFF_AVAILABLE=false

if [[ -n "$BASE_SHA" && -n "$HEAD_SHA" && "$BASE_SHA" != "null" && "$HEAD_SHA" != "null" ]]; then
    # Verify both SHAs exist locally
    if git cat-file -t "$BASE_SHA" >/dev/null 2>&1 && git cat-file -t "$HEAD_SHA" >/dev/null 2>&1; then
        DIFF_RANGE="${BASE_SHA}..${HEAD_SHA}"
        DIFF_AVAILABLE=true
        log "Using range: $DIFF_RANGE"
    else
        log "Warning: Base or head SHA not found locally, trying merge commit"
    fi
fi

# Fallback: use merge commit parents
if [[ "$DIFF_AVAILABLE" == "false" && -n "$MERGE_COMMIT" && "$MERGE_COMMIT" != "null" ]]; then
    if git cat-file -t "$MERGE_COMMIT" >/dev/null 2>&1; then
        # Get first parent (the base) of merge commit
        MERGE_BASE=$(git rev-parse "${MERGE_COMMIT}^1" 2>/dev/null || true)
        if [[ -n "$MERGE_BASE" ]]; then
            DIFF_RANGE="${MERGE_BASE}..${MERGE_COMMIT}"
            DIFF_AVAILABLE=true
            log "Using merge commit range: $DIFF_RANGE"
        fi
    else
        log "Warning: Merge commit not found locally"
    fi
fi

# Generate diff stats
if [[ "$DIFF_AVAILABLE" == "true" ]]; then
    # diff --stat for histogram view
    git diff --stat "$DIFF_RANGE" > "${PR_DIR}/diff.stat" 2>/dev/null || {
        echo "# diff --stat failed" > "${PR_DIR}/diff.stat"
    }
    log "Saved: diff.stat"

    # diff --numstat for precise counts
    git diff --numstat "$DIFF_RANGE" > "${PR_DIR}/diff.numstat" 2>/dev/null || {
        echo "# diff --numstat failed" > "${PR_DIR}/diff.numstat"
    }
    log "Saved: diff.numstat"
else
    echo "# Git diff not available - SHAs not found locally" > "${PR_DIR}/diff.stat"
    echo "# Run 'git fetch origin' and retry to get diff stats" >> "${PR_DIR}/diff.stat"
    echo "# Reported base SHA: ${BASE_SHA:-unknown}" >> "${PR_DIR}/diff.stat"
    echo "# Reported head SHA: ${HEAD_SHA:-unknown}" >> "${PR_DIR}/diff.stat"
    cp "${PR_DIR}/diff.stat" "${PR_DIR}/diff.numstat"
    log "Warning: Git diff not available (SHAs not found locally)"
fi

# -----------------------------------------------------------------------------
# 6. Commit timeline
# -----------------------------------------------------------------------------
log_progress "Building commit timeline..."

# Extract commit SHAs from commits.json
COMMIT_SHAS=$(jq -r '.commits[].oid // empty' "${PR_DIR}/commits.json" 2>/dev/null)

if [[ -n "$COMMIT_SHAS" ]]; then
    # Build timeline from local git
    {
        echo "# Commit Timeline for PR #${PR_NUMBER}"
        echo "# Format: ISO_TIMESTAMP | SHORT_SHA | AUTHOR | SUBJECT"
        echo "#"

        while IFS= read -r sha; do
            if [[ -n "$sha" ]] && git cat-file -t "$sha" >/dev/null 2>&1; then
                git log -1 --format="%aI | %.8H | %an | %s" "$sha" 2>/dev/null || true
            fi
        done <<< "$COMMIT_SHAS"
    } > "${PR_DIR}/timeline.txt"
    log "Saved: timeline.txt"
else
    # Fallback: use diff range if available
    if [[ "$DIFF_AVAILABLE" == "true" ]]; then
        {
            echo "# Commit Timeline for PR #${PR_NUMBER}"
            echo "# Format: ISO_TIMESTAMP | SHORT_SHA | AUTHOR | SUBJECT"
            echo "#"
            git log --format="%aI | %.8H | %an | %s" "$DIFF_RANGE" 2>/dev/null || true
        } > "${PR_DIR}/timeline.txt"
        log "Saved: timeline.txt (from diff range)"
    else
        {
            echo "# Commit timeline not available"
            echo "# SHAs not found locally - run 'git fetch origin' and retry"
        } > "${PR_DIR}/timeline.txt"
        log "Warning: Timeline not available"
    fi
fi

# -----------------------------------------------------------------------------
# 7. Full commit messages (for decision event extraction)
# -----------------------------------------------------------------------------
log_progress "Extracting full commit messages..."

if [[ -n "$COMMIT_SHAS" ]]; then
    {
        echo "# Full Commit Messages for PR #${PR_NUMBER}"
        echo "# For decision event extraction per DEVLT_ESTIMATION.md"
        echo "#"
        echo ""

        while IFS= read -r sha; do
            if [[ -n "$sha" ]] && git cat-file -t "$sha" >/dev/null 2>&1; then
                echo "================================================================================"
                echo "COMMIT: $sha"
                echo "================================================================================"
                git log -1 --format="Author: %an <%ae>%nDate: %aI%n%n%B" "$sha" 2>/dev/null || true
                echo ""
            fi
        done <<< "$COMMIT_SHAS"
    } > "${PR_DIR}/full_messages.txt"
    log "Saved: full_messages.txt"
else
    # Fallback: use diff range
    if [[ "$DIFF_AVAILABLE" == "true" ]]; then
        {
            echo "# Full Commit Messages for PR #${PR_NUMBER}"
            echo "# For decision event extraction per DEVLT_ESTIMATION.md"
            echo "#"
            echo ""
            git log --format="================================================================================%nCOMMIT: %H%n=================================================================================%nAuthor: %an <%ae>%nDate: %aI%n%n%B%n" "$DIFF_RANGE" 2>/dev/null || true
        } > "${PR_DIR}/full_messages.txt"
        log "Saved: full_messages.txt (from diff range)"
    else
        {
            echo "# Full commit messages not available"
            echo "# SHAs not found locally - run 'git fetch origin' and retry"
        } > "${PR_DIR}/full_messages.txt"
        log "Warning: Full messages not available"
    fi
fi

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
log_progress "Harvest complete!"

# Count what we got
COMMIT_COUNT=$(jq '.commits | length' "${PR_DIR}/commits.json" 2>/dev/null || echo "0")
FILE_COUNT=$(jq '.files | length' "${PR_DIR}/files.json" 2>/dev/null || echo "0")
COMMENT_COUNT=$(jq '.comments | length' "${PR_DIR}/comments.json" 2>/dev/null || echo "0")
REVIEW_COUNT=$(jq '.reviews | length' "${PR_DIR}/comments.json" 2>/dev/null || echo "0")

log ""
log "Summary:"
log "  PR #${PR_NUMBER}: $TITLE"
log "  Commits: $COMMIT_COUNT"
log "  Files changed: $FILE_COUNT"
log "  Comments: $COMMENT_COUNT"
log "  Reviews: $REVIEW_COUNT"
log "  Git diff: $(if [[ "$DIFF_AVAILABLE" == "true" ]]; then echo "available"; else echo "NOT available (fetch and retry)"; fi)"
log ""
log "Output: $PR_DIR"
log ""

# List files
log "Files created:"
for f in metadata.json commits.json files.json comments.json diff.stat diff.numstat timeline.txt full_messages.txt; do
    if [[ -f "${PR_DIR}/$f" ]]; then
        SIZE=$(wc -c < "${PR_DIR}/$f" | tr -d ' ')
        log "  $f (${SIZE} bytes)"
    fi
done

# Exit with success, print path for scripting
echo "$PR_DIR"
exit 0
