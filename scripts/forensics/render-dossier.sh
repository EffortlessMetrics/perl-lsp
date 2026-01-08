#!/usr/bin/env bash
# Dossier Renderer: Synthesize forensics outputs into complete dossier markdown
#
# Reads YAML outputs from other forensics tools and renders a complete
# dossier following docs/FORENSICS_SCHEMA.md format.
#
# Usage:
#   ./scripts/forensics/render-dossier.sh 259
#   ./scripts/forensics/render-dossier.sh 259 -o docs/forensics/pr-259.md
#   ./scripts/forensics/render-dossier.sh 259 --cover-sheet
#   ./scripts/forensics/render-dossier.sh 259 --exhibit
#
# Input sources (if available):
#   - pr-harvest.yaml   - PR metadata and change surface (from pr-harvest.sh)
#   - temporal.yaml     - Convergence analysis (from temporal-analysis.sh)
#   - telemetry.yaml    - Static analysis deltas (future)
#
# See: docs/FORENSICS_SCHEMA.md for dossier structure
#      docs/CASEBOOK.md for exhibit format
#      docs/forensics/INDEX.md for cover sheet format

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# -----------------------------------------------------------------------------
# Usage
# -----------------------------------------------------------------------------
usage() {
    cat <<EOF
Usage: $(basename "$0") <PR_NUMBER> [OPTIONS]

Render a dossier from forensics data for a merged PR.

Arguments:
    PR_NUMBER           GitHub PR number (required)

Options:
    -o, --output FILE   Output to file instead of stdout
    --cover-sheet       Output only the cover sheet format (for pasting into PR)
    --exhibit           Output only the exhibit format (for CASEBOOK.md)
    --harvest FILE      Path to pr-harvest.yaml (default: ./pr-<N>-harvest.yaml)
    --temporal FILE     Path to temporal.yaml (default: ./pr-<N>-temporal.yaml)
    --telemetry FILE    Path to telemetry.yaml (default: ./pr-<N>-telemetry.yaml)
    -h, --help          Show this help

Examples:
    $(basename "$0") 259
    $(basename "$0") 259 -o docs/forensics/pr-259.md
    $(basename "$0") 259 --cover-sheet | pbcopy
    $(basename "$0") 259 --exhibit >> docs/CASEBOOK.md

Output: Markdown following docs/FORENSICS_SCHEMA.md template
EOF
    exit 0
}

# -----------------------------------------------------------------------------
# Helpers
# -----------------------------------------------------------------------------
log() {
    echo "[render-dossier] $*" >&2
}

die() {
    echo "[render-dossier] ERROR: $*" >&2
    exit 1
}

# Safe YAML field extraction (handles missing yq gracefully)
# Usage: yaml_get <file> <path>
# Returns empty string if file missing or field not found
# Compatible with both jq-style yq (kislyuk) and go-yq (mikefarah)
yaml_get() {
    local file="$1"
    local path="$2"

    if [[ ! -f "$file" ]]; then
        echo ""
        return
    fi

    if command -v yq &>/dev/null; then
        # mikefarah/yq (Go version) - most common
        # Returns "null" for missing keys, so we convert to empty
        local result
        result=$(yq -r "$path" "$file" 2>/dev/null) || result=""
        # Convert "null" to empty string
        if [[ "$result" == "null" ]]; then
            echo ""
        else
            echo "$result"
        fi
    else
        # Fallback: simple grep-based extraction for common patterns
        # This only works for simple top-level keys
        local key="${path##*.}"
        grep -E "^${key}:" "$file" 2>/dev/null | sed 's/^[^:]*:[[:space:]]*//' | tr -d '"' || echo ""
    fi
}

# Format date for display
format_date() {
    date -u +"%Y-%m-%d"
}

# Get current date in ISO format
current_date() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# Slugify a string for exhibit ID
slugify() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g' | sed -E 's/^-|-$//g'
}

