#!/usr/bin/env bash
#
# measure-ci-baseline.sh - Measure CI baseline metrics from GitHub Actions
#
# This script collects workflow run data from GitHub Actions and calculates
# baseline metrics including median duration, P95 duration, success rate,
# and approximate billable minutes.
#
# Requirements:
#   - gh CLI installed and authenticated (gh auth login)
#   - jq for JSON processing
#   - bc for calculations (optional, falls back to awk)
#
# Usage:
#   ./measure-ci-baseline.sh [options]
#
# Options:
#   -b, --branch BRANCH    Branch to analyze (default: master)
#   -d, --days DAYS        Number of days to look back (default: 30)
#   -l, --limit LIMIT      Max runs to fetch (default: 200)
#   -o, --output DIR       Output directory (default: .ci)
#   -h, --help             Show this help message
#
# Output:
#   .ci/ci_baseline.json   Machine-readable metrics
#   .ci/ci_baseline.md     Human-readable report

set -euo pipefail

# Default configuration
BRANCH="master"
DAYS=30
LIMIT=200
OUTPUT_DIR=".ci"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output (disabled if not a terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

usage() {
    head -30 "$0" | grep -E "^#" | sed 's/^# //' | sed 's/^#//'
    exit 0
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -b|--branch) BRANCH="$2"; shift 2 ;;
        -d|--days) DAYS="$2"; shift 2 ;;
        -l|--limit) LIMIT="$2"; shift 2 ;;
        -o|--output) OUTPUT_DIR="$2"; shift 2 ;;
        -h|--help) usage ;;
        *) log_error "Unknown option: $1"; usage ;;
    esac
done

# Check prerequisites
check_prerequisites() {
    local missing=()
    
    if ! command -v gh &>/dev/null; then
        missing+=("gh (GitHub CLI)")
    fi
    
    if ! command -v jq &>/dev/null; then
        missing+=("jq")
    fi
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        echo ""
        echo "Installation instructions:"
        echo "  gh:  https://cli.github.com/manual/installation"
        echo "  jq:  https://stedolan.github.io/jq/download/"
        exit 1
    fi
    
    # Check gh authentication
    if ! gh auth status &>/dev/null; then
        log_error "GitHub CLI is not authenticated"
        echo ""
        echo "Please run: gh auth login"
        exit 1
    fi
    
    log_success "Prerequisites satisfied"
}

