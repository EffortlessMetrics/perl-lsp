#!/usr/bin/env bash
# Compatibility shim for the Rust gate runner.
#
# Usage:
#   ./scripts/execute-gate.sh <gate_name> [--receipt-dir <dir>] [cargo xtask gates args...]
#
# Gate definitions live in .ci/gate-policy.yaml and are executed by
# `cargo xtask gates`, not by this legacy shell script.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE_NAME="${1:?Gate name required}"
RECEIPT_DIR="${RECEIPT_DIR:-$ROOT/target/receipts}"

shift
extra_args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --receipt-dir)
            RECEIPT_DIR="${2:?--receipt-dir requires a directory}"
            shift 2
            ;;
        *)
            extra_args+=("$1")
            shift
            ;;
    esac
done

mkdir -p "$RECEIPT_DIR"
cd "$ROOT"
exec cargo xtask gates \
    --gate "$GATE_NAME" \
    --receipt \
    --receipt-path "$RECEIPT_DIR/gate-$GATE_NAME.json" \
    "${extra_args[@]}"