# -----------------------------------------------------------------------------
# Parse arguments
# -----------------------------------------------------------------------------
PR_NUMBER=""
OUTPUT_FILE=""
FORMAT="full"  # full | cover-sheet | exhibit
HARVEST_FILE=""
TEMPORAL_FILE=""
TELEMETRY_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --cover-sheet)
            FORMAT="cover-sheet"
            shift
            ;;
        --exhibit)
            FORMAT="exhibit"
            shift
            ;;
        --harvest)
            HARVEST_FILE="$2"
            shift 2
            ;;
        --temporal)
            TEMPORAL_FILE="$2"
            shift 2
            ;;
        --telemetry)
            TELEMETRY_FILE="$2"
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

# Set default input file paths if not specified
[[ -z "$HARVEST_FILE" ]] && HARVEST_FILE="./pr-${PR_NUMBER}-harvest.yaml"
[[ -z "$TEMPORAL_FILE" ]] && TEMPORAL_FILE="./pr-${PR_NUMBER}-temporal.yaml"
[[ -z "$TELEMETRY_FILE" ]] && TELEMETRY_FILE="./pr-${PR_NUMBER}-telemetry.yaml"

log "Rendering dossier for PR #$PR_NUMBER (format: $FORMAT)"

# -----------------------------------------------------------------------------
# Load data from YAML sources
# -----------------------------------------------------------------------------

# Check which sources are available
HAS_HARVEST=false
HAS_TEMPORAL=false
HAS_TELEMETRY=false

[[ -f "$HARVEST_FILE" ]] && HAS_HARVEST=true
[[ -f "$TEMPORAL_FILE" ]] && HAS_TEMPORAL=true
[[ -f "$TELEMETRY_FILE" ]] && HAS_TELEMETRY=true

# Determine coverage level
COVERAGE="github_only"
if [[ "$HAS_HARVEST" == "true" && "$HAS_TEMPORAL" == "true" ]]; then
    COVERAGE="github_plus_temporal"
fi

# If we have no data sources at all, try to fetch minimal data from GitHub
if [[ "$HAS_HARVEST" != "true" ]]; then
    log "Warning: No harvest file found at $HARVEST_FILE"
    log "Attempting to fetch minimal data from GitHub..."

    if command -v gh &>/dev/null; then
        PR_JSON=$(gh pr view "$PR_NUMBER" --json number,title,url,author,body,labels,createdAt,mergedAt,files,additions,deletions 2>/dev/null) || {
            log "Warning: Could not fetch PR data from GitHub"
            PR_JSON=""
        }
    else
        PR_JSON=""
    fi
fi

# Extract data from sources
if [[ "$HAS_HARVEST" == "true" ]]; then
    TITLE=$(yaml_get "$HARVEST_FILE" '.pr.title')
    URL=$(yaml_get "$HARVEST_FILE" '.pr.url')
    AUTHOR=$(yaml_get "$HARVEST_FILE" '.pr.author')
    CREATED_AT=$(yaml_get "$HARVEST_FILE" '.pr.created_at')
    MERGED_AT=$(yaml_get "$HARVEST_FILE" '.pr.merged_at')
    BODY=$(yaml_get "$HARVEST_FILE" '.body')
    FILES_CHANGED=$(yaml_get "$HARVEST_FILE" '.change_surface.files_changed')
    INSERTIONS=$(yaml_get "$HARVEST_FILE" '.change_surface.insertions')
    DELETIONS=$(yaml_get "$HARVEST_FILE" '.change_surface.deletions')
    COMMIT_COUNT=$(yaml_get "$HARVEST_FILE" '.commits.count')
