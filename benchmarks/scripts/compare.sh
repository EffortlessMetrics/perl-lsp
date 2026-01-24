#!/bin/bash
# Compare benchmark results against baseline
#
# This is a wrapper that calls the Python implementation for full functionality.
# See compare.py for the actual comparison logic.
#
# Usage:
#   ./compare.sh                              # Compare latest vs committed baseline
#   ./compare.sh baseline.json current.json   # Compare two specific files
#   ./compare.sh --fail-on-regression         # Exit non-zero if regression found

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if Python is available
if command -v python3 >/dev/null 2>&1; then
    exec python3 "$SCRIPT_DIR/compare.py" "$@"
else
    echo "Error: Python 3 is required but not found" >&2
    echo "Install Python 3 or use the compare.py script directly" >&2
    exit 1
fi