# Calculate percentile from sorted array (passed as newline-separated values)
calculate_percentile() {
    local percentile=$1
    local values
    mapfile -t values < <(sort -n)
    local count=${#values[@]}
    
    if [[ $count -eq 0 ]]; then
        echo "0"
        return
    fi
    
    local index
    index=$(awk "BEGIN {printf \"%d\", ($count - 1) * $percentile / 100}")
    echo "${values[$index]}"
}

# Calculate median
calculate_median() {
    calculate_percentile 50
}

# Calculate P95
calculate_p95() {
    calculate_percentile 95
}

# Convert ISO 8601 timestamp to epoch seconds
iso_to_epoch() {
    local ts="$1"
    # Handle both GNU and BSD date
    if date --version &>/dev/null 2>&1; then
        date -d "$ts" +%s 2>/dev/null || echo "0"
    else
        date -j -f "%Y-%m-%dT%H:%M:%SZ" "$ts" +%s 2>/dev/null || echo "0"
    fi
}

# Calculate duration between two ISO timestamps in seconds
calculate_duration() {
    local start="$1"
    local end="$2"
    local start_epoch end_epoch
    
    start_epoch=$(iso_to_epoch "$start")
    end_epoch=$(iso_to_epoch "$end")
    
    if [[ "$start_epoch" -eq 0 ]] || [[ "$end_epoch" -eq 0 ]]; then
        echo "0"
        return
    fi
    
    echo $((end_epoch - start_epoch))
}

# Format seconds as human-readable duration
format_duration() {
    local seconds=$1
    if [[ $seconds -lt 60 ]]; then
        echo "${seconds}s"
    elif [[ $seconds -lt 3600 ]]; then
        echo "$((seconds / 60))m $((seconds % 60))s"
    else
        echo "$((seconds / 3600))h $((seconds % 3600 / 60))m"
    fi
}

# Main measurement function
measure_ci_baseline() {
    local cutoff_date
    cutoff_date=$(date -d "-${DAYS} days" +%Y-%m-%d 2>/dev/null || date -v-${DAYS}d +%Y-%m-%d)
    
    log_info "Fetching workflow runs from branch '$BRANCH' (last $DAYS days, limit $LIMIT)"
    
    # Fetch workflow runs
    local runs_json
    runs_json=$(gh run list --limit "$LIMIT" --branch "$BRANCH" \
        --json name,conclusion,createdAt,updatedAt,databaseId,workflowName,status)
    
    if [[ -z "$runs_json" ]] || [[ "$runs_json" == "[]" ]]; then
        log_warn "No workflow runs found for branch '$BRANCH'"
        return 1
    fi
    
    local run_count
    run_count=$(echo "$runs_json" | jq 'length')
    log_info "Found $run_count workflow runs"
    
    # Get unique workflow names
    local workflows
    mapfile -t workflows < <(echo "$runs_json" | jq -r '.[].workflowName' | sort -u)
    
    log_info "Analyzing ${#workflows[@]} unique workflows..."
    
    # Initialize results
    local results='{"generated_at": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'", "branch": "'$BRANCH'", "days_analyzed": '$DAYS', "workflows": {}}'
    
    # Process each workflow
    for workflow in "${workflows[@]}"; do
        log_info "  Processing: $workflow"
        
        # Filter runs for this workflow
        local workflow_runs
        workflow_runs=$(echo "$runs_json" | jq --arg wf "$workflow" '[.[] | select(.workflowName == $wf)]')
        
        local total_runs completed_runs success_count skipped_count failure_count
        total_runs=$(echo "$workflow_runs" | jq 'length')
        
        # Count by conclusion
        success_count=$(echo "$workflow_runs" | jq '[.[] | select(.conclusion == "success")] | length')
        failure_count=$(echo "$workflow_runs" | jq '[.[] | select(.conclusion == "failure")] | length')
        skipped_count=$(echo "$workflow_runs" | jq '[.[] | select(.conclusion == "skipped")] | length')
        
        # Only count non-skipped runs for duration analysis
        completed_runs=$((success_count + failure_count))
        
        # Calculate success rate (excluding skipped)
        local success_rate="0"
        if [[ $completed_runs -gt 0 ]]; then
            success_rate=$(awk "BEGIN {printf \"%.1f\", $success_count * 100 / $completed_runs}")
        fi
        
        # Calculate durations for completed runs
        local durations=()
        while IFS= read -r run; do
            local created updated duration
            created=$(echo "$run" | jq -r '.createdAt')
            updated=$(echo "$run" | jq -r '.updatedAt')
            conclusion=$(echo "$run" | jq -r '.conclusion')
            
            # Skip skipped runs
            if [[ "$conclusion" == "skipped" ]]; then
                continue
            fi
            
            duration=$(calculate_duration "$created" "$updated")
            if [[ $duration -gt 0 ]]; then
                durations+=("$duration")
            fi
        done < <(echo "$workflow_runs" | jq -c '.[]')
        
        # Calculate statistics
        local median_duration=0 p95_duration=0 avg_duration=0
        local total_duration=0
        
        if [[ ${#durations[@]} -gt 0 ]]; then
            # Calculate median
            median_duration=$(printf '%s\n' "${durations[@]}" | calculate_median)
            
            # Calculate P95
            p95_duration=$(printf '%s\n' "${durations[@]}" | calculate_p95)
            
            # Calculate average
            for d in "${durations[@]}"; do
                total_duration=$((total_duration + d))
            done
            avg_duration=$((total_duration / ${#durations[@]}))
        fi
        
        # Estimate billable minutes (rounded up to nearest minute per run)
        local billable_minutes=0
        for d in "${durations[@]}"; do
            billable_minutes=$((billable_minutes + (d + 59) / 60))
        done
        
        # Add workflow results
        local workflow_key
        workflow_key=$(echo "$workflow" | tr ' ' '_' | tr -cd '[:alnum:]_-')
        
        results=$(echo "$results" | jq --arg key "$workflow_key" \
            --arg name "$workflow" \
            --argjson total "$total_runs" \
            --argjson completed "$completed_runs" \
            --argjson success "$success_count" \
            --argjson failure "$failure_count" \
            --argjson skipped "$skipped_count" \
            --arg rate "$success_rate" \
            --argjson median "$median_duration" \
            --argjson p95 "$p95_duration" \
            --argjson avg "$avg_duration" \
            --argjson billable "$billable_minutes" \
            '.workflows[$key] = {
                "name": $name,
                "total_runs": $total,
                "completed_runs": $completed,
                "success_count": $success,
                "failure_count": $failure,
                "skipped_count": $skipped,
                "success_rate_percent": ($rate | tonumber),
                "median_duration_seconds": $median,
                "p95_duration_seconds": $p95,
                "avg_duration_seconds": $avg,
                "billable_minutes": $billable
            }')
    done
    
    # Calculate totals
    local total_billable total_runs total_success
    total_billable=$(echo "$results" | jq '[.workflows[].billable_minutes] | add // 0')
    total_runs=$(echo "$results" | jq '[.workflows[].total_runs] | add // 0')
    total_success=$(echo "$results" | jq '[.workflows[].success_count] | add // 0')
    total_completed=$(echo "$results" | jq '[.workflows[].completed_runs] | add // 0')
    
    local overall_success_rate="0"
    if [[ $total_completed -gt 0 ]]; then
        overall_success_rate=$(awk "BEGIN {printf \"%.1f\", $total_success * 100 / $total_completed}")
    fi
    
    results=$(echo "$results" | jq \
        --argjson total_runs "$total_runs" \
        --argjson total_billable "$total_billable" \
        --arg overall_rate "$overall_success_rate" \
        '. + {
            "summary": {
                "total_runs": $total_runs,
                "total_billable_minutes": $total_billable,
                "overall_success_rate_percent": ($overall_rate | tonumber)
            }
        }')
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Write JSON output
    echo "$results" | jq '.' > "$OUTPUT_DIR/ci_baseline.json"
    log_success "Written: $OUTPUT_DIR/ci_baseline.json"
    
    # Generate markdown report
    generate_markdown_report "$results" "$OUTPUT_DIR/ci_baseline.md"
    log_success "Written: $OUTPUT_DIR/ci_baseline.md"
    
    # Print summary
    echo ""
    echo "======================================"
    echo "CI Baseline Summary"
    echo "======================================"
    echo "Branch:              $BRANCH"
    echo "Analysis period:     Last $DAYS days"
    echo "Total runs:          $total_runs"
    echo "Total billable:      ${total_billable}m"
    echo "Overall success:     ${overall_success_rate}%"
    echo "======================================"
}

generate_markdown_report() {
    local results="$1"
    local output_file="$2"
    
    cat > "$output_file" << HEADER
# CI Baseline Metrics Report

**Generated:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Branch:** $(echo "$results" | jq -r '.branch')
**Analysis Period:** Last $(echo "$results" | jq -r '.days_analyzed') days

## Summary

| Metric | Value |
|--------|-------|
| Total Runs | $(echo "$results" | jq -r '.summary.total_runs') |
| Overall Success Rate | $(echo "$results" | jq -r '.summary.overall_success_rate_percent')% |
| Total Billable Minutes | $(echo "$results" | jq -r '.summary.total_billable_minutes')m |

## Workflow Details

| Workflow | Runs | Success Rate | Median | P95 | Billable |
|----------|------|--------------|--------|-----|----------|
HEADER

    # Add each workflow
    echo "$results" | jq -r '.workflows | to_entries[] | 
        "| \(.value.name) | \(.value.completed_runs) | \(.value.success_rate_percent)% | \(.value.median_duration_seconds)s | \(.value.p95_duration_seconds)s | \(.value.billable_minutes)m |"' >> "$output_file"
    
    cat >> "$output_file" << 'FOOTER'

## Notes

- **Median Duration:** 50th percentile of run duration (in seconds)
- **P95 Duration:** 95th percentile of run duration (in seconds)
- **Billable Minutes:** Estimated billable time (each run rounded up to nearest minute)
- **Success Rate:** Calculated excluding skipped runs

## Recommendations

Based on this baseline:

1. **Monitor P95 durations** - Workflows with high P95 vs median ratio may have reliability issues
2. **Success rate targets** - Aim for >95% success rate on critical workflows
3. **Cost optimization** - Focus on high-billable workflows for optimization efforts

---
*Generated by measure-ci-baseline.sh*
FOOTER
}

# Entry point
main() {
    cd "$REPO_ROOT"
    check_prerequisites
    measure_ci_baseline
}

main "$@"
