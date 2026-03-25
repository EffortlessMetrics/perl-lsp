#!/usr/bin/env bash
# Canonical implementation: cargo xtask ci-baseline.
#
# Supported flags are passed through to:
#   cargo xtask ci-baseline --branch BRANCH --days DAYS --limit LIMIT --output DIR

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_ROOT"

exec cargo xtask ci-baseline "$@"
