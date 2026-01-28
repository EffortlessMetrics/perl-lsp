#!/usr/bin/env bash
# Compare build timing receipts
# Usage:
#   ./scripts/compare-build-timing.sh BASELINE.json CURRENT.json

set -euo pipefail

# Set locale to C for consistent number formatting
export LC_ALL=C

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Check for required arguments
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 BASELINE.json CURRENT.json" >&2
  echo ""
  echo "Compare build timing receipts and generate a comparison report."
  echo ""
  echo "Arguments:"
  echo "  BASELINE.json  Path to the baseline build timing receipt"
  echo "  CURRENT.json   Path to the current build timing receipt"
  echo ""
  echo "Examples:"
  echo "  $0 artifacts/build-timing-baseline.json artifacts/build-timing-receipt.json"
  echo "  $0 baseline.json current.json > comparison.md"
  exit 1
fi

BASELINE_FILE="$1"
CURRENT_FILE="$2"

# Check if files exist
if [[ ! -f "${BASELINE_FILE}" ]]; then
  echo "Error: Baseline file not found: ${BASELINE_FILE}" >&2
  exit 1
fi

if [[ ! -f "${CURRENT_FILE}" ]]; then
  echo "Error: Current file not found: ${CURRENT_FILE}" >&2
  exit 1
fi

# Check if jq is available
if ! command -v jq &> /dev/null; then
  echo "Error: jq is required but not installed" >&2
  exit 1
fi

# Read JSON files
BASELINE_JSON=$(cat "${BASELINE_FILE}")
CURRENT_JSON=$(cat "${CURRENT_FILE}")

# Extract metadata
BASELINE_TIMESTAMP=$(echo "${BASELINE_JSON}" | jq -r '.timestamp')
CURRENT_TIMESTAMP=$(echo "${CURRENT_JSON}" | jq -r '.timestamp')
BASELINE_TOOLCHAIN=$(echo "${BASELINE_JSON}" | jq -r '.toolchain')
CURRENT_TOOLCHAIN=$(echo "${CURRENT_JSON}" | jq -r '.toolchain')

# Generate markdown report
cat <<EOF
# Build Timing Comparison

**Generated:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")

## Metadata

| Property | Baseline | Current |
|----------|----------|---------|
| Timestamp | ${BASELINE_TIMESTAMP} | ${CURRENT_TIMESTAMP} |
| Toolchain | ${BASELINE_TOOLCHAIN} | ${CURRENT_TOOLCHAIN} |

## Build Timing Results

EOF

# Get all measurement keys from both files
BASELINE_KEYS=$(echo "${BASELINE_JSON}" | jq -r '.measurements | keys[]')
CURRENT_KEYS=$(echo "${CURRENT_JSON}" | jq -r '.measurements | keys[]')

# Combine and deduplicate keys
ALL_KEYS=$(echo -e "${BASELINE_KEYS}\n${CURRENT_KEYS}" | sort -u)

# Print table header
echo "| Metric | Baseline | Current | Change | Improvement |"
echo "|--------|----------|---------|---------|-------------|"

# Track summary statistics
TOTAL_METRICS=0
IMPROVEMENTS=0
REGRESSIONS=0

# Process each measurement
for key in ${ALL_KEYS}; do
  TOTAL_METRICS=$((TOTAL_METRICS + 1))

  # Get baseline and current values
  BASELINE_VALUE=$(echo "${BASELINE_JSON}" | jq -r ".measurements.${key}.duration_seconds // null")
  CURRENT_VALUE=$(echo "${CURRENT_JSON}" | jq -r ".measurements.${key}.duration_seconds // null")

  # Calculate change and improvement
  if [[ "${BASELINE_VALUE}" != "null" && "${CURRENT_VALUE}" != "null" ]]; then
    CHANGE=$(echo "${CURRENT_VALUE} - ${BASELINE_VALUE}" | bc -l)
    ABS_CHANGE=$(echo "${CHANGE}" | tr -d '-')
    IMPROVEMENT=$(echo "(${BASELINE_VALUE} - ${CURRENT_VALUE}) / ${BASELINE_VALUE} * 100" | bc -l)

    # Format values
    BASELINE_FMT=$(printf "%.1fs" "${BASELINE_VALUE}")
    CURRENT_FMT=$(printf "%.1fs" "${CURRENT_VALUE}")
    CHANGE_FMT=$(printf "%+.1fs" "${CHANGE}")
    IMPROVEMENT_FMT=$(printf "%.1f%%" "${IMPROVEMENT}")

    # Determine if improvement or regression
    if (( $(echo "${IMPROVEMENT} > 0" | bc -l) )); then
      IMPROVEMENTS=$((IMPROVEMENTS + 1))
      IMPROVEMENT_FMT="🟢 ${IMPROVEMENT_FMT}"
    elif (( $(echo "${IMPROVEMENT} < 0" | bc -l) )); then
      REGRESSIONS=$((REGRESSIONS + 1))
      IMPROVEMENT_FMT="🔴 ${IMPROVEMENT_FMT}"
    fi

    echo "| ${key} | ${BASELINE_FMT} | ${CURRENT_FMT} | ${CHANGE_FMT} | ${IMPROVEMENT_FMT} |"
  elif [[ "${BASELINE_VALUE}" != "null" ]]; then
    # Only baseline has this metric
    BASELINE_FMT=$(printf "%.1fs" "${BASELINE_VALUE}")
    echo "| ${key} | ${BASELINE_FMT} | N/A | N/A | N/A |"
  elif [[ "${CURRENT_VALUE}" != "null" ]]; then
    # Only current has this metric
    CURRENT_FMT=$(printf "%.1fs" "${CURRENT_VALUE}")
    echo "| ${key} | N/A | ${CURRENT_FMT} | N/A | N/A |"
  fi
