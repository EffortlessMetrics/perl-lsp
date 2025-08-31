#!/usr/bin/env bash
set -euo pipefail

# Run E2E tests with concurrency gating
source "$(dirname "$0")/preflight.sh"

echo "Running comprehensive E2E tests with concurrency caps..."
"$(dirname "$0")/e2e-gate.sh" "$@"