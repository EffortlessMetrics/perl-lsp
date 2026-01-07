#!/usr/bin/env bash
# Temporal forensics: analyze commit patterns to understand convergence
#
# Usage:
#   ./scripts/forensics/temporal-analysis.sh base..head
#   ./scripts/forensics/temporal-analysis.sh --pr 123
#   ./scripts/forensics/temporal-analysis.sh HEAD~10..HEAD
#
# Computes:
#   - Session detection (gap > 30 min = new session)
#   - Burst/grind analysis (high commit rate, same files repeatedly)
#   - Friction heatmap (files with highest churn)
#   - Oscillation markers (reverts, repeated edits)
#   - Stabilization point (when logic files stop changing)
#
# Output: YAML to stdout
#
# See: docs/DEVLT_ESTIMATION.md for context on decision/friction events

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
SESSION_GAP_SECONDS=1800  # 30 minutes = new session
GRIND_THRESHOLD_COMMITS=5  # commits per session for "grind" detection
GRIND_THRESHOLD_SAME_FILE=3  # same file touched N+ times = grind indicator

# Parse arguments
COMMIT_RANGE=""
PR_NUMBER=""

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS] <commit-range>

Analyze commit patterns to understand how a feature converged.

Arguments:
  commit-range    Git revision range (e.g., base..head, HEAD~10..HEAD)

Options:
  --pr NUMBER     Fetch commit range from GitHub PR
  -h, --help      Show this help message

Examples:
  $(basename "$0") origin/master..HEAD
  $(basename "$0") --pr 251
  $(basename "$0") abc1234..def5678

Output: YAML structure with timeline, sessions, friction, oscillations, convergence
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --pr)
            PR_NUMBER="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            COMMIT_RANGE="$1"
            shift
            ;;
    esac
done

# If PR specified, fetch the commit range
if [[ -n "$PR_NUMBER" ]]; then
    if ! command -v gh &>/dev/null; then
        echo "Error: gh CLI required for --pr option" >&2
        exit 1
    fi
    # Get base and head refs from PR
    PR_INFO=$(gh pr view "$PR_NUMBER" --json baseRefName,headRefName,commits 2>/dev/null) || {
        echo "Error: Could not fetch PR #$PR_NUMBER" >&2
        exit 1
    }
    BASE_REF=$(echo "$PR_INFO" | jq -r '.baseRefName')
    HEAD_REF=$(echo "$PR_INFO" | jq -r '.headRefName')
    COMMIT_RANGE="origin/${BASE_REF}..origin/${HEAD_REF}"
fi

if [[ -z "$COMMIT_RANGE" ]]; then
    echo "Error: Commit range required" >&2
    echo "Usage: $(basename "$0") <base..head> or --pr <number>" >&2
    exit 1
fi

cd "$REPO_ROOT"

# Validate the commit range
if ! git rev-parse "${COMMIT_RANGE%%..*}" &>/dev/null 2>&1; then
    echo "Error: Invalid commit range: $COMMIT_RANGE" >&2
    exit 1
fi

# =============================================================================
# Data Collection
# =============================================================================

# Get all commits in range with timestamp, sha, message
# Format: epoch_timestamp|sha|subject
COMMITS_DATA=$(git log --reverse --format="%at|%H|%s" "$COMMIT_RANGE" 2>/dev/null)

if [[ -z "$COMMITS_DATA" ]]; then
    echo "Error: No commits found in range: $COMMIT_RANGE" >&2
    exit 1
fi

COMMIT_COUNT=$(echo "$COMMITS_DATA" | wc -l | tr -d ' ')

# Get first and last timestamps
FIRST_TIMESTAMP=$(echo "$COMMITS_DATA" | head -1 | cut -d'|' -f1)
LAST_TIMESTAMP=$(echo "$COMMITS_DATA" | tail -1 | cut -d'|' -f1)

