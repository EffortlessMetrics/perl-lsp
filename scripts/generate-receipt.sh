#!/usr/bin/env bash
# Generate a formal gate execution receipt
#
# Usage:
#   ./scripts/generate-receipt.sh <gate_id> <exit_code> <duration_ms> [output_file]
#
# Environment variables:
#   GATE_COMMAND - Command that was executed
#   GATE_STDOUT - Path to stdout capture file
#   GATE_STDERR - Path to stderr capture file
#   PR_NUMBER - GitHub PR number (optional)
#   COMMIT_SHA - Git commit SHA (optional)
#   EXECUTOR - Where gate ran (local|github-actions|manual)

set -euo pipefail

# Arguments
GATE_ID="${1:?Gate ID required}"
EXIT_CODE="${2:?Exit code required}"
DURATION_MS="${3:?Duration in milliseconds required}"
OUTPUT_FILE="${4:-receipts/gate-${GATE_ID}-$(date +%s).yaml}"

# Environment defaults
GATE_COMMAND="${GATE_COMMAND:-unknown}"
GATE_STDOUT="${GATE_STDOUT:-/dev/null}"
GATE_STDERR="${GATE_STDERR:-/dev/null}"
PR_NUMBER="${PR_NUMBER:-}"
COMMIT_SHA="${COMMIT_SHA:-$(git rev-parse HEAD 2>/dev/null || echo 'unknown')}"
BRANCH="${BRANCH:-$(git branch --show-current 2>/dev/null || echo 'unknown')}"
EXECUTOR="${EXECUTOR:-local}"
AGENT="${AGENT:-human}"

# Read gate definition from registry
GATE_NAME=$(python3 -c "
import tomllib
with open('.ci/GATE_REGISTRY.toml', 'rb') as f:
    data = tomllib.load(f)
for gate in data.get('gate', []):
    if gate['id'] == '$GATE_ID':
        print(gate['name'])
        exit(0)
print('$GATE_ID')
")

# Determine status
if [ "$EXIT_CODE" -eq 0 ]; then
    STATUS="pass"
    CONCLUSION="Gate passed successfully"
else
    STATUS="fail"
    CONCLUSION="Gate failed with exit code $EXIT_CODE"
fi

# Truncate output files (max 50KB)
truncate_output() {
    local file="$1"
    local max_bytes=51200  # 50KB

    if [ -f "$file" ]; then
        local size=$(wc -c < "$file")
        if [ "$size" -gt "$max_bytes" ]; then
            echo "$(head -c $max_bytes "$file")"
            echo ""
            echo "[... output truncated at 50KB ...]"
        else
            cat "$file"
        fi
    else
        echo ""
    fi
}

STDOUT_CONTENT=$(truncate_output "$GATE_STDOUT")
STDERR_CONTENT=$(truncate_output "$GATE_STDERR")

# Create receipt directory
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Generate YAML receipt
cat > "$OUTPUT_FILE" <<EOF
---
receipt_version: "1.0"
gate_id: "$GATE_ID"
gate_name: "$GATE_NAME"

execution:
  timestamp: "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  duration_ms: $DURATION_MS
  exit_code: $EXIT_CODE
  command: |
    $(echo "$GATE_COMMAND" | sed 's/^/    /')
  environment:
    RUSTC_WRAPPER: "${RUSTC_WRAPPER:-}"
    RUST_TEST_THREADS: "${RUST_TEST_THREADS:-}"
    CARGO_BUILD_JOBS: "${CARGO_BUILD_JOBS:-}"

result:
  status: "$STATUS"
  conclusion: "$CONCLUSION"
  threshold_met: $([ "$EXIT_CODE" -eq 0 ] && echo "true" || echo "false")

evidence:
  stdout: |
$(echo "$STDOUT_CONTENT" | sed 's/^/    /')
  stderr: |
$(echo "$STDERR_CONTENT" | sed 's/^/    /')
  metrics: {}

metadata:
  commit_sha: "$COMMIT_SHA"
  branch: "$BRANCH"
  executor: "$EXECUTOR"
  agent: "$AGENT"
EOF

# Add PR number if provided
if [ -n "$PR_NUMBER" ]; then
    cat >> "$OUTPUT_FILE" <<EOF
  pr_number: $PR_NUMBER
EOF
fi

# Add routing decision
cat >> "$OUTPUT_FILE" <<EOF

routing:
  action: "$([ "$EXIT_CODE" -eq 0 ] && echo 'proceed' || echo 'block')"
  rationale: |
    Gate $GATE_ID $([ "$EXIT_CODE" -eq 0 ] && echo 'passed' || echo 'failed')
    $([ "$EXIT_CODE" -eq 0 ] && echo 'Proceeding to next gate' || echo 'Blocking merge until fixed')
EOF

echo "✅ Receipt generated: $OUTPUT_FILE"
