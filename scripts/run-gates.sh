#!/usr/bin/env bash
# Compatibility shim for the Rust gate runner.
#
# Usage:
#   RUN_FULL=1 ./scripts/run-gates.sh   # optional all-tier gate
#   ./scripts/run-gates.sh --format json

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TIER="merge-gate"
if [[ "${RUN_FULL:-}" == "1" ]]; then
    TIER="all"
fi

cd "$ROOT"
exec cargo xtask gates \
    --tier "$TIER" \
    --receipt \
    --receipt-path "$ROOT/target/receipts/receipt.json" \
    "$@"
