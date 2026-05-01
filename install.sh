#!/usr/bin/env bash
# Compatibility wrapper for the canonical Linux/macOS installer.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
#   bash install.sh 0.13.1 "$HOME/.local/bin"

set -euo pipefail

SCRIPT_SOURCE="${BASH_SOURCE[0]}"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_SOURCE")" 2>/dev/null && pwd || pwd)"
CANONICAL_INSTALLER="$SCRIPT_DIR/scripts/install.sh"
CANONICAL_INSTALLER_URL="https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/scripts/install.sh"

ARGS=("$@")

if [ "${1:-}" != "" ] && [[ "${1:-}" != -* ]]; then
    if [ -z "${VERSION:-}" ]; then
        export VERSION="$1"
    fi
    shift

    if [ "${1:-}" != "" ] && [[ "${1:-}" != -* ]]; then
        if [ -z "${INSTALL_DIR:-}" ]; then
            export INSTALL_DIR="$1"
        fi
        shift
    fi

    ARGS=("$@")
fi

if [ -f "$CANONICAL_INSTALLER" ]; then
    exec "$CANONICAL_INSTALLER" "${ARGS[@]}"
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "Error: curl is required to fetch the canonical installer" >&2
    exit 1
fi

TMP_INSTALLER="$(mktemp)"
trap 'rm -f "$TMP_INSTALLER"' EXIT
curl -fsSL "$CANONICAL_INSTALLER_URL" -o "$TMP_INSTALLER"
exec bash "$TMP_INSTALLER" "${ARGS[@]}"