done

# Print summary
cat <<EOF

## Summary

- **Total metrics compared:** ${TOTAL_METRICS}
- **Improvements:** ${IMPROVEMENTS}
- **Regressions:** ${REGRESSIONS}

EOF

# Check for specific targets
CLEAN_BUILD_BASE=$(echo "${BASELINE_JSON}" | jq -r '.measurements.clean_build_workspace.duration_seconds // null')
CLEAN_BUILD_CUR=$(echo "${CURRENT_JSON}" | jq -r '.measurements.clean_build_workspace.duration_seconds // null')

INCREMENTAL_BUILD_BASE=$(echo "${BASELINE_JSON}" | jq -r '.measurements.incremental_build_providers.duration_seconds // .measurements.incremental_build_parser.duration_seconds // null')
INCREMENTAL_BUILD_CUR=$(echo "${CURRENT_JSON}" | jq -r '.measurements.incremental_build_providers.duration_seconds // .measurements.incremental_build_parser.duration_seconds // null')

TEST_BUILD_BASE=$(echo "${BASELINE_JSON}" | jq -r '.measurements.test_build_workspace.duration_seconds // null')
TEST_BUILD_CUR=$(echo "${CURRENT_JSON}" | jq -r '.measurements.test_build_workspace.duration_seconds // null')

cat <<EOF
## Target Validation

EOF

# Clean build target (40% faster)
if [[ "${CLEAN_BUILD_BASE}" != "null" && "${CLEAN_BUILD_CUR}" != "null" ]]; then
  CLEAN_IMPROVEMENT=$(echo "(${CLEAN_BUILD_BASE} - ${CLEAN_BUILD_CUR}) / ${CLEAN_BUILD_BASE} * 100" | bc -l)
  CLEAN_TARGET_MET=$(echo "${CLEAN_IMPROVEMENT} >= 40" | bc -l)

  echo "### Full Workspace Build (Target: 40% faster)"
  echo "- Baseline: $(printf "%.1fs" "${CLEAN_BUILD_BASE}")"
  echo "- Current: $(printf "%.1fs" "${CLEAN_BUILD_CUR}")"
  echo "- Improvement: $(printf "%.1f%%" "${CLEAN_IMPROVEMENT}")"
  if [[ "${CLEAN_TARGET_MET}" == "1" ]]; then
    echo "- Status: ✅ **Target Met**"
  else
    echo "- Status: ❌ **Target Not Met**"
  fi
  echo ""
else
  echo "### Full Workspace Build (Target: 40% faster)"
  echo "- Status: ⚠️ **No data available**"
  echo ""
fi

# Incremental build target (67% faster)
if [[ "${INCREMENTAL_BUILD_BASE}" != "null" && "${INCREMENTAL_BUILD_CUR}" != "null" ]]; then
  INCREMENTAL_IMPROVEMENT=$(echo "(${INCREMENTAL_BUILD_BASE} - ${INCREMENTAL_BUILD_CUR}) / ${INCREMENTAL_BUILD_BASE} * 100" | bc -l)
  INCREMENTAL_TARGET_MET=$(echo "${INCREMENTAL_IMPROVEMENT} >= 67" | bc -l)

  echo "### Incremental Build (Target: 67% faster)"
  echo "- Baseline: $(printf "%.1fs" "${INCREMENTAL_BUILD_BASE}")"
  echo "- Current: $(printf "%.1fs" "${INCREMENTAL_BUILD_CUR}")"
  echo "- Improvement: $(printf "%.1f%%" "${INCREMENTAL_IMPROVEMENT}")"
  if [[ "${INCREMENTAL_TARGET_MET}" == "1" ]]; then
    echo "- Status: ✅ **Target Met**"
  else
    echo "- Status: ❌ **Target Not Met**"
  fi
  echo ""
else
  echo "### Incremental Build (Target: 67% faster)"
  echo "- Status: ⚠️ **No data available**"
  echo ""
fi

# Test build
if [[ "${TEST_BUILD_BASE}" != "null" && "${TEST_BUILD_CUR}" != "null" ]]; then
  TEST_IMPROVEMENT=$(echo "(${TEST_BUILD_BASE} - ${TEST_BUILD_CUR}) / ${TEST_BUILD_BASE} * 100" | bc -l)

  echo "### Test Build"
  echo "- Baseline: $(printf "%.1fs" "${TEST_BUILD_BASE}")"
  echo "- Current: $(printf "%.1fs" "${TEST_BUILD_CUR}")"
  echo "- Improvement: $(printf "%.1f%%" "${TEST_IMPROVEMENT}")"
  echo ""
else
  echo "### Test Build"
  echo "- Status: ⚠️ **No data available**"
  echo ""
fi
