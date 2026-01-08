#!/usr/bin/env bash
# dossier-runner.sh - Orchestrate full PR forensics analysis
#
# Usage:
#   ./dossier-runner.sh <PR_NUMBER> [--mode quick|full] [--output DIR]
#
# This script runs the complete forensics pipeline:
# 1. pr-harvest.sh      - Pull PR facts from GitHub
# 2. temporal-analysis.sh - Compute commit topology
# 3. telemetry-runner.sh  - Run static analysis (base→head deltas)
# 4. Render dossier markdown
#
# Output: docs/forensics/pr-<number>.md (or custom output dir)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Defaults
MODE="quick"
OUTPUT_DIR=""
PR_NUMBER=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --output|-o)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 <PR_NUMBER> [--mode quick|full] [--output DIR]"
            echo ""
            echo "Options:"
            echo "  --mode       Analysis mode: 'quick' (always-on tools) or 'full' (exhibit-grade)"
            echo "  --output     Output directory (default: docs/forensics)"
            echo ""
            echo "Example:"
            echo "  $0 275 --mode full"
            exit 0
            ;;
        *)
            if [[ -z "$PR_NUMBER" ]]; then
                PR_NUMBER="$1"
            else
                echo "Unknown argument: $1" >&2
                exit 1
            fi
            shift
            ;;
    esac
done

if [[ -z "$PR_NUMBER" ]]; then
    echo "Error: PR number required" >&2
    echo "Usage: $0 <PR_NUMBER> [--mode quick|full] [--output DIR]" >&2
    exit 1
fi

# Set output directory
if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="$REPO_ROOT/docs/forensics"
fi
mkdir -p "$OUTPUT_DIR"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

echo "=== PR Forensics Dossier: #$PR_NUMBER ===" >&2
echo "Mode: $MODE" >&2
echo "Output: $OUTPUT_DIR" >&2
echo "" >&2

# Step 1: Harvest PR facts
echo "[1/4] Harvesting PR facts..." >&2
if [[ -x "$SCRIPT_DIR/pr-harvest.sh" ]]; then
    "$SCRIPT_DIR/pr-harvest.sh" "$PR_NUMBER" -o "$WORK_DIR/harvest" >&2
    HARVEST_DIR="$WORK_DIR/harvest/pr-$PR_NUMBER"
else
    echo "  Warning: pr-harvest.sh not found, creating minimal harvest" >&2
    HARVEST_DIR="$WORK_DIR/harvest/pr-$PR_NUMBER"
    mkdir -p "$HARVEST_DIR"

    # Minimal harvest using gh directly
    gh pr view "$PR_NUMBER" --json number,title,url,state,createdAt,mergedAt,author,labels,body,baseRefOid,headRefOid > "$HARVEST_DIR/metadata.json"
    gh pr view "$PR_NUMBER" --json commits > "$HARVEST_DIR/commits.json"
    gh pr view "$PR_NUMBER" --json files > "$HARVEST_DIR/files.json"
    gh pr view "$PR_NUMBER" --json comments,reviews > "$HARVEST_DIR/comments.json"
fi

# Extract base and head SHAs using shared library
# Source lib_gh.sh for SHA extraction utilities (handles gh CLI version differences)
source "$SCRIPT_DIR/lib_gh.sh"
read -r BASE_SHA HEAD_SHA MERGE_COMMIT < <(extract_shas_from_file "$HARVEST_DIR/metadata.json")
PR_TITLE=$(jq -r '.title // "Unknown"' "$HARVEST_DIR/metadata.json" 2>/dev/null || echo "Unknown")
PR_STATE=$(jq -r '.state // "unknown"' "$HARVEST_DIR/metadata.json" 2>/dev/null || echo "unknown")
CREATED_AT=$(jq -r '.createdAt // ""' "$HARVEST_DIR/metadata.json" 2>/dev/null || echo "")
MERGED_AT=$(jq -r '.mergedAt // ""' "$HARVEST_DIR/metadata.json" 2>/dev/null || echo "")

echo "  Title: $PR_TITLE" >&2
echo "  State: $PR_STATE" >&2
echo "  Base: ${BASE_SHA:0:8}" >&2
echo "  Head: ${HEAD_SHA:0:8}" >&2

# Step 2: Temporal analysis
echo "[2/4] Running temporal analysis..." >&2
TEMPORAL_OUTPUT="$WORK_DIR/temporal.yaml"
if [[ -x "$SCRIPT_DIR/temporal-analysis.sh" ]]; then
    "$SCRIPT_DIR/temporal-analysis.sh" "$HARVEST_DIR" > "$TEMPORAL_OUTPUT" 2>/dev/null || {
        echo "  Warning: temporal analysis failed, creating placeholder" >&2
        echo "# Temporal analysis not available" > "$TEMPORAL_OUTPUT"
    }
else
    echo "  Warning: temporal-analysis.sh not found, skipping" >&2
    echo "# Temporal analysis not available" > "$TEMPORAL_OUTPUT"
fi

