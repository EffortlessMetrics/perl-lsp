#!/usr/bin/env bash
# Temporal Analysis: Compute temporal topology from commit history
#
# Analyzes how a PR converged over time - identifies grind sessions,
# friction hotspots, oscillations, and stabilization points.
#
# Usage:
#   ./scripts/forensics/temporal-analysis.sh <harvest_dir>
#   ./scripts/forensics/temporal-analysis.sh <pr_number>
#   ./scripts/forensics/temporal-analysis.sh <pr_number> --gap 45
#   ./scripts/forensics/temporal-analysis.sh <pr_number> -o output.yaml
#
# Inputs:
#   - Path to pr-harvest output directory, OR
#   - PR number (will run pr-harvest first)
#   - Optional: session gap threshold in minutes (default: 30)
#
# Output: YAML to stdout or file
#
# See: docs/DEVLT_ESTIMATION.md for temporal topology concepts

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# -----------------------------------------------------------------------------
# Configuration Defaults
# -----------------------------------------------------------------------------
SESSION_GAP_MINUTES=30
OUTPUT_FILE=""
PR_NUMBER=""
HARVEST_DIR=""

# Grind detection thresholds
GRIND_COMMIT_RATE=4        # commits per session to consider "grind"
GRIND_REPETITIVE_EDITS=3   # same file touched N+ times = repetitive

# Logic file patterns (files that indicate "core work" vs test/docs/config)
LOGIC_PATTERNS=(
    '\.rs$'
    '\.go$'
    '\.py$'
    '\.js$'
    '\.ts$'
    '\.c$'
    '\.cpp$'
    '\.h$'
    '\.java$'
)

# Non-logic patterns (excluded from stabilization detection)
NON_LOGIC_PATTERNS=(
    '^docs/'
    '^test'
    '/tests/'
    '\.md$'
    '\.toml$'
    '\.yaml$'
    '\.yml$'
    '\.json$'
    '^\.github/'
    '\.gitignore$'
    'Cargo\.lock$'
    'package-lock\.json$'
)

# -----------------------------------------------------------------------------
# Usage
# -----------------------------------------------------------------------------
usage() {
    cat <<EOF
Usage: $(basename "$0") <harvest_dir|pr_number> [OPTIONS]

Compute temporal topology from commit history.

Arguments:
    harvest_dir     Path to pr-harvest output directory
    pr_number       GitHub PR number (will run pr-harvest first)

Options:
    --gap MINUTES   Session gap threshold (default: 30)
    -o FILE         Output to file instead of stdout
    -h, --help      Show this help message

Examples:
    $(basename "$0") ./harvest/pr-259
    $(basename "$0") 259
    $(basename "$0") 259 --gap 45
    $(basename "$0") 259 -o temporal-259.yaml

Output: YAML structure per docs/DEVLT_ESTIMATION.md schema
EOF
    exit 0
}

# -----------------------------------------------------------------------------
# Helpers
# -----------------------------------------------------------------------------
log() {
    echo "[temporal-analysis] $*" >&2
}

die() {
    echo "[temporal-analysis] ERROR: $*" >&2
    exit 1
}

# Convert epoch timestamp to ISO8601
timestamp_to_iso() {
    local ts="$1"
    if [[ "$(uname)" == "Darwin" ]]; then
        date -r "$ts" -u +"%Y-%m-%dT%H:%M:%SZ"
    else
        date -u -d "@$ts" +"%Y-%m-%dT%H:%M:%SZ"
    fi
}

# Check if a file path matches logic patterns
is_logic_file() {
    local file="$1"

    # First, check if it matches non-logic patterns
    for pattern in "${NON_LOGIC_PATTERNS[@]}"; do
        if [[ "$file" =~ $pattern ]]; then
            return 1
        fi
    done

    # Then check if it matches logic patterns
    for pattern in "${LOGIC_PATTERNS[@]}"; do
        if [[ "$file" =~ $pattern ]]; then
            return 0
        fi
    done

    # Default: not a logic file
    return 1
}

