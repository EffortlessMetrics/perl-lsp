#!/usr/bin/env bash
# CI Cost Monitor for perl-lsp GitHub Actions
#
# Queries GitHub Actions API to calculate estimated CI costs based on workflow runs.
# Uses Linux runner pricing: $0.008/minute
#
# Usage:
#   ./scripts/ci-cost-monitor.sh               # Show cost breakdown (last 30 days)
#   ./scripts/ci-cost-monitor.sh --days 7      # Show last 7 days
#   ./scripts/ci-cost-monitor.sh --json        # Output as JSON
#   ./scripts/ci-cost-monitor.sh --help        # Show help message
#
# Requirements:
#   - gh (GitHub CLI) authenticated
#
# Cost Reference:
#   - GitHub Actions Linux runners: $0.008/minute
#   - Target budget: $720/year savings from Issue #211
#   - Monthly budget target: $60/month ($720/year)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
DAYS=30
OUTPUT_JSON=false
COST_PER_MINUTE=0.008  # GitHub Actions Linux runner pricing
ANNUAL_BUDGET_TARGET=720  # From Issue #211
MONTHLY_BUDGET_TARGET=60  # $720/year ÷ 12

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --days)
            DAYS="$2"
            shift 2
            ;;
        --json)
            OUTPUT_JSON=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --days N     Number of days to analyze (default: 30)"
            echo "  --json       Output as JSON"
            echo "  --help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Last 30 days"
            echo "  $0 --days 7           # Last 7 days"
            echo "  $0 --json             # JSON output"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# Check for gh CLI
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: gh CLI not found${NC}"
    echo "Install from: https://cli.github.com/"
    exit 1
fi

# Check gh authentication
if ! gh auth status &> /dev/null; then
    echo -e "${RED}Error: gh CLI not authenticated${NC}"
    echo "Run: gh auth login"
    exit 1
fi

# Get repository info
REPO_OWNER=$(gh repo view --json owner -q .owner.login 2>/dev/null || echo "")
REPO_NAME=$(gh repo view --json name -q .name 2>/dev/null || echo "")

if [[ -z "$REPO_OWNER" || -z "$REPO_NAME" ]]; then
    echo -e "${RED}Error: Could not determine repository owner/name${NC}"
    echo "Make sure you're in a git repository with a GitHub remote"
    exit 1
fi

# Calculate date range
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    START_DATE=$(date -v-${DAYS}d -u +%Y-%m-%dT%H:%M:%SZ)
else
    # Linux
    START_DATE=$(date -u -d "$DAYS days ago" +%Y-%m-%dT%H:%M:%SZ)
fi

# Query workflow runs
if ! $OUTPUT_JSON; then
    echo -e "${BLUE}Fetching workflow runs for ${BOLD}${REPO_OWNER}/${REPO_NAME}${NC}"
    echo -e "${BLUE}Period: Last $DAYS days (since $START_DATE)${NC}"
    echo ""
fi

# Fetch all workflow runs in the date range
WORKFLOW_RUNS=$(gh api "repos/${REPO_OWNER}/${REPO_NAME}/actions/runs" \
    --paginate \
    -X GET \
    -F created=">${START_DATE}" \
    -F per_page=100 \
    --jq '.workflow_runs[] | select(.conclusion != null) | {
        id: .id,
        name: .name,
        status: .status,
        conclusion: .conclusion,
        created_at: .created_at,
        updated_at: .updated_at,
        run_started_at: .run_started_at
    }' 2>/dev/null || echo "[]")

if [[ -z "$WORKFLOW_RUNS" || "$WORKFLOW_RUNS" == "[]" ]]; then
    if ! $OUTPUT_JSON; then
        echo -e "${YELLOW}No workflow runs found in the last $DAYS days${NC}"
    else
        echo '{"error": "No workflow runs found", "period_days": '$DAYS'}'
    fi
    exit 0
fi

# Process workflow runs and calculate costs
declare -A workflow_minutes
declare -A workflow_runs_count
declare -A workflow_cost
total_minutes=0
total_runs=0
successful_runs=0
failed_runs=0