# Step 3: Telemetry (if we have valid SHAs and they exist in repo)
echo "[3/4] Running telemetry analysis ($MODE mode)..." >&2
TELEMETRY_OUTPUT="$WORK_DIR/telemetry.yaml"
if [[ -n "$BASE_SHA" ]] && [[ -n "$HEAD_SHA" ]]; then
    if git cat-file -e "$BASE_SHA" 2>/dev/null && git cat-file -e "$HEAD_SHA" 2>/dev/null; then
        if [[ -x "$SCRIPT_DIR/telemetry-runner.sh" ]]; then
            # Map MODE to telemetry-runner.sh flag (telemetry uses --quick/--full/--research, not --mode)
            telemetry_flag="--quick"
            case "$MODE" in
                quick) telemetry_flag="--quick" ;;
                full) telemetry_flag="--full" ;;
                research) telemetry_flag="--research" ;;
                *) telemetry_flag="--quick" ;;
            esac

            "$SCRIPT_DIR/telemetry-runner.sh" "$telemetry_flag" "$BASE_SHA" "$HEAD_SHA" > "$TELEMETRY_OUTPUT" 2>/dev/null || {
                echo "  Warning: telemetry analysis failed" >&2
                echo "# Telemetry not available" > "$TELEMETRY_OUTPUT"
            }
        else
            echo "  Warning: telemetry-runner.sh not found, running basic checks" >&2
            # Run basic checks inline
            cat > "$TELEMETRY_OUTPUT" << EOF
# Basic telemetry (telemetry-runner.sh not available)
pr: $PR_NUMBER
base_sha: $BASE_SHA
head_sha: $HEAD_SHA
mode: $MODE
analyzed_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
tools:
  note: Full telemetry requires telemetry-runner.sh
EOF
        fi
    else
        echo "  Warning: Base or head SHA not found in local repo" >&2
        echo "# SHAs not available locally" > "$TELEMETRY_OUTPUT"
    fi
else
    echo "  Warning: Missing base/head SHA, skipping telemetry" >&2
    echo "# No base/head SHA available" > "$TELEMETRY_OUTPUT"
fi

# Step 4: Render dossier
echo "[4/4] Rendering dossier..." >&2

DOSSIER_FILE="$OUTPUT_DIR/pr-$PR_NUMBER.md"

# Get file list for review map
FILES_CHANGED=""
if [[ -f "$HARVEST_DIR/files.json" ]]; then
    FILES_CHANGED=$(jq -r '.files[].path' "$HARVEST_DIR/files.json" 2>/dev/null | head -20 || echo "")
fi

# Get commit count
COMMIT_COUNT=$(jq -r '.commits | length' "$HARVEST_DIR/commits.json" 2>/dev/null || echo "0")

# Render the dossier
cat > "$DOSSIER_FILE" << EOF
# PR #$PR_NUMBER: $PR_TITLE

**Generated:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**State:** $PR_STATE
**Created:** $CREATED_AT
**Merged:** ${MERGED_AT:-N/A}
**Base:** \`${BASE_SHA:0:8}\`
**Head:** \`${HEAD_SHA:0:8}\`
**Commits:** $COMMIT_COUNT

---

## Review Map

### Files Changed

\`\`\`
$FILES_CHANGED
\`\`\`

---

## Quality Deltas

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | TBD | *Requires design-auditor analysis* |
| Correctness | TBD | *Requires verification-auditor analysis* |
| Governance | TBD | *Requires policy-auditor analysis* |
| Reproducibility | TBD | *Requires docs-auditor analysis* |

---

## Budget

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | TBD | *Requires decision-extractor analysis* |
| CI | TBD | *From telemetry or workflow runs* |
| LLM | TBD | *From agent logs if available* |

**Coverage:** github_only (no agent logs)
**Confidence:** low (automated harvest only)

---

## Temporal Analysis

\`\`\`yaml
$(cat "$TEMPORAL_OUTPUT")
\`\`\`

---

## Telemetry

\`\`\`yaml
$(cat "$TELEMETRY_OUTPUT")
\`\`\`

---

## Raw Harvest

### PR Metadata

\`\`\`json
$(cat "$HARVEST_DIR/metadata.json" 2>/dev/null | head -100 || echo '{}')
\`\`\`

### Commits

\`\`\`json
$(cat "$HARVEST_DIR/commits.json" 2>/dev/null | head -100 || echo '{"commits":[]}')
\`\`\`

---

## Next Steps

To complete this dossier, run the LLM analyzers:

1. **Diff Scout:** Generate review map and hotspots
2. **Design Auditor:** Assess maintainability surface
3. **Verification Auditor:** Assess correctness surface
4. **Docs Auditor:** Assess reproducibility surface
5. **Chronologist:** Generate convergence narrative
6. **Decision Extractor:** Estimate DevLT

See [\`docs/forensics/prompts/README.md\`](prompts/README.md) for analyzer invocation.

---

*This dossier was auto-generated. Quality deltas and budget estimates require LLM analysis.*
EOF

echo "" >&2
echo "=== Dossier generated ===" >&2
echo "Output: $DOSSIER_FILE" >&2
echo "" >&2
echo "To complete analysis, run LLM analyzers on the harvest data." >&2
echo "See: docs/forensics/prompts/README.md" >&2

# Return the path
echo "$DOSSIER_FILE"