elif [[ -n "${PR_JSON:-}" ]]; then
    TITLE=$(echo "$PR_JSON" | jq -r '.title // ""')
    URL=$(echo "$PR_JSON" | jq -r '.url // ""')
    AUTHOR=$(echo "$PR_JSON" | jq -r '.author.login // ""')
    CREATED_AT=$(echo "$PR_JSON" | jq -r '.createdAt // ""')
    MERGED_AT=$(echo "$PR_JSON" | jq -r '.mergedAt // ""')
    BODY=$(echo "$PR_JSON" | jq -r '.body // ""')
    FILES_CHANGED=$(echo "$PR_JSON" | jq -r '.files | length')
    INSERTIONS=$(echo "$PR_JSON" | jq -r '.additions // 0')
    DELETIONS=$(echo "$PR_JSON" | jq -r '.deletions // 0')
    COMMIT_COUNT="unknown"
else
    TITLE="<PR title - run pr-harvest.sh first>"
    URL="https://github.com/<owner>/<repo>/pull/$PR_NUMBER"
    AUTHOR="<unknown>"
    CREATED_AT="<unknown>"
    MERGED_AT="<unknown>"
    BODY=""
    FILES_CHANGED="<unknown>"
    INSERTIONS="<unknown>"
    DELETIONS="<unknown>"
    COMMIT_COUNT="<unknown>"
fi

# Extract temporal data if available
if [[ "$HAS_TEMPORAL" == "true" ]]; then
    SESSION_COUNT=$(yaml_get "$TEMPORAL_FILE" '.sessions.count')
    PATTERN=$(yaml_get "$TEMPORAL_FILE" '.convergence.pattern')
    STABILIZATION=$(yaml_get "$TEMPORAL_FILE" '.convergence.stabilization_commit')
    WALL_CLOCK=$(yaml_get "$TEMPORAL_FILE" '.timeline.wall_clock_hours')
else
    SESSION_COUNT="<unknown>"
    PATTERN="<unknown - run temporal-analysis.sh>"
    STABILIZATION="<unknown>"
    WALL_CLOCK="<unknown>"
fi

# Generate exhibit ID from title
EXHIBIT_ID=$(slugify "${TITLE:-pr-$PR_NUMBER}")

# Extract linked issues from PR body (common patterns)
LINKED_ISSUES=""
if [[ -n "$BODY" ]]; then
    # Look for common patterns: "Fixes #123", "Closes #456", "Issue #789"
    LINKED_ISSUES=$(echo "$BODY" | grep -oE '(fixes|closes|resolves|addresses|issue)[[:space:]]*#[0-9]+' -i | grep -oE '#[0-9]+' | sort -u | tr '\n' ', ' | sed 's/,$//')
fi
[[ -z "$LINKED_ISSUES" ]] && LINKED_ISSUES="<none linked>"

# Extract stated goal from PR body (first non-empty line or ## Summary content)
STATED_GOAL=""
if [[ -n "$BODY" ]]; then
    # Try to find Summary section
    STATED_GOAL=$(echo "$BODY" | sed -n '/^##[[:space:]]*Summary/,/^##/p' | head -5 | tail -n +2 | head -1 | sed 's/^[[:space:]-]*//')

    # Fallback to first non-empty, non-header line
    if [[ -z "$STATED_GOAL" ]]; then
        STATED_GOAL=$(echo "$BODY" | grep -v '^#' | grep -v '^$' | head -1 | cut -c1-100)
    fi
fi
[[ -z "$STATED_GOAL" ]] && STATED_GOAL="<extract from PR body>"

