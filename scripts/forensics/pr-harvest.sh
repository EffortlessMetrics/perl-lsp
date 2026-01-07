#!/usr/bin/env bash
# PR Harvest: Extract fact bundle from a merged GitHub PR
#
# Outputs structured YAML with PR metadata, commits, change surface,
# and verification data for forensics analysis.
#
# Usage:
#   ./scripts/forensics/pr-harvest.sh 259
#   ./scripts/forensics/pr-harvest.sh 259 -o pr-259-facts.yaml
#
# Requires: gh, git, jq, yq (optional, falls back to manual YAML)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# -----------------------------------------------------------------------------
# Usage
# -----------------------------------------------------------------------------
usage() {
    cat <<EOF
Usage: $(basename "$0") <PR_NUMBER> [-o <output_file>]

Extract fact bundle from a merged PR for forensics analysis.

Arguments:
    PR_NUMBER       GitHub PR number (required)
    -o <file>       Output to file instead of stdout

Examples:
    $(basename "$0") 259
    $(basename "$0") 259 -o pr-259-facts.yaml

Output: Structured YAML matching docs/FORENSICS_SCHEMA.md
EOF
    exit 1
}

# -----------------------------------------------------------------------------
# Helpers
# -----------------------------------------------------------------------------
log() {
    echo "[pr-harvest] $*" >&2
}

die() {
    echo "[pr-harvest] ERROR: $*" >&2
    exit 1
}

# Escape YAML string (handle multiline, special chars)
yaml_escape() {
    local str="$1"
    # If contains newlines or special chars, use literal block
    if [[ "$str" == *$'\n'* ]] || [[ "$str" == *':'* && "$str" != *': '* ]]; then
        echo "|"
        echo "$str" | sed 's/^/    /'
    else
        # Simple quoting for single-line strings
        printf '"%s"' "$(echo "$str" | sed 's/"/\\"/g')"
    fi
}

# Format ISO8601 date (handle GitHub's format)
format_date() {
    local date="$1"
    # GitHub returns ISO8601, just pass through
    echo "$date"
}

# -----------------------------------------------------------------------------
# Parse arguments
# -----------------------------------------------------------------------------
PR_NUMBER=""
OUTPUT_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
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
[[ "$PR_NUMBER" =~ ^[0-9]+$ ]] || die "PR number must be numeric: $PR_NUMBER"

# -----------------------------------------------------------------------------
# Verify prerequisites
# -----------------------------------------------------------------------------
command -v gh >/dev/null 2>&1 || die "gh CLI is required (brew install gh)"
command -v jq >/dev/null 2>&1 || die "jq is required (apt install jq)"

# Check gh auth
gh auth status >/dev/null 2>&1 || die "gh not authenticated (run: gh auth login)"

log "Harvesting PR #$PR_NUMBER..."

# -----------------------------------------------------------------------------
# Fetch PR metadata from GitHub
# -----------------------------------------------------------------------------
log "Fetching PR metadata..."
PR_JSON=$(gh pr view "$PR_NUMBER" --json \
    number,title,url,author,createdAt,mergedAt,labels,body,\
headRefOid,baseRefName,commits,files,additions,deletions,changedFiles,\
statusCheckRollup,reviews,mergeCommit 2>/dev/null) || die "Failed to fetch PR #$PR_NUMBER"

# Check if PR is merged
MERGED_AT=$(echo "$PR_JSON" | jq -r '.mergedAt // empty')
[[ -z "$MERGED_AT" || "$MERGED_AT" == "null" ]] && die "PR #$PR_NUMBER is not merged"

# Extract basic metadata
TITLE=$(echo "$PR_JSON" | jq -r '.title')
URL=$(echo "$PR_JSON" | jq -r '.url')
AUTHOR=$(echo "$PR_JSON" | jq -r '.author.login')
CREATED_AT=$(echo "$PR_JSON" | jq -r '.createdAt')
HEAD_SHA=$(echo "$PR_JSON" | jq -r '.headRefOid')
BASE_REF=$(echo "$PR_JSON" | jq -r '.baseRefName')
MERGE_COMMIT=$(echo "$PR_JSON" | jq -r '.mergeCommit.oid // empty')