# Convert to ISO8601
timestamp_to_iso() {
    if [[ "$(uname)" == "Darwin" ]]; then
        date -r "$1" -u +"%Y-%m-%dT%H:%M:%SZ"
    else
        date -u -d "@$1" +"%Y-%m-%dT%H:%M:%SZ"
    fi
}

START_ISO=$(timestamp_to_iso "$FIRST_TIMESTAMP")
END_ISO=$(timestamp_to_iso "$LAST_TIMESTAMP")
WALL_CLOCK_SECONDS=$((LAST_TIMESTAMP - FIRST_TIMESTAMP))
WALL_CLOCK_HOURS=$(awk "BEGIN {printf \"%.2f\", $WALL_CLOCK_SECONDS / 3600}")

# =============================================================================
# Session Detection
# =============================================================================

declare -a SESSION_STARTS=()
declare -a SESSION_ENDS=()
declare -a SESSION_COMMITS=()
declare -a SESSION_FILES=()
declare -a SESSION_IS_GRIND=()

PREV_TIMESTAMP=""
CURRENT_SESSION_START=""
CURRENT_SESSION_COMMITS=0
declare -A CURRENT_SESSION_FILES

process_session() {
    if [[ -n "$CURRENT_SESSION_START" ]]; then
        SESSION_STARTS+=("$CURRENT_SESSION_START")
        SESSION_ENDS+=("$PREV_TIMESTAMP")
        SESSION_COMMITS+=("$CURRENT_SESSION_COMMITS")

        # Collect unique files
        FILES_LIST=""
        for f in "${!CURRENT_SESSION_FILES[@]}"; do
            if [[ -n "$FILES_LIST" ]]; then
                FILES_LIST="$FILES_LIST, \"$f\""
            else
                FILES_LIST="\"$f\""
            fi
        done
        SESSION_FILES+=("[$FILES_LIST]")

        # Check for grind pattern
        IS_GRIND="false"
        if [[ $CURRENT_SESSION_COMMITS -ge $GRIND_THRESHOLD_COMMITS ]]; then
            for f in "${!CURRENT_SESSION_FILES[@]}"; do
                if [[ ${CURRENT_SESSION_FILES[$f]} -ge $GRIND_THRESHOLD_SAME_FILE ]]; then
                    IS_GRIND="true"
                    break
                fi
            done
        fi
        SESSION_IS_GRIND+=("$IS_GRIND")
    fi
}

while IFS='|' read -r timestamp sha subject; do
    if [[ -n "$PREV_TIMESTAMP" ]]; then
        GAP=$((timestamp - PREV_TIMESTAMP))
        if [[ $GAP -gt $SESSION_GAP_SECONDS ]]; then
            # New session - save current one
            process_session
            CURRENT_SESSION_START="$timestamp"
            CURRENT_SESSION_COMMITS=0
            declare -A CURRENT_SESSION_FILES=()
        fi
    else
        CURRENT_SESSION_START="$timestamp"
    fi

    CURRENT_SESSION_COMMITS=$((CURRENT_SESSION_COMMITS + 1))

    # Get files changed in this commit
    while IFS= read -r file; do
        if [[ -n "$file" ]]; then
            CURRENT_SESSION_FILES["$file"]=$((${CURRENT_SESSION_FILES["$file"]:-0} + 1))
        fi
    done < <(git show --name-only --format="" "$sha" 2>/dev/null)

    PREV_TIMESTAMP="$timestamp"
done <<< "$COMMITS_DATA"

# Save final session
process_session