# Classify commit type from conventional commit message
classify_commit() {
    local msg="$1"
    local lower_msg="${msg,,}"

    if [[ "$lower_msg" =~ ^feat[:\(] || "$lower_msg" =~ ^feature[:\(] ]]; then
        echo "feat"
    elif [[ "$lower_msg" =~ ^fix[:\(] || "$lower_msg" =~ ^bugfix[:\(] ]]; then
        echo "fix"
    elif [[ "$lower_msg" =~ ^test[:\(] || "$lower_msg" =~ ^tests[:\(] ]]; then
        echo "test"
    elif [[ "$lower_msg" =~ ^docs[:\(] || "$lower_msg" =~ ^doc[:\(] ]]; then
        echo "docs"
    elif [[ "$lower_msg" =~ ^chore[:\(] || "$lower_msg" =~ ^build[:\(] || "$lower_msg" =~ ^ci[:\(] ]]; then
        echo "chore"
    elif [[ "$lower_msg" =~ ^refactor[:\(] || "$lower_msg" =~ ^style[:\(] || "$lower_msg" =~ ^perf[:\(] ]]; then
        echo "chore"
    else
        echo "other"
    fi
}

# Check if message indicates a revert
is_revert() {
    local msg="$1"
    local lower_msg="${msg,,}"
    [[ "$lower_msg" =~ ^revert || "$lower_msg" =~ revert: || "$lower_msg" =~ "revert \"" ]]
}

# -----------------------------------------------------------------------------
# Parse Arguments
# -----------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --gap)
            SESSION_GAP_MINUTES="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            if [[ -z "$PR_NUMBER" && -z "$HARVEST_DIR" ]]; then
                # Determine if this is a directory or PR number
                if [[ -d "$1" ]]; then
                    HARVEST_DIR="$1"
                elif [[ "$1" =~ ^[0-9]+$ ]]; then
                    PR_NUMBER="$1"
                else
                    die "Invalid argument: $1 (expected directory path or PR number)"
                fi
            else
                die "Unexpected argument: $1"
            fi
            shift
            ;;
    esac
done

# Validate input
if [[ -z "$PR_NUMBER" && -z "$HARVEST_DIR" ]]; then
    usage
fi

# Convert gap to seconds
SESSION_GAP_SECONDS=$((SESSION_GAP_MINUTES * 60))

cd "$REPO_ROOT"

# -----------------------------------------------------------------------------
# If PR number provided, run pr-harvest first or fetch commits directly
# -----------------------------------------------------------------------------
COMMITS_DATA=""
BASE_SHA=""
HEAD_SHA=""

if [[ -n "$PR_NUMBER" ]]; then
    log "Processing PR #$PR_NUMBER..."

    # Check prerequisites
    command -v gh >/dev/null 2>&1 || die "gh CLI is required (brew install gh)"
    command -v jq >/dev/null 2>&1 || die "jq is required (apt install jq)"
    gh auth status >/dev/null 2>&1 || die "gh not authenticated (run: gh auth login)"

    # Fetch PR info to get commit range
    PR_JSON=$(gh pr view "$PR_NUMBER" --json \
        number,headRefOid,mergeCommit,commits,mergedAt 2>/dev/null) || die "Failed to fetch PR #$PR_NUMBER"

    MERGED_AT=$(echo "$PR_JSON" | jq -r '.mergedAt // empty')
    [[ -z "$MERGED_AT" || "$MERGED_AT" == "null" ]] && die "PR #$PR_NUMBER is not merged"

    HEAD_SHA=$(echo "$PR_JSON" | jq -r '.headRefOid')
    MERGE_COMMIT=$(echo "$PR_JSON" | jq -r '.mergeCommit.oid // empty')

    # Compute base SHA
    if [[ -n "$MERGE_COMMIT" && "$MERGE_COMMIT" != "null" ]]; then
        BASE_SHA=$(git rev-parse "${MERGE_COMMIT}^1" 2>/dev/null || true)
    fi

    if [[ -z "$BASE_SHA" ]]; then
        FIRST_COMMIT=$(echo "$PR_JSON" | jq -r '.commits[0].oid // empty')
        if [[ -n "$FIRST_COMMIT" ]]; then
            BASE_SHA=$(git rev-parse "${FIRST_COMMIT}^" 2>/dev/null || echo "unknown")
        else
            die "Could not determine base SHA for PR #$PR_NUMBER"
        fi
    fi

    COMMIT_RANGE="${BASE_SHA}..${HEAD_SHA}"
    log "Commit range: $COMMIT_RANGE"

elif [[ -n "$HARVEST_DIR" ]]; then
    log "Using harvest directory: $HARVEST_DIR"

    # Look for harvest YAML file
    HARVEST_FILE=""
    if [[ -f "$HARVEST_DIR" && "$HARVEST_DIR" =~ \.ya?ml$ ]]; then
        HARVEST_FILE="$HARVEST_DIR"
    elif [[ -d "$HARVEST_DIR" ]]; then
        # Find YAML file in directory
        HARVEST_FILE=$(find "$HARVEST_DIR" -maxdepth 1 -name "*.yaml" -o -name "*.yml" | head -1)
    fi

    if [[ -z "$HARVEST_FILE" || ! -f "$HARVEST_FILE" ]]; then
        die "Could not find harvest YAML file in: $HARVEST_DIR"
    fi

    # Extract commit info from harvest file
    if command -v yq >/dev/null 2>&1; then
        BASE_SHA=$(yq -r '.commits.base_sha' "$HARVEST_FILE")
        HEAD_SHA=$(yq -r '.commits.head_sha' "$HARVEST_FILE")
        PR_NUMBER=$(yq -r '.pr.number' "$HARVEST_FILE")
    else
        # Fallback: grep parsing
        BASE_SHA=$(grep -E '^\s*base_sha:' "$HARVEST_FILE" | head -1 | sed 's/.*: *"\?\([^"]*\)"\?/\1/')
        HEAD_SHA=$(grep -E '^\s*head_sha:' "$HARVEST_FILE" | head -1 | sed 's/.*: *"\?\([^"]*\)"\?/\1/')
        PR_NUMBER=$(grep -E '^\s*number:' "$HARVEST_FILE" | head -1 | sed 's/.*: *\([0-9]*\).*/\1/')
    fi

    [[ -z "$BASE_SHA" || "$BASE_SHA" == "null" ]] && die "Could not extract base_sha from harvest file"
    [[ -z "$HEAD_SHA" || "$HEAD_SHA" == "null" ]] && die "Could not extract head_sha from harvest file"

    COMMIT_RANGE="${BASE_SHA}..${HEAD_SHA}"
    log "Commit range from harvest: $COMMIT_RANGE"
fi

# -----------------------------------------------------------------------------
# Collect Commit Data
# -----------------------------------------------------------------------------
log "Collecting commit data..."

# Get all commits in range with timestamp, sha, message
# Format: epoch_timestamp|sha|subject
COMMITS_DATA=$(git log --reverse --format="%at|%H|%s" "$COMMIT_RANGE" 2>/dev/null || true)

if [[ -z "$COMMITS_DATA" ]]; then
    die "No commits found in range: $COMMIT_RANGE"
fi

COMMIT_COUNT=$(echo "$COMMITS_DATA" | wc -l | tr -d ' ')
log "Found $COMMIT_COUNT commits"

# Get timeline bounds
FIRST_TIMESTAMP=$(echo "$COMMITS_DATA" | head -1 | cut -d'|' -f1)
LAST_TIMESTAMP=$(echo "$COMMITS_DATA" | tail -1 | cut -d'|' -f1)

# -----------------------------------------------------------------------------
# Session Detection (Commit Burst Analysis)
# -----------------------------------------------------------------------------
log "Detecting sessions (gap > ${SESSION_GAP_MINUTES}m)..."

declare -a SESSIONS=()
declare -a SESSION_COMMIT_DATA=()

PREV_TIMESTAMP=""
CURRENT_SESSION_START=""
CURRENT_SESSION_END=""
CURRENT_SESSION_COMMITS=""
declare -A CURRENT_SESSION_FILES

new_session() {
    if [[ -n "$CURRENT_SESSION_START" && -n "$CURRENT_SESSION_COMMITS" ]]; then
        # Calculate metrics for the session
        local commit_count
        commit_count=$(echo "$CURRENT_SESSION_COMMITS" | wc -l | tr -d ' ')

        # Collect files
        local files_json="["
        local first=true
        for f in "${!CURRENT_SESSION_FILES[@]}"; do
            if [[ "$first" == "true" ]]; then
                files_json+="\"$f\""
                first=false
            else
                files_json+=", \"$f\""
            fi
        done
        files_json+="]"

        # Detect grind pattern
        local is_grind="false"
        if [[ $commit_count -ge $GRIND_COMMIT_RATE ]]; then
            for f in "${!CURRENT_SESSION_FILES[@]}"; do
                if [[ ${CURRENT_SESSION_FILES[$f]} -ge $GRIND_REPETITIVE_EDITS ]]; then
                    is_grind="true"
                    break
                fi
            done
        fi

        # Store session data
        SESSIONS+=("$CURRENT_SESSION_START|$CURRENT_SESSION_END|$commit_count|$files_json|$is_grind")
    fi
}

while IFS='|' read -r timestamp sha subject; do
    if [[ -n "$PREV_TIMESTAMP" ]]; then
        GAP=$((timestamp - PREV_TIMESTAMP))
        if [[ $GAP -gt $SESSION_GAP_SECONDS ]]; then
            # Gap exceeded - save current session and start new one
            CURRENT_SESSION_END="$PREV_TIMESTAMP"
            new_session

            # Reset for new session
            CURRENT_SESSION_START="$timestamp"
            CURRENT_SESSION_COMMITS=""
            declare -A CURRENT_SESSION_FILES=()
        fi
    else
        # First commit
        CURRENT_SESSION_START="$timestamp"
    fi

    # Add commit to current session
    CURRENT_SESSION_COMMITS+="$sha"$'\n'
    CURRENT_SESSION_END="$timestamp"

    # Track files changed in this commit
    while IFS= read -r file; do
        if [[ -n "$file" ]]; then
            CURRENT_SESSION_FILES["$file"]=$((${CURRENT_SESSION_FILES["$file"]:-0} + 1))
        fi
    done < <(git show --name-only --format="" "$sha" 2>/dev/null)

    PREV_TIMESTAMP="$timestamp"
done <<< "$COMMITS_DATA"

# Save final session
new_session

SESSION_COUNT=${#SESSIONS[@]}
log "Detected $SESSION_COUNT sessions"

# -----------------------------------------------------------------------------
# Friction Heatmap
# -----------------------------------------------------------------------------
log "Computing friction heatmap..."

declare -A FILE_COMMITS
declare -A FILE_CHURN
declare -A FILE_SESSION_SET

SESSION_IDX=0
PREV_TIMESTAMP=""
CURRENT_SESSION_END_TS=""

# First pass: determine session boundaries
declare -a SESSION_BOUNDS=()
for session in "${SESSIONS[@]}"; do
    IFS='|' read -r start end _ _ _ <<< "$session"
    SESSION_BOUNDS+=("$start|$end")
done

# Process each commit for friction metrics
while IFS='|' read -r timestamp sha subject; do
    # Determine which session this commit belongs to
    local_session_idx=0
    for i in "${!SESSION_BOUNDS[@]}"; do
        IFS='|' read -r s_start s_end <<< "${SESSION_BOUNDS[$i]}"
        if [[ $timestamp -ge $s_start && $timestamp -le $s_end ]]; then
            local_session_idx=$i
            break
        fi
    done

    # Get numstat for churn calculation
    while IFS=$'\t' read -r added deleted file; do
        if [[ -n "$file" && "$added" != "-" && "$deleted" != "-" ]]; then
            FILE_COMMITS["$file"]=$((${FILE_COMMITS["$file"]:-0} + 1))
            FILE_CHURN["$file"]=$((${FILE_CHURN["$file"]:-0} + added + deleted))
            # Track sessions (append to set)
            FILE_SESSION_SET["$file"]="${FILE_SESSION_SET["$file"]:-}|$local_session_idx"
        fi
    done < <(git show --numstat --format="" "$sha" 2>/dev/null)
done <<< "$COMMITS_DATA"

# Calculate unique session count per file
declare -A FILE_SESSION_COUNT
for file in "${!FILE_SESSION_SET[@]}"; do
    unique_sessions=$(echo "${FILE_SESSION_SET[$file]}" | tr '|' '\n' | sort -u | grep -v '^$' | wc -l | tr -d ' ')
    FILE_SESSION_COUNT["$file"]=$unique_sessions
done

# Sort files by friction score (commits * churn) and build output data
FRICTION_DATA=""
for file in "${!FILE_COMMITS[@]}"; do
    commits=${FILE_COMMITS[$file]}
    churn=${FILE_CHURN[$file]}
    sessions=${FILE_SESSION_COUNT[$file]:-1}
    friction_score=$((commits * churn))
    FRICTION_DATA+="$friction_score|$file|$commits|$churn|$sessions"$'\n'
done

FRICTION_SORTED=$(echo "$FRICTION_DATA" | sort -t'|' -k1 -nr | head -10)

# -----------------------------------------------------------------------------
# Oscillation Detection
# -----------------------------------------------------------------------------
log "Detecting oscillations..."

# Files with 3+ edits
FILES_3PLUS_EDITS=""
for file in "${!FILE_COMMITS[@]}"; do
    if [[ ${FILE_COMMITS[$file]} -ge 3 ]]; then
        FILES_3PLUS_EDITS+="\"$file\", "
    fi
done
FILES_3PLUS_EDITS="${FILES_3PLUS_EDITS%, }"  # Remove trailing comma

# Detect potential reverts
POTENTIAL_REVERTS=""
while IFS='|' read -r timestamp sha subject; do
    if is_revert "$subject"; then
        POTENTIAL_REVERTS+="\"${sha:0:8}\", "
    fi
done <<< "$COMMITS_DATA"
POTENTIAL_REVERTS="${POTENTIAL_REVERTS%, }"

# Check for test ignore/unignore patterns
TEST_TOGGLE_COMMITS=""
while IFS='|' read -r timestamp sha subject; do
    lower_subject="${subject,,}"
    if [[ "$lower_subject" =~ ignore || "$lower_subject" =~ skip || "$lower_subject" =~ disable ]]; then
        if [[ "$lower_subject" =~ test ]]; then
            TEST_TOGGLE_COMMITS+="\"${sha:0:8}\", "
        fi
    fi
done <<< "$COMMITS_DATA"
TEST_TOGGLE_COMMITS="${TEST_TOGGLE_COMMITS%, }"

# -----------------------------------------------------------------------------
# Stabilization Inflection Detection
# -----------------------------------------------------------------------------
log "Finding stabilization inflection point..."

INFLECTION_SHA=""
INFLECTION_INDEX=0
COMMITS_BEFORE=0
COMMITS_AFTER=0
LOGIC_FILES_AFTER=()

# Find the last commit that touched a logic file
COMMIT_INDEX=0
LAST_LOGIC_INDEX=0
LAST_LOGIC_SHA=""

while IFS='|' read -r timestamp sha subject; do
    COMMIT_INDEX=$((COMMIT_INDEX + 1))

    # Check if this commit touches logic files
    has_logic=false
    while IFS= read -r file; do
        if [[ -n "$file" ]] && is_logic_file "$file"; then
            has_logic=true
            break
        fi
    done < <(git show --name-only --format="" "$sha" 2>/dev/null)

    if [[ "$has_logic" == "true" ]]; then
        LAST_LOGIC_INDEX=$COMMIT_INDEX
        LAST_LOGIC_SHA="$sha"
    fi
done <<< "$COMMITS_DATA"

if [[ -n "$LAST_LOGIC_SHA" ]]; then
    INFLECTION_SHA="$LAST_LOGIC_SHA"
    COMMITS_BEFORE=$LAST_LOGIC_INDEX
    COMMITS_AFTER=$((COMMIT_COUNT - LAST_LOGIC_INDEX))

    # Check for any logic files touched after inflection (shouldn't be any by definition)
    # But let's verify and report if stabilization was imperfect
    COMMIT_INDEX=0
    while IFS='|' read -r timestamp sha subject; do
        COMMIT_INDEX=$((COMMIT_INDEX + 1))
        if [[ $COMMIT_INDEX -gt $LAST_LOGIC_INDEX ]]; then
            while IFS= read -r file; do
                if [[ -n "$file" ]] && is_logic_file "$file"; then
                    LOGIC_FILES_AFTER+=("$file")
                fi
            done < <(git show --name-only --format="" "$sha" 2>/dev/null)
        fi
    done <<< "$COMMITS_DATA"
fi

# Calculate stabilization ratio
if [[ $COMMITS_AFTER -gt 0 && $COMMITS_BEFORE -gt 0 ]]; then
    STABILIZATION_RATIO=$(awk "BEGIN {printf \"%.2f\", $COMMITS_BEFORE / $COMMITS_AFTER}")
else
    STABILIZATION_RATIO="N/A"
fi

# -----------------------------------------------------------------------------
# Commit Topology (Conventional Commits Classification)
# -----------------------------------------------------------------------------
log "Classifying commits by type..."

declare -A TOPOLOGY
TOPOLOGY[feat]=0
TOPOLOGY[fix]=0
TOPOLOGY[test]=0
TOPOLOGY[docs]=0
TOPOLOGY[chore]=0
TOPOLOGY[other]=0

while IFS='|' read -r timestamp sha subject; do
    commit_type=$(classify_commit "$subject")
    TOPOLOGY[$commit_type]=$((TOPOLOGY[$commit_type] + 1))
done <<< "$COMMITS_DATA"

# Calculate semantic ratio (feat + fix) / total
SEMANTIC_COUNT=$((TOPOLOGY[feat] + TOPOLOGY[fix]))
if [[ $COMMIT_COUNT -gt 0 ]]; then
    SEMANTIC_RATIO=$(awk "BEGIN {printf \"%.1f\", ($SEMANTIC_COUNT / $COMMIT_COUNT) * 100}")
else
    SEMANTIC_RATIO="0.0"
fi

# -----------------------------------------------------------------------------
# Generate YAML Output
# -----------------------------------------------------------------------------
generate_yaml() {
    local analyzed_at
    analyzed_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    cat <<EOF
# Temporal Analysis
# Generated by: scripts/forensics/temporal-analysis.sh
# See: docs/DEVLT_ESTIMATION.md

pr: ${PR_NUMBER:-"unknown"}
analyzed_at: "$analyzed_at"
session_gap_minutes: $SESSION_GAP_MINUTES

sessions:
EOF

    local session_id=1
    for session in "${SESSIONS[@]}"; do
        IFS='|' read -r start end commits files is_grind <<< "$session"
        cat <<EOF
  - id: $session_id
    start: "$(timestamp_to_iso "$start")"
    end: "$(timestamp_to_iso "$end")"
    commits: $commits
    files_touched: $files
    is_grind: $is_grind
EOF
        session_id=$((session_id + 1))
    done

    if [[ ${#SESSIONS[@]} -eq 0 ]]; then
        echo "  []"
    fi

    echo ""
    echo "friction_heatmap:"

    if [[ -n "$FRICTION_SORTED" ]]; then
        while IFS='|' read -r score path commits churn sessions; do
            if [[ -n "$path" ]]; then
                cat <<EOF
  - file: "$path"
    commits: $commits
    churn: $churn
    sessions: $sessions
EOF
            fi
        done <<< "$FRICTION_SORTED"
    else
        echo "  []"
    fi

    echo ""
    echo "oscillations:"

    if [[ -n "$FILES_3PLUS_EDITS" ]]; then
        echo "  files_with_3plus_edits: [$FILES_3PLUS_EDITS]"
    else
        echo "  files_with_3plus_edits: []"
    fi

    if [[ -n "$POTENTIAL_REVERTS" ]]; then
        echo "  potential_reverts: [$POTENTIAL_REVERTS]"
    else
        echo "  potential_reverts: []"
    fi

    echo ""
    echo "stabilization:"

    if [[ -n "$INFLECTION_SHA" ]]; then
        echo "  inflection_commit: \"${INFLECTION_SHA:0:8}\""
    else
        echo "  inflection_commit: null"
    fi

    echo "  commits_before: $COMMITS_BEFORE"
    echo "  commits_after: $COMMITS_AFTER"

    if [[ ${#LOGIC_FILES_AFTER[@]} -gt 0 ]]; then
        echo "  logic_files_after_inflection:"
        for f in "${LOGIC_FILES_AFTER[@]}"; do
            echo "    - \"$f\""
        done
    else
        echo "  logic_files_after_inflection: []"
    fi

    echo ""
    echo "commit_topology:"
    echo "  feat: ${TOPOLOGY[feat]}"
    echo "  fix: ${TOPOLOGY[fix]}"
    echo "  test: ${TOPOLOGY[test]}"
    echo "  docs: ${TOPOLOGY[docs]}"
    echo "  chore: ${TOPOLOGY[chore]}"
    echo "  other: ${TOPOLOGY[other]}"

    echo ""
    echo "semantic_ratio: $SEMANTIC_RATIO"
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