# Extract labels as YAML array
LABELS=$(echo "$PR_JSON" | jq -r '.labels | map(.name) | if length == 0 then "[]" else . end | @json')

# Extract body (may be multiline)
BODY=$(echo "$PR_JSON" | jq -r '.body // ""')

log "PR: $TITLE"
log "Merged: $MERGED_AT"

# -----------------------------------------------------------------------------
# Compute base SHA from merge commit
# -----------------------------------------------------------------------------
log "Computing base SHA..."
BASE_SHA=""
if [[ -n "$MERGE_COMMIT" && "$MERGE_COMMIT" != "null" ]]; then
    # Get the first parent of the merge commit (the base)
    BASE_SHA=$(git rev-parse "${MERGE_COMMIT}^1" 2>/dev/null || true)
fi

# Fallback: use first commit's parent
if [[ -z "$BASE_SHA" ]]; then
    FIRST_COMMIT=$(echo "$PR_JSON" | jq -r '.commits[0].oid // empty')
    if [[ -n "$FIRST_COMMIT" ]]; then
        BASE_SHA=$(git rev-parse "${FIRST_COMMIT}^" 2>/dev/null || echo "unknown")
    else
        BASE_SHA="unknown"
    fi
fi

# -----------------------------------------------------------------------------
# Extract commit history
# -----------------------------------------------------------------------------
log "Extracting commit history..."
COMMIT_COUNT=$(echo "$PR_JSON" | jq '.commits | length')

# Build commit history YAML
COMMIT_HISTORY=""
while IFS= read -r line; do
    SHA=$(echo "$line" | jq -r '.oid')
    DATE=$(echo "$line" | jq -r '.authoredDate')
    COMMIT_AUTHOR=$(echo "$line" | jq -r '.authors[0].login // .authors[0].name // "unknown"')
    MESSAGE=$(echo "$line" | jq -r '.messageHeadline' | sed 's/"/\\"/g')

    COMMIT_HISTORY+="    - sha: \"$SHA\"
      date: \"$DATE\"
      author: \"$COMMIT_AUTHOR\"
      message: \"$MESSAGE\"
"
done < <(echo "$PR_JSON" | jq -c '.commits[]')

# -----------------------------------------------------------------------------
# Extract change surface
# -----------------------------------------------------------------------------
log "Extracting change surface..."
FILES_CHANGED=$(echo "$PR_JSON" | jq -r '.changedFiles')
INSERTIONS=$(echo "$PR_JSON" | jq -r '.additions')
DELETIONS=$(echo "$PR_JSON" | jq -r '.deletions')

# Build hotspots (files with most changes)
HOTSPOTS=""
while IFS= read -r line; do
    FILE_PATH=$(echo "$line" | jq -r '.path')
    FILE_ADD=$(echo "$line" | jq -r '.additions')
    FILE_DEL=$(echo "$line" | jq -r '.deletions')

    HOTSPOTS+="    - path: \"$FILE_PATH\"
      insertions: $FILE_ADD
      deletions: $FILE_DEL
"
done < <(echo "$PR_JSON" | jq -c '.files | sort_by(-(.additions + .deletions))[:10][]')

# Extract crates touched
CRATES_TOUCHED=$(echo "$PR_JSON" | jq -r '[.files[].path | capture("^crates/(?<crate>[^/]+)") | .crate] | unique | @json')

# -----------------------------------------------------------------------------
# Analyze dependency changes
# -----------------------------------------------------------------------------
log "Analyzing dependency changes..."
DEPS_ADDED="[]"
DEPS_REMOVED="[]"
DEPS_UPDATED="[]"

# Check if Cargo.lock was modified
CARGO_LOCK_CHANGED=$(echo "$PR_JSON" | jq -r '.files[] | select(.path == "Cargo.lock") | .path // empty')