SESSION_COUNT=${#SESSION_STARTS[@]}

# =============================================================================
# Friction Heatmap
# =============================================================================

# Get file-level stats: commits touching file, total churn, sessions touched
declare -A FILE_COMMITS
declare -A FILE_CHURN
declare -A FILE_SESSIONS

# Process each commit for file stats
while IFS='|' read -r timestamp sha subject; do
    # Determine which session this commit belongs to
    SESSION_IDX=0
    for i in "${!SESSION_STARTS[@]}"; do
        if [[ $timestamp -ge ${SESSION_STARTS[$i]} && $timestamp -le ${SESSION_ENDS[$i]} ]]; then
            SESSION_IDX=$i
            break
        fi
    done

    # Get numstat for churn calculation
    while IFS=$'\t' read -r added deleted file; do
        if [[ -n "$file" && "$added" != "-" && "$deleted" != "-" ]]; then
            FILE_COMMITS["$file"]=$((${FILE_COMMITS["$file"]:-0} + 1))
            FILE_CHURN["$file"]=$((${FILE_CHURN["$file"]:-0} + added + deleted))
            # Track sessions (use string concatenation and unique later)
            FILE_SESSIONS["$file"]="${FILE_SESSIONS["$file"]:-}|$SESSION_IDX"
        fi
    done < <(git show --numstat --format="" "$sha" 2>/dev/null)
done <<< "$COMMITS_DATA"

# Calculate unique sessions per file
declare -A FILE_SESSION_COUNT
for file in "${!FILE_SESSIONS[@]}"; do
    UNIQUE_SESSIONS=$(echo "${FILE_SESSIONS[$file]}" | tr '|' '\n' | sort -u | grep -v '^$' | wc -l | tr -d ' ')
    FILE_SESSION_COUNT["$file"]=$UNIQUE_SESSIONS
done

# Sort files by churn (commits * lines_changed)
FRICTION_DATA=""
for file in "${!FILE_COMMITS[@]}"; do
    COMMITS=${FILE_COMMITS[$file]}
    CHURN=${FILE_CHURN[$file]}
    SESSIONS_TOUCHED=${FILE_SESSION_COUNT[$file]:-1}
    # Friction score = commits * churn
    FRICTION_SCORE=$((COMMITS * CHURN))
    FRICTION_DATA+="$FRICTION_SCORE|$file|$COMMITS|$CHURN|$SESSIONS_TOUCHED"$'\n'
done

# Sort by friction score (descending) and take top 10
FRICTION_SORTED=$(echo "$FRICTION_DATA" | sort -t'|' -k1 -nr | head -10)

# =============================================================================
# Oscillation Detection
# =============================================================================

# Count reverts (grep may return 1 if no matches, so use || true)
REVERT_COUNT=$(echo "$COMMITS_DATA" | { grep -i '|revert' || true; } | wc -l | tr -d ' ')

# Find files with repeated edits (edited in 3+ commits)
REPEATED_EDITS=""
for file in "${!FILE_COMMITS[@]}"; do
    if [[ ${FILE_COMMITS[$file]} -ge 3 ]]; then
        if [[ -n "$REPEATED_EDITS" ]]; then
            REPEATED_EDITS+=$'\n'
        fi
        REPEATED_EDITS+="${FILE_COMMITS[$file]}|$file"
    fi
done
REPEATED_EDITS=$(echo "$REPEATED_EDITS" | sort -t'|' -k1 -nr)

# Detect dependency churn (Cargo.toml, package.json changes)
DEP_CHURN=""
for depfile in Cargo.toml Cargo.lock package.json package-lock.json; do
    if [[ -n "${FILE_COMMITS[$depfile]:-}" ]]; then
        if [[ ${FILE_COMMITS[$depfile]} -ge 2 ]]; then
            if [[ -n "$DEP_CHURN" ]]; then
                DEP_CHURN+=$'\n'
            fi
            DEP_CHURN+="${FILE_COMMITS[$depfile]}|$depfile"
        fi
    fi
done

# =============================================================================
# Convergence / Stabilization Detection
# =============================================================================

# Logic files: anything not in docs/, tests, or .md files
# Find the last commit that touched a "logic" file
STABILIZATION_SHA=""
STABILIZATION_PCT="0.0"
LOGIC_COMMIT_COUNT=0
TOTAL_COMMITS_PROCESSED=0

while IFS='|' read -r timestamp sha subject; do
    TOTAL_COMMITS_PROCESSED=$((TOTAL_COMMITS_PROCESSED + 1))

    # Check if this commit touches logic files
    HAS_LOGIC=false
    while IFS= read -r file; do
        # Skip docs, tests, markdown
        if [[ "$file" =~ ^docs/ || "$file" =~ ^test || "$file" =~ \.md$ || "$file" =~ /tests/ ]]; then
            continue
        fi
        # Skip pure config files for stabilization purposes
        if [[ "$file" =~ ^\.github/ || "$file" == ".gitignore" ]]; then
            continue
        fi
        # This is a logic file
        HAS_LOGIC=true
        break
    done < <(git show --name-only --format="" "$sha" 2>/dev/null)

    if [[ "$HAS_LOGIC" == "true" ]]; then
        STABILIZATION_SHA="$sha"
        LOGIC_COMMIT_COUNT=$((LOGIC_COMMIT_COUNT + 1))
    fi
done <<< "$COMMITS_DATA"

# Calculate what percentage of commits came after stabilization
if [[ -n "$STABILIZATION_SHA" && $COMMIT_COUNT -gt 0 ]]; then
    # Find index of stabilization commit
    STAB_IDX=0
    IDX=0
    while IFS='|' read -r timestamp sha subject; do
        IDX=$((IDX + 1))
        if [[ "$sha" == "$STABILIZATION_SHA" ]]; then
            STAB_IDX=$IDX
            break
        fi
    done <<< "$COMMITS_DATA"

    COMMITS_AFTER=$((COMMIT_COUNT - STAB_IDX))
    if [[ $COMMIT_COUNT -gt 0 ]]; then
        STABILIZATION_PCT=$(awk "BEGIN {printf \"%.1f\", ($COMMITS_AFTER / $COMMIT_COUNT) * 100}")
    fi
fi

# Determine convergence pattern
PATTERN="linear"
if [[ $REVERT_COUNT -gt 0 ]]; then
    PATTERN="oscillating"
fi
# Count files with 5+ edits as indicator of chaos
CHAOTIC_FILES=0
for file in "${!FILE_COMMITS[@]}"; do
    if [[ ${FILE_COMMITS[$file]} -ge 5 ]]; then
        CHAOTIC_FILES=$((CHAOTIC_FILES + 1))
    fi
done
if [[ $CHAOTIC_FILES -ge 3 ]]; then
    PATTERN="chaotic"
elif [[ $REVERT_COUNT -gt 0 || $CHAOTIC_FILES -ge 1 ]]; then
    PATTERN="oscillating"
fi

# =============================================================================
# Commit Topology (conventional commits categorization)
# =============================================================================

declare -A TOPOLOGY
TOPOLOGY[feat]=0
TOPOLOGY[fix]=0
TOPOLOGY[test]=0
TOPOLOGY[docs]=0
TOPOLOGY[chore]=0
TOPOLOGY[refactor]=0
TOPOLOGY[other]=0

while IFS='|' read -r timestamp sha subject; do
    LOWER_SUBJECT="${subject,,}"

    if [[ "$LOWER_SUBJECT" =~ ^feat[:\(] || "$LOWER_SUBJECT" =~ ^feature[:\(] ]]; then
        TOPOLOGY[feat]=$((TOPOLOGY[feat] + 1))
    elif [[ "$LOWER_SUBJECT" =~ ^fix[:\(] || "$LOWER_SUBJECT" =~ ^bugfix[:\(] ]]; then
        TOPOLOGY[fix]=$((TOPOLOGY[fix] + 1))
    elif [[ "$LOWER_SUBJECT" =~ ^test[:\(] || "$LOWER_SUBJECT" =~ ^tests[:\(] ]]; then
        TOPOLOGY[test]=$((TOPOLOGY[test] + 1))
    elif [[ "$LOWER_SUBJECT" =~ ^docs[:\(] || "$LOWER_SUBJECT" =~ ^doc[:\(] ]]; then
        TOPOLOGY[docs]=$((TOPOLOGY[docs] + 1))
    elif [[ "$LOWER_SUBJECT" =~ ^chore[:\(] || "$LOWER_SUBJECT" =~ ^build[:\(] || "$LOWER_SUBJECT" =~ ^ci[:\(] ]]; then
        TOPOLOGY[chore]=$((TOPOLOGY[chore] + 1))
    elif [[ "$LOWER_SUBJECT" =~ ^refactor[:\(] ]]; then
        TOPOLOGY[refactor]=$((TOPOLOGY[refactor] + 1))
    else
        TOPOLOGY[other]=$((TOPOLOGY[other] + 1))
    fi
done <<< "$COMMITS_DATA"

# =============================================================================
# YAML Output
# =============================================================================

echo "# Temporal Forensics Analysis"
echo "# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "# Range: $COMMIT_RANGE"
echo ""
echo "timeline:"
echo "  start: \"$START_ISO\""
echo "  end: \"$END_ISO\""
echo "  wall_clock_hours: $WALL_CLOCK_HOURS"
echo "  total_commits: $COMMIT_COUNT"
echo ""
echo "sessions:"
echo "  count: $SESSION_COUNT"
echo "  list:"

for i in "${!SESSION_STARTS[@]}"; do
    START_TS=${SESSION_STARTS[$i]}
    END_TS=${SESSION_ENDS[$i]}
    echo "    - start: \"$(timestamp_to_iso "$START_TS")\""
    echo "      end: \"$(timestamp_to_iso "$END_TS")\""
    echo "      commits: ${SESSION_COMMITS[$i]}"
    echo "      files_touched: ${SESSION_FILES[$i]}"
    echo "      is_grind: ${SESSION_IS_GRIND[$i]}"
done

echo ""
echo "friction_heatmap:"

if [[ -n "$FRICTION_SORTED" ]]; then
    while IFS='|' read -r score path commits churn sessions; do
        if [[ -n "$path" ]]; then
            echo "  - path: \"$path\""
            echo "    commits: $commits"
            echo "    total_churn: $churn"
            echo "    sessions_touched: $sessions"
        fi
    done <<< "$FRICTION_SORTED"
else
    echo "  []"
fi

echo ""
echo "oscillations:"
echo "  reverts: $REVERT_COUNT"
echo "  repeated_edits:"

if [[ -n "$REPEATED_EDITS" ]]; then
    while IFS='|' read -r count path; do
        if [[ -n "$path" ]]; then
            echo "    - path: \"$path\""
            echo "      count: $count"
        fi
    done <<< "$REPEATED_EDITS"
else
    echo "    []"
fi

echo "  dep_churn:"
if [[ -n "$DEP_CHURN" ]]; then
    while IFS='|' read -r count path; do
        if [[ -n "$path" ]]; then
            echo "    - path: \"$path\""
            echo "      count: $count"
        fi
    done <<< "$DEP_CHURN"
else
    echo "    []"
fi

echo ""
echo "convergence:"
if [[ -n "$STABILIZATION_SHA" ]]; then
    echo "  stabilization_commit: \"${STABILIZATION_SHA:0:8}\""
else
    echo "  stabilization_commit: null"
fi
echo "  stabilization_pct: $STABILIZATION_PCT"
echo "  pattern: $PATTERN"

echo ""
echo "commit_topology:"
echo "  feat: ${TOPOLOGY[feat]}"
echo "  fix: ${TOPOLOGY[fix]}"
echo "  test: ${TOPOLOGY[test]}"
echo "  docs: ${TOPOLOGY[docs]}"
echo "  chore: ${TOPOLOGY[chore]}"
echo "  refactor: ${TOPOLOGY[refactor]}"
echo "  other: ${TOPOLOGY[other]}"