while IFS= read -r run; do
    if [[ -z "$run" ]]; then
        continue
    fi

    workflow_name=$(echo "$run" | jq -r '.name')
    conclusion=$(echo "$run" | jq -r '.conclusion')
    run_started_at=$(echo "$run" | jq -r '.run_started_at // .created_at')
    updated_at=$(echo "$run" | jq -r '.updated_at')

    # Calculate duration in minutes
    if [[ "$run_started_at" != "null" && "$updated_at" != "null" ]]; then
        start_epoch=$(date -d "$run_started_at" +%s 2>/dev/null || date -j -f "%Y-%m-%dT%H:%M:%SZ" "$run_started_at" +%s 2>/dev/null || echo 0)
        end_epoch=$(date -d "$updated_at" +%s 2>/dev/null || date -j -f "%Y-%m-%dT%H:%M:%SZ" "$updated_at" +%s 2>/dev/null || echo 0)

        if [[ $start_epoch -gt 0 && $end_epoch -gt 0 ]]; then
            duration_seconds=$((end_epoch - start_epoch))
            duration_minutes=$((duration_seconds / 60))

            # Add to totals
            workflow_minutes["$workflow_name"]=$((${workflow_minutes["$workflow_name"]:-0} + duration_minutes))
            workflow_runs_count["$workflow_name"]=$((${workflow_runs_count["$workflow_name"]:-0} + 1))
            total_minutes=$((total_minutes + duration_minutes))
            total_runs=$((total_runs + 1))

            # Track success/failure
            if [[ "$conclusion" == "success" ]]; then
                successful_runs=$((successful_runs + 1))
            else
                failed_runs=$((failed_runs + 1))
            fi
        fi
    fi
done <<< "$WORKFLOW_RUNS"

# Calculate costs per workflow
for workflow in "${!workflow_minutes[@]}"; do
    minutes=${workflow_minutes["$workflow"]}
    cost=$(awk "BEGIN {printf \"%.2f\", $minutes * $COST_PER_MINUTE}")
    workflow_cost["$workflow"]=$cost
done

# Calculate total cost
total_cost=$(awk "BEGIN {printf \"%.2f\", $total_minutes * $COST_PER_MINUTE}")

# Calculate projections
monthly_minutes=$(awk "BEGIN {printf \"%.0f\", $total_minutes * 30 / $DAYS}")
monthly_cost=$(awk "BEGIN {printf \"%.2f\", $monthly_minutes * $COST_PER_MINUTE}")
annual_cost=$(awk "BEGIN {printf \"%.2f\", $monthly_cost * 12}")

# Calculate budget status
budget_percentage=$(awk "BEGIN {printf \"%.1f\", ($monthly_cost / $MONTHLY_BUDGET_TARGET) * 100}")

