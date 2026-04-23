#!/usr/bin/env bash
# Execute a single gate with receipt generation
#
# Usage:
#   ./scripts/execute-gate.sh <gate_id> [--receipt-dir <dir>]

set -euo pipefail

# Arguments
GATE_ID="${1:?Gate ID required}"
RECEIPT_DIR="${RECEIPT_DIR:-.receipts}"

# Parse optional arguments
shift
while [[ $# -gt 0 ]]; do
    case $1 in
        --receipt-dir)
            RECEIPT_DIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Load gate definition from registry
GATE_TOML=".ci/GATE_REGISTRY.toml"
if [ ! -f "$GATE_TOML" ]; then
    echo "❌ Gate registry not found: $GATE_TOML" >&2
    exit 1
fi

# Extract gate command using Python
GATE_COMMAND=$(python3 -c "
import sys
import tomllib

with open('$GATE_TOML', 'rb') as f:
    data = tomllib.load(f)

for gate in data.get('gate', []):
    if gate['id'] == '$GATE_ID':
        print(gate['command'])
        sys.exit(0)

print(f'Gate not found: $GATE_ID', file=sys.stderr)
sys.exit(1)
")

GATE_TIMEOUT=$(python3 -c "
import sys
import tomllib

with open('$GATE_TOML', 'rb') as f:
    data = tomllib.load(f)

for gate in data.get('gate', []):
    if gate['id'] == '$GATE_ID':
        print(gate.get('timeout_seconds', 600))
        sys.exit(0)
print('600')
")

# Create temporary files for output capture
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
trap "rm -f $STDOUT_FILE $STDERR_FILE" EXIT

# Execute gate with timeout and capture
echo "🚪 Executing gate: $GATE_ID"
echo "📝 Command: $GATE_COMMAND"

START_TIME=$(date +%s%3N)  # Milliseconds
set +e
timeout "$GATE_TIMEOUT" bash -c "$GATE_COMMAND" > "$STDOUT_FILE" 2> "$STDERR_FILE"
EXIT_CODE=$?
set -e
END_TIME=$(date +%s%3N)

DURATION_MS=$((END_TIME - START_TIME))

# Handle timeout specially
if [ "$EXIT_CODE" -eq 124 ]; then
    echo "⏱️  Gate timed out after ${GATE_TIMEOUT}s"
    EXIT_CODE=124
fi

# Generate receipt
export GATE_COMMAND
export GATE_STDOUT="$STDOUT_FILE"
export GATE_STDERR="$STDERR_FILE"
export EXECUTOR="${EXECUTOR:-local}"
export AGENT="${AGENT:-human}"

RECEIPT_FILE="${RECEIPT_DIR}/gate-${GATE_ID}-$(date +%Y%m%d-%H%M%S).yaml"
bash ./scripts/generate-receipt.sh "$GATE_ID" "$EXIT_CODE" "$DURATION_MS" "$RECEIPT_FILE"

# Display result
if [ "$EXIT_CODE" -eq 0 ]; then
    echo "✅ Gate $GATE_ID PASSED (${DURATION_MS}ms)"
else
    echo "❌ Gate $GATE_ID FAILED (exit code: $EXIT_CODE, ${DURATION_MS}ms)"
    echo "📄 Receipt: $RECEIPT_FILE"
    echo ""
    echo "=== STDERR ==="
    cat "$STDERR_FILE"
    echo ""
fi

exit "$EXIT_CODE"
