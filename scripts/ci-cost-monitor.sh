#!/usr/bin/env bash
# Canonical implementation: cargo xtask ci-cost-monitor.
#
# Supported flags are:
#   --days N     Number of days to analyze
#   --json       JSON output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

exec cargo xtask ci-cost-monitor "$@"