# Output results
if $OUTPUT_JSON; then
    # JSON output
    echo "{"
    echo "  \"period_days\": $DAYS,"
    echo "  \"start_date\": \"$START_DATE\","
    echo "  \"repository\": \"${REPO_OWNER}/${REPO_NAME}\","
    echo "  \"total_runs\": $total_runs,"
    echo "  \"successful_runs\": $successful_runs,"
    echo "  \"failed_runs\": $failed_runs,"
    echo "  \"total_minutes\": $total_minutes,"
    echo "  \"total_cost\": $total_cost,"
    echo "  \"monthly_projection\": {"
    echo "    \"minutes\": $monthly_minutes,"
    echo "    \"cost\": $monthly_cost,"
    echo "    \"budget_target\": $MONTHLY_BUDGET_TARGET,"
    echo "    \"budget_percentage\": $budget_percentage"
    echo "  },"
    echo "  \"annual_projection\": {"
    echo "    \"cost\": $annual_cost,"
    echo "    \"budget_target\": $ANNUAL_BUDGET_TARGET"
    echo "  },"
    echo "  \"workflows\": ["

    first=true
    if [[ ${#workflow_minutes[@]} -gt 0 ]]; then
        for workflow in $(printf '%s\n' "${!workflow_minutes[@]}" | sort); do
            minutes=${workflow_minutes["$workflow"]:-0}
            runs=${workflow_runs_count["$workflow"]:-0}
            cost=${workflow_cost["$workflow"]:-0}

            # Skip workflows with 0 runs or 0 minutes
            if [[ $runs -eq 0 || $minutes -eq 0 ]]; then
                continue
            fi

            if ! $first; then
                echo ","
            fi
            first=false

            avg_minutes=$(awk "BEGIN {printf \"%.1f\", $minutes / ($runs > 0 ? $runs : 1)}")

            echo "    {"
            echo "      \"name\": \"$workflow\","
            echo "      \"runs\": $runs,"
            echo "      \"total_minutes\": $minutes,"
            echo "      \"average_minutes\": $avg_minutes,"
            echo "      \"cost\": $cost"
            echo -n "    }"
        done
    fi
    echo ""
    echo "  ]"
    echo "}"
else
    # Human-readable output
    echo "==============================================================================="
    echo "                     CI Cost Analysis Report"
    echo "==============================================================================="
    echo ""
    echo -e "${BOLD}Period:${NC} Last $DAYS days (since $START_DATE)"
    echo -e "${BOLD}Repository:${NC} ${REPO_OWNER}/${REPO_NAME}"
    echo ""
    echo "==============================================================================="
    echo "                          Summary"
    echo "==============================================================================="
    printf "%-30s %10d\n" "Total workflow runs:" "$total_runs"
    printf "%-30s %10d (%.1f%%)\n" "  Successful:" "$successful_runs" "$(awk "BEGIN {if ($total_runs > 0) printf \"%.1f\", ($successful_runs / $total_runs) * 100; else print \"0.0\"}")"
    printf "%-30s %10d (%.1f%%)\n" "  Failed:" "$failed_runs" "$(awk "BEGIN {if ($total_runs > 0) printf \"%.1f\", ($failed_runs / $total_runs) * 100; else print \"0.0\"}")"
    printf "%-30s %10d minutes\n" "Total CI time:" "$total_minutes"
    printf "%-30s %10s\n" "Total cost:" "\$${total_cost}"
    echo ""

    echo "==============================================================================="
    echo "                       Monthly Projection"
    echo "==============================================================================="
    printf "%-30s %10d minutes\n" "Estimated monthly usage:" "$monthly_minutes"
    printf "%-30s %10s\n" "Estimated monthly cost:" "\$${monthly_cost}"
    printf "%-30s %10s\n" "Monthly budget target:" "\$${MONTHLY_BUDGET_TARGET}"

    # Budget status with color
    if (( $(awk "BEGIN {print ($monthly_cost <= $MONTHLY_BUDGET_TARGET)}") )); then
        printf "%-30s ${GREEN}%9.1f%%${NC} " "Budget utilization:" "$budget_percentage"
        echo -e "${GREEN}✓ Within budget${NC}"
    else
        printf "%-30s ${RED}%9.1f%%${NC} " "Budget utilization:" "$budget_percentage"
        echo -e "${RED}⚠ Over budget${NC}"
    fi
    echo ""

    echo "==============================================================================="
    echo "                       Annual Projection"
    echo "==============================================================================="
    printf "%-30s %10s\n" "Estimated annual cost:" "\$${annual_cost}"
    printf "%-30s %10s\n" "Annual budget target:" "\$${ANNUAL_BUDGET_TARGET}"

    if (( $(awk "BEGIN {print ($annual_cost <= $ANNUAL_BUDGET_TARGET)}") )); then
        echo -e "${GREEN}✓ Annual projection within budget${NC}"
    else
        savings_needed=$(awk "BEGIN {printf \"%.2f\", $annual_cost - $ANNUAL_BUDGET_TARGET}")
        echo -e "${RED}⚠ Annual projection exceeds budget by \$${savings_needed}${NC}"
    fi
    echo ""

    echo "==============================================================================="
    echo "                    Per-Workflow Breakdown"
    echo "==============================================================================="
    printf "%-35s %8s %12s %12s %10s\n" "Workflow" "Runs" "Total Min" "Avg Min" "Cost"
    echo "-------------------------------------------------------------------------------"

    # Sort workflows by cost (descending)
    if [[ ${#workflow_cost[@]} -gt 0 ]]; then
        for workflow in $(for w in "${!workflow_cost[@]}"; do echo "$w ${workflow_cost[$w]}"; done | sort -k2 -rn | cut -d' ' -f1); do
            minutes=${workflow_minutes["$workflow"]:-0}
            runs=${workflow_runs_count["$workflow"]:-0}
            cost=${workflow_cost["$workflow"]:-0.00}

            # Skip workflows with 0 runs or 0 minutes
            if [[ $runs -eq 0 || $minutes -eq 0 ]]; then
                continue
            fi

            avg_minutes=$(awk "BEGIN {printf \"%.1f\", $minutes / ($runs > 0 ? $runs : 1)}")

            # Truncate workflow name if too long
            if [[ ${#workflow} -gt 35 ]]; then
                display_name="${workflow:0:32}..."
            else
                display_name="$workflow"
            fi

            printf "%-35s %8d %12d %12s %10s\n" "$display_name" "$runs" "$minutes" "$avg_minutes" "\$${cost}"
        done
    fi
    echo "==============================================================================="
    echo ""

    # Recommendations
    echo "==============================================================================="
    echo "                        Recommendations"
    echo "==============================================================================="
    echo ""

    # Find most expensive workflow
    most_expensive_workflow=""
    most_expensive_cost=0
    if [[ ${#workflow_cost[@]} -gt 0 ]]; then
        for workflow in "${!workflow_cost[@]}"; do
            cost=${workflow_cost["$workflow"]:-0}
            if (( $(awk "BEGIN {print ($cost > $most_expensive_cost)}") )); then
                most_expensive_cost=$cost
                most_expensive_workflow="$workflow"
            fi
        done
    fi

    if [[ -n "$most_expensive_workflow" ]]; then
        echo -e "${YELLOW}1. Most Expensive Workflow:${NC}"
        echo "   '$most_expensive_workflow' costs \$${most_expensive_cost} ($(awk "BEGIN {printf \"%.1f\", ($most_expensive_cost / $total_cost) * 100}")% of total)"
        echo "   Consider optimizing or gating behind labels"
        echo ""
    fi

    # Check for failed runs
    if [[ $failed_runs -gt 0 ]]; then
        failure_rate=$(awk "BEGIN {printf \"%.1f\", ($failed_runs / $total_runs) * 100}")
        echo -e "${YELLOW}2. Failed Runs:${NC}"
        echo "   $failed_runs failed runs (${failure_rate}% failure rate)"
        echo "   Failed runs waste CI budget - investigate and fix flaky tests"
        echo ""
    fi

    # Concurrency cancellation savings
    echo -e "${GREEN}3. Concurrency Cancellation:${NC}"
    echo "   Ensure all workflows use concurrency.cancel-in-progress: true"
    echo "   This prevents wasted minutes on superseded runs"
    echo ""

    # Caching recommendations
    echo -e "${GREEN}4. Caching Strategy:${NC}"
    echo "   Use Swatinem/rust-cache@v2 to reduce build times by 50-70%"
    echo "   Ensure cache-on-failure: true to maximize cache hits"
    echo ""

    # Conditional execution
    echo -e "${GREEN}5. Conditional Execution:${NC}"
    echo "   Gate expensive workflows behind labels or workflow_dispatch"
    echo "   Run benchmarks/mutation tests only when needed"
    echo ""

    echo "==============================================================================="
    echo ""

    # Budget summary
    if (( $(awk "BEGIN {print ($monthly_cost <= $MONTHLY_BUDGET_TARGET)}") )); then
        echo -e "${GREEN}${BOLD}✓ CI costs are within budget${NC}"
    else
        echo -e "${RED}${BOLD}⚠ Action required: CI costs exceed budget${NC}"
        echo ""
        echo "Next steps:"
        echo "  1. Review expensive workflows above"
        echo "  2. Implement caching for all workflows"
        echo "  3. Gate expensive tests behind labels"
        echo "  4. Fix flaky tests to reduce failures"
    fi
    echo ""
fi
