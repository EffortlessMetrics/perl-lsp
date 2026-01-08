#!/usr/bin/env bash
# lib_gh.sh - Shared functions for GitHub/git operations in forensics scripts
#
# This library provides common utilities for:
# - SHA extraction from GitHub PR metadata
# - Fallback resolution for older gh CLI versions
# - Merge commit parent computation
#
# Usage:
#   source "$SCRIPT_DIR/lib_gh.sh"
#   read -r BASE_SHA HEAD_SHA MERGE_COMMIT < <(extract_shas_from_json "$metadata_json")
#   read -r BASE_SHA HEAD_SHA MERGE_COMMIT < <(extract_shas_from_file "$metadata_file")
#
# See: scripts/forensics/pr-harvest.sh, dossier-runner.sh, temporal-analysis.sh

# -----------------------------------------------------------------------------
# SHA Extraction from JSON string (for piped gh output)
# -----------------------------------------------------------------------------
# Extract base, head, and merge commit SHAs from PR metadata JSON
# Handles multiple gh CLI versions with appropriate fallbacks
#
# Arguments:
#   $1 - JSON string containing PR metadata
#
# Output (space-separated): BASE_SHA HEAD_SHA MERGE_COMMIT
# Any value may be empty if not available
#
# Example:
#   PR_JSON=$(gh pr view 123 --json baseRefOid,headRefOid,mergeCommit,commits)
#   read -r base head merge < <(extract_shas_from_json "$PR_JSON")
extract_shas_from_json() {
    local json="$1"
    local base head merge

    # Primary: Try baseRefOid/headRefOid (newer gh CLI)
    base=$(echo "$json" | jq -r '.baseRefOid // empty' 2>/dev/null || true)
    head=$(echo "$json" | jq -r '.headRefOid // empty' 2>/dev/null || true)
    merge=$(echo "$json" | jq -r '.mergeCommit.oid // empty' 2>/dev/null || true)

    # Fallback: Extract HEAD_SHA from last commit in PR
    if [[ -z "$head" || "$head" == "null" ]]; then
        head=$(echo "$json" | jq -r '.commits[-1].oid // empty' 2>/dev/null || true)
    fi

    # Fallback: Compute BASE_SHA from merge commit parent
    if [[ -z "$base" || "$base" == "null" ]]; then
        if [[ -n "$merge" && "$merge" != "null" ]] && git cat-file -t "$merge" >/dev/null 2>&1; then
            base=$(git rev-parse "${merge}^1" 2>/dev/null || true)
        fi
    fi

    # Normalize null to empty
    [[ "$base" == "null" ]] && base=""
    [[ "$head" == "null" ]] && head=""
    [[ "$merge" == "null" ]] && merge=""

    echo "$base $head $merge"
}

# -----------------------------------------------------------------------------
# SHA Extraction from file (for harvested metadata)
# -----------------------------------------------------------------------------
# Extract base, head, and merge commit SHAs from PR metadata JSON file
#
# Arguments:
#   $1 - Path to metadata.json file
#
# Output (space-separated): BASE_SHA HEAD_SHA MERGE_COMMIT
#
# Example:
#   read -r base head merge < <(extract_shas_from_file "$HARVEST_DIR/metadata.json")
extract_shas_from_file() {
    local file="$1"

    if [[ ! -f "$file" ]]; then
        echo "" "" ""
        return 1
    fi

    local base head merge

    # Primary: Try baseRefOid/headRefOid
    base=$(jq -r '.baseRefOid // empty' "$file" 2>/dev/null || true)
    head=$(jq -r '.headRefOid // empty' "$file" 2>/dev/null || true)
    merge=$(jq -r '.mergeCommit.oid // empty' "$file" 2>/dev/null || true)

    # Fallback: Extract HEAD_SHA from last commit in PR
    if [[ -z "$head" || "$head" == "null" ]]; then
        head=$(jq -r '.commits[-1].oid // empty' "$file" 2>/dev/null || true)
    fi

    # Fallback: Compute BASE_SHA from merge commit parent
    if [[ -z "$base" || "$base" == "null" ]]; then
        if [[ -n "$merge" && "$merge" != "null" ]] && git cat-file -t "$merge" >/dev/null 2>&1; then
            base=$(git rev-parse "${merge}^1" 2>/dev/null || true)
        fi
    fi

    # Normalize null to empty
    [[ "$base" == "null" ]] && base=""
    [[ "$head" == "null" ]] && head=""
    [[ "$merge" == "null" ]] && merge=""

    echo "$base $head $merge"
}

# -----------------------------------------------------------------------------
# SHA Validation
# -----------------------------------------------------------------------------
# Check if SHAs are valid and exist locally
#
# Arguments:
#   $1 - BASE_SHA
#   $2 - HEAD_SHA
#
# Returns:
#   0 if both SHAs are valid and exist locally
#   1 otherwise
#
# Example:
#   if validate_shas "$BASE_SHA" "$HEAD_SHA"; then
#       git diff "$BASE_SHA..$HEAD_SHA"
#   fi
validate_shas() {
    local base="$1"
    local head="$2"

    [[ -z "$base" || -z "$head" ]] && return 1
    [[ "$base" == "null" || "$head" == "null" ]] && return 1

    git cat-file -e "$base" 2>/dev/null || return 1
    git cat-file -e "$head" 2>/dev/null || return 1

    return 0
}

# -----------------------------------------------------------------------------
# Compute diff range with fallbacks
# -----------------------------------------------------------------------------
# Compute the best available diff range for a PR
#
# Arguments:
#   $1 - BASE_SHA
#   $2 - HEAD_SHA
#   $3 - MERGE_COMMIT (optional)
#
# Output: Diff range string (e.g., "abc123..def456") or empty if unavailable
#
# Example:
#   range=$(compute_diff_range "$BASE_SHA" "$HEAD_SHA" "$MERGE_COMMIT")
#   if [[ -n "$range" ]]; then
#       git diff "$range"
#   fi
compute_diff_range() {
    local base="$1"
    local head="$2"
    local merge="${3:-}"

    # Try base..head first
    if validate_shas "$base" "$head"; then
        echo "${base}..${head}"
        return 0
    fi

    # Fallback: use merge commit parents
    if [[ -n "$merge" && "$merge" != "null" ]] && git cat-file -t "$merge" >/dev/null 2>&1; then
        local merge_base
        merge_base=$(git rev-parse "${merge}^1" 2>/dev/null || true)
        if [[ -n "$merge_base" ]]; then
            echo "${merge_base}..${merge}"
            return 0
        fi
    fi

    # No valid range available
    echo ""
    return 1
}
