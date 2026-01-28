#!/usr/bin/env bash
# Build timing receipt script for Perl LSP microcrate modularization
# Usage:
#   ./scripts/build-timing-receipt.sh [--clean] [--incremental] [--tests] [--output FILE] [--baseline]
#
# Options:
#   --clean         Run clean build (cargo clean first)
#   --incremental   Run incremental build (touch a file in target crate)
#   --tests         Run test build
#   --output FILE   Output file (default: artifacts/build-timing-receipt.json)
#   --baseline      Save as baseline (artifacts/build-timing-baseline.json)

set -euo pipefail

# Set locale to C for consistent number formatting (prevent comma separators)
export LC_ALL=C

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACTS_DIR="${PROJECT_ROOT}/artifacts"
mkdir -p "${ARTIFACTS_DIR}"

# Default values
RUN_CLEAN=false
RUN_INCREMENTAL=false
RUN_TESTS=false
OUTPUT_FILE="${ARTIFACTS_DIR}/build-timing-receipt.json"
IS_BASELINE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --clean)
      RUN_CLEAN=true
      shift
      ;;
    --incremental)
      RUN_INCREMENTAL=true
      shift
      ;;
    --tests)
      RUN_TESTS=true
      shift
      ;;
    --output)
      OUTPUT_FILE="$2"
      shift 2
      ;;
    --baseline)
      IS_BASELINE=true
      OUTPUT_FILE="${ARTIFACTS_DIR}/build-timing-baseline.json"
      shift
      ;;
    --help)
      echo "Usage: $0 [--clean] [--incremental] [--tests] [--output FILE] [--baseline]"
      echo ""
      echo "Options:"
      echo "  --clean         Run clean build (cargo clean first)"
      echo "  --incremental   Run incremental build (touch a file in target crate)"
      echo "  --tests         Run test build"
      echo "  --output FILE   Output file (default: artifacts/build-timing-receipt.json)"
      echo "  --baseline      Save as baseline (artifacts/build-timing-baseline.json)"
      echo ""
      echo "Examples:"
      echo "  $0 --clean --incremental --tests --baseline"
      echo "  $0 --clean --output my-timing.json"
      echo "  $0 --incremental"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Use --help for usage information" >&2
      exit 1
      ;;
  esac
done

# If no specific options provided, run all measurements
if [[ "$RUN_CLEAN" == false && "$RUN_INCREMENTAL" == false && "$RUN_TESTS" == false ]]; then
  RUN_CLEAN=true
  RUN_INCREMENTAL=true
  RUN_TESTS=true
fi

cd "${PROJECT_ROOT}"

# Gather system information
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
TOOLCHAIN=$(rustc --version 2>/dev/null || echo "unknown")
CPU_CORES=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "unknown")
MEMORY_GB=$(free -g 2>/dev/null | awk '/^Mem:/{print $2}' || sysctl -n hw.memsize 2>/dev/null | awk '{print int($1/1024/1024/1024)}' || echo "unknown")
OS=$(uname -s -r 2>/dev/null || echo "unknown")

# Start building JSON output
JSON_OUTPUT="{"
JSON_OUTPUT+='"timestamp":"'"${TIMESTAMP}"'",'
JSON_OUTPUT+='"toolchain":"'"${TOOLCHAIN}"'",'
JSON_OUTPUT+='"system":{'
JSON_OUTPUT+='"cpu_cores":'"${CPU_CORES}"','
JSON_OUTPUT+='"memory_gb":'"${MEMORY_GB}"','
JSON_OUTPUT+='"os":"'"${OS}"'"'
JSON_OUTPUT+='},'
JSON_OUTPUT+='"measurements":{'

# Temporary file for measurements
MEASUREMENTS_FILE=$(mktemp)

# Function to run a command and measure its time
measure_time() {
  local name="$1"
  local command="$2"
  local pre_command="${3:-}"

  echo "=== Measuring: ${name} ==="
  echo "Command: ${command}"

  # Run pre-command if provided
  if [[ -n "${pre_command}" ]]; then
    echo "Pre-command: ${pre_command}"
    eval "${pre_command}" > /dev/null 2>&1 || true
  fi

  # Measure time using bash's built-in time
  local start_time end_time elapsed
  start_time=$(date +%s.%N)
  eval "${command}" > /dev/null 2>&1 || true
  end_time=$(date +%s.%N)
  elapsed=$(echo "${end_time} - ${start_time}" | bc)

  echo "Duration: ${elapsed}s"
  echo ""

  # Escape the command for JSON
  local escaped_command
  escaped_command=$(echo "${command}" | sed 's/"/\\"/g')

  # Write measurement to temp file as single line
  echo '"'"${name}"'":{"duration_seconds":'"${elapsed}"',"command":"'"${escaped_command}"'"}' >> "${MEASUREMENTS_FILE}"
}

# Run clean build measurement
if [[ "$RUN_CLEAN" == true ]]; then
  measure_time "clean_build_workspace" "cargo build --workspace" "cargo clean"
fi

# Run incremental build measurement
if [[ "$RUN_INCREMENTAL" == true ]]; then
  # First ensure we have a clean build to measure from
  cargo build --workspace > /dev/null 2>&1 || true

  # Touch a file in perl-lsp-providers to trigger incremental rebuild
  if [[ -d "${PROJECT_ROOT}/crates/perl-lsp-providers" ]]; then
    touch "${PROJECT_ROOT}/crates/perl-lsp-providers/src/lib.rs" 2>/dev/null || true
    measure_time "incremental_build_providers" "cargo build -p perl-lsp-providers"
  else
    # If perl-lsp-providers doesn't exist yet, measure incremental on perl-parser
    touch "${PROJECT_ROOT}/crates/perl-parser/src/lib.rs" 2>/dev/null || true
    measure_time "incremental_build_parser" "cargo build -p perl-parser"
  fi
fi

# Run test build measurement
if [[ "$RUN_TESTS" == true ]]; then
  measure_time "test_build_workspace" "cargo test --workspace --lib"
fi

# Read measurements from temp file and build final JSON
MEASUREMENTS_JSON=$(cat "${MEASUREMENTS_FILE}" | paste -sd ',')

# Clean up temp file
rm -f "${MEASUREMENTS_FILE}"

# Build final JSON
FINAL_JSON="{"
FINAL_JSON+='"timestamp":"'"${TIMESTAMP}"'",'
FINAL_JSON+='"toolchain":"'"${TOOLCHAIN}"'",'
FINAL_JSON+='"system":{'
FINAL_JSON+='"cpu_cores":'"${CPU_CORES}"','
FINAL_JSON+='"memory_gb":'"${MEMORY_GB}"','
FINAL_JSON+='"os":"'"${OS}"'"'
FINAL_JSON+='},'
FINAL_JSON+='"measurements":{'"${MEASUREMENTS_JSON}"'}'
FINAL_JSON+='}'

# Write output to file
echo "${FINAL_JSON}" | jq '.' > "${OUTPUT_FILE}"

echo "=== Build Timing Receipt Generated ==="
echo "Output: ${OUTPUT_FILE}"
echo ""
cat "${OUTPUT_FILE}"

if [[ "$IS_BASELINE" == true ]]; then
  echo ""
  echo "Baseline saved to: ${OUTPUT_FILE}"
  echo "Use this baseline to compare against future measurements:"
  echo "  ./scripts/compare-build-timing.sh ${OUTPUT_FILE} <new-measurement.json>"
fi
