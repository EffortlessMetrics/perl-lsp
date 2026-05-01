#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || true)"

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/scripts/install.sh" ]; then
    exec "$SCRIPT_DIR/scripts/install.sh" "$@"
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required to fetch the canonical installer" >&2
    exit 1
fi

curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/scripts/install.sh | bash -s -- "$@"