if [[ -n "$CARGO_LOCK_CHANGED" && -n "$MERGE_COMMIT" && "$MERGE_COMMIT" != "null" ]]; then
    log "Parsing Cargo.lock diff..."

    # Get the Cargo.lock diff
    CARGO_DIFF=$(git diff "${BASE_SHA}..${HEAD_SHA}" -- Cargo.lock 2>/dev/null || true)

    if [[ -n "$CARGO_DIFF" ]]; then
        # Extract added packages (lines starting with +name = "...")
        ADDED=$(echo "$CARGO_DIFF" | grep '^+name = "' | sed 's/^+name = "\([^"]*\)".*/\1/' | sort -u | jq -R -s 'split("\n") | map(select(length > 0))')
        [[ "$ADDED" != "[]" ]] && DEPS_ADDED="$ADDED"

        # Extract removed packages
        REMOVED=$(echo "$CARGO_DIFF" | grep '^-name = "' | sed 's/^-name = "\([^"]*\)".*/\1/' | sort -u | jq -R -s 'split("\n") | map(select(length > 0))')
        [[ "$REMOVED" != "[]" ]] && DEPS_REMOVED="$REMOVED"

        # Updated = packages that appear in both added and removed
        if [[ "$DEPS_ADDED" != "[]" && "$DEPS_REMOVED" != "[]" ]]; then
            DEPS_UPDATED=$(jq -n --argjson a "$DEPS_ADDED" --argjson r "$DEPS_REMOVED" '$a - ($a - $r)')
            # Remove updated from added/removed
            DEPS_ADDED=$(jq -n --argjson a "$DEPS_ADDED" --argjson u "$DEPS_UPDATED" '$a - $u')
            DEPS_REMOVED=$(jq -n --argjson r "$DEPS_REMOVED" --argjson u "$DEPS_UPDATED" '$r - $u')
        fi
    fi
fi

# -----------------------------------------------------------------------------
# Extract check run results
# -----------------------------------------------------------------------------
log "Extracting check run results..."
CHECK_RUNS=""
while IFS= read -r line; do
    CHECK_NAME=$(echo "$line" | jq -r '.name // .context // "unknown"')
    CHECK_CONCLUSION=$(echo "$line" | jq -r '.conclusion // .state // "unknown"' | tr '[:upper:]' '[:lower:]')

    CHECK_RUNS+="    - name: \"$CHECK_NAME\"
      conclusion: \"$CHECK_CONCLUSION\"
"
done < <(echo "$PR_JSON" | jq -c '.statusCheckRollup[] | select(.name != null or .context != null)')

# If no check runs, add placeholder
[[ -z "$CHECK_RUNS" ]] && CHECK_RUNS="    []  # No check runs recorded
"

# -----------------------------------------------------------------------------
# Extract reviewers
# -----------------------------------------------------------------------------
REVIEWERS=$(echo "$PR_JSON" | jq -r '[.reviews[].author.login] | unique | @json')

# -----------------------------------------------------------------------------
# Generate YAML output
# -----------------------------------------------------------------------------
generate_yaml() {
    cat <<EOF
# PR Fact Bundle
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Script: scripts/forensics/pr-harvest.sh

pr:
  number: $PR_NUMBER
  title: "$TITLE"
  url: "$URL"
  author: "$AUTHOR"
  created_at: "$CREATED_AT"
  merged_at: "$MERGED_AT"
  labels: $LABELS
  reviewers: $REVIEWERS

commits:
  base_sha: "$BASE_SHA"
  head_sha: "$HEAD_SHA"
  merge_commit: "${MERGE_COMMIT:-null}"
  count: $COMMIT_COUNT
  history:
$COMMIT_HISTORY
change_surface:
  files_changed: $FILES_CHANGED
  insertions: $INSERTIONS
  deletions: $DELETIONS
  hotspots:
$HOTSPOTS
  crates_touched: $CRATES_TOUCHED
  dependency_delta:
    added: $DEPS_ADDED
    removed: $DEPS_REMOVED
    updated: $DEPS_UPDATED

verification:
  check_runs:
$CHECK_RUNS
# Body preserved for context analysis
body: |
$(echo "$BODY" | sed 's/^/  /')
EOF
}

# -----------------------------------------------------------------------------
# Output
# -----------------------------------------------------------------------------
if [[ -n "$OUTPUT_FILE" ]]; then
    generate_yaml > "$OUTPUT_FILE"
    log "Output written to: $OUTPUT_FILE"
else
    generate_yaml
fi

log "Done."