# -----------------------------------------------------------------------------
# Render: Cover Sheet Format
# -----------------------------------------------------------------------------
render_cover_sheet() {
    cat <<EOF
## Cover sheet (added $(format_date); original notes below)

- **Issue(s):** $LINKED_ISSUES
- **PR:** #$PR_NUMBER
- **Exhibit ID:** \`$EXHIBIT_ID\`

### What changed
<summarize key changes from diff>

### Why
$STATED_GOAL

### Review map
<list key files touched with purpose>

### Verification (receipts)
<test output, CI results, gate output>

### Known limits / follow-ups
<note any deferred work or known gaps>

### How to reproduce trust
\`\`\`bash
# Command to verify the PR's claims
cargo test -p <crate> --test <test_name>
\`\`\`

### Quality Deltas

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | 0 | <placeholder - requires LLM analysis> |
| Correctness | 0 | <placeholder> |
| Governance | 0 | <placeholder> |
| Reproducibility | 0 | <placeholder> |

### Budget (with Provenance)

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | <X-Ym> | estimated; $COVERAGE; medium; <N decisions, N friction> |
| CI | <Zm> | estimated; local gate |
| LLM | ~<N> units | estimated; <N iterations> |
EOF
}

# -----------------------------------------------------------------------------
# Render: Exhibit Format (for CASEBOOK.md)
# -----------------------------------------------------------------------------
render_exhibit() {
    cat <<EOF
### Exhibit N: $TITLE (PR #$PR_NUMBER)

**What it proves:** <one-line summary of what this PR demonstrates>

**Review map:**
- <key file 1> (<delta summary>)
- <key file 2> (<delta summary>)

**Proof bundle:**
- <test output or gate output>
- <metrics or baselines>

**Scar story:** <N/A if clean, or: Wrong -> Caught -> Fix -> Prevention>

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | 0 | <placeholder - requires LLM analysis> |
| Correctness | 0 | <placeholder> |
| Governance | 0 | <placeholder> |
| Reproducibility | 0 | <placeholder> |

**Factory delta:**
- <systemic improvement 1>
- <systemic improvement 2>

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | <X-Ym> | estimated; $COVERAGE; medium; <N decisions, N friction> |
| CI | <Zm> | estimated; local gate |
| LLM | ~<N> units | estimated; <N iterations> |

**Exhibit score:** <X.X>/5 (Clarity: X, Scope: X, Evidence: X, Tests: X, Efficiency: X)

**Dossier:** [\`forensics/pr-$PR_NUMBER.md\`](forensics/pr-$PR_NUMBER.md)

---
EOF
}

# -----------------------------------------------------------------------------
# Render: Full Dossier Format
# -----------------------------------------------------------------------------
render_full_dossier() {
    cat <<EOF
# PR #$PR_NUMBER: $TITLE

**Archaeology Date**: $(format_date)
**URL**: $URL
**Author**: $AUTHOR

---

## Intent

| Field | Value |
|-------|-------|
| **Issue(s)** | $LINKED_ISSUES |
| **Stated goal** | $STATED_GOAL |
| **Actual scope** | +$INSERTIONS/-$DELETIONS across $FILES_CHANGED files |

---

## Scope Map

| Directory | Files | Delta | Notes |
|-----------|-------|-------|-------|
EOF

    # Try to extract directory-level summary from harvest data
    if [[ "$HAS_HARVEST" == "true" ]] && command -v yq &>/dev/null; then
        # Group hotspots by directory
        yq -r '.change_surface.hotspots[] | "\(.path)|\(.insertions)|\(.deletions)"' "$HARVEST_FILE" 2>/dev/null | \
        while IFS='|' read -r path add del; do
            dir=$(dirname "$path")
            echo "| \`$dir/\` | 1+ | +$add/-$del | |"
        done | sort -u | head -10
    else
        cat <<EOF
| \`<directory>/\` | <N> | +<X>/-<Y> | <notes> |
| \`<directory>/\` | <N> | +<X>/-<Y> | <notes> |
EOF
    fi

    cat <<EOF

---

## Evidence Pointers

| Type | Link/Value | Notes |
|------|------------|-------|
| CI | <workflow run URL if available> | |
| Gate | <local gate output excerpt> | |
| Tests | <pass/fail summary> | |

---

## Findings

| Category | Finding | Severity | Evidence |
|----------|---------|----------|----------|
| <category> | <finding> | P1/P2/P3 | <evidence> |

_Severity: P1=blocks correctness, P2=causes confusion, P3=cleanup opportunity_

---

## Quality Deltas

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | 0 | <placeholder - requires LLM analysis> |
| Correctness | 0 | <placeholder> |
| Governance | 0 | <placeholder> |
| Reproducibility | 0 | <placeholder> |

_Delta scale: +2=significant improvement, +1=minor improvement, 0=no change, -1=minor regression, -2=significant regression_

---

## Budget (with Provenance)

| Metric | Value | Kind | Confidence | Basis |
|--------|-------|------|------------|-------|
EOF

    # Generate DevLT estimate based on session count and pattern
    if [[ "$HAS_TEMPORAL" == "true" && "$SESSION_COUNT" != "null" && "$SESSION_COUNT" != "" ]]; then
        # Rough DevLT bands based on session count
        case "$SESSION_COUNT" in
            1) DEVLT_RANGE="15-45m" ;;
            2) DEVLT_RANGE="30-90m" ;;
            3|4) DEVLT_RANGE="60-120m" ;;
            *) DEVLT_RANGE="90-180m" ;;
        esac
        echo "| DevLT | $DEVLT_RANGE | estimated | medium | $SESSION_COUNT sessions, pattern: $PATTERN |"
    else
        echo "| DevLT | <X-Ym> | estimated | medium | <from temporal analysis> |"
    fi

    cat <<EOF
| CI | <Zm> | estimated | low | local gate |
| LLM | ~<N> units | estimated | medium | <iteration count> |

**Coverage**: \`$COVERAGE\`

---

## Factory Delta

| Guardrail | Before | After | Notes |
|-----------|--------|-------|-------|
| <guardrail> | <before state> | <after state> | |

_What systemic improvement resulted from this PR?_

---

## Temporal Profile

EOF

    if [[ "$HAS_TEMPORAL" == "true" ]]; then
        cat <<EOF
| Metric | Value |
|--------|-------|
| **Sessions** | $SESSION_COUNT |
| **Pattern** | $PATTERN |
| **Wall clock** | ${WALL_CLOCK}h |
| **Stabilization** | ${STABILIZATION:-none} |
| **Commits** | $COMMIT_COUNT |
EOF
    else
        cat <<EOF
_Run \`temporal-analysis.sh --pr $PR_NUMBER\` to populate this section._

| Metric | Value |
|--------|-------|
| **Sessions** | <unknown> |
| **Pattern** | <unknown> |
| **Stabilization** | <unknown> |
EOF
    fi

    cat <<EOF

---

## Exhibit Score

| Dimension | Score (1-5) | Notes |
|-----------|-------------|-------|
| Clarity of intent | _ | Was the goal clear? |
| Scope discipline | _ | Did it stay in scope? |
| Evidence quality | _ | Were claims backed? |
| Test coverage | _ | Did tests match claims? |
| DevLT efficiency | _ | Human time well spent? |

---

## Data Sources

| Source | Available | Path |
|--------|-----------|------|
| pr-harvest.yaml | $HAS_HARVEST | $HARVEST_FILE |
| temporal.yaml | $HAS_TEMPORAL | $TEMPORAL_FILE |
| telemetry.yaml | $HAS_TELEMETRY | $TELEMETRY_FILE |

_Generated: $(current_date) by render-dossier.sh_
EOF
}

# -----------------------------------------------------------------------------
# Output
# -----------------------------------------------------------------------------
render_output() {
    case "$FORMAT" in
        cover-sheet)
            render_cover_sheet
            ;;
        exhibit)
            render_exhibit
            ;;
        full)
            render_full_dossier
            ;;
    esac
}

if [[ -n "$OUTPUT_FILE" ]]; then
    render_output > "$OUTPUT_FILE"
    log "Output written to: $OUTPUT_FILE"
else
    render_output
fi

# Summary to stderr
log "Done. Coverage: $COVERAGE"
if [[ "$HAS_HARVEST" != "true" ]]; then
    log "Tip: Run pr-harvest.sh $PR_NUMBER first for richer data"
fi
if [[ "$HAS_TEMPORAL" != "true" ]]; then
    log "Tip: Run temporal-analysis.sh --pr $PR_NUMBER for convergence data"
fi
