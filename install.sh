#!/usr/bin/env bash
# Compatibility wrapper for the canonical Linux/macOS installer.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
#   bash install.sh 0.13.1 "$HOME/.local/bin"

set -euo pipefail

# `${BASH_SOURCE[0]:-}` — not a bare `${BASH_SOURCE[0]}`. When this script is
# read from stdin (`curl -fsSL .../install.sh | bash`, the documented bootstrap)
# BASH_SOURCE is an empty array, and under `set -u` the unguarded expansion
# aborts with `BASH_SOURCE[0]: unbound variable` before anything is downloaded.
# A piped invocation has no script directory, so there is no sibling checkout to
# prefer: leave CANONICAL_INSTALLER empty and fetch the canonical installer.
SCRIPT_SOURCE="${BASH_SOURCE[0]:-}"
CANONICAL_INSTALLER=""
if [ -n "$SCRIPT_SOURCE" ]; then
    SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_SOURCE")" 2>/dev/null && pwd || pwd)"
    CANONICAL_INSTALLER="$SCRIPT_DIR/scripts/install.sh"
fi
CANONICAL_INSTALLER_URL="https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/scripts/install.sh"

# Every expansion of ARGS below uses `${ARGS[@]+"${ARGS[@]}"}`, never a bare
# `"${ARGS[@]}"`. Under `set -u`, expanding an empty array as `"${arr[@]}"` is
# an unbound-variable error on bash < 4.4, and macOS ships /bin/bash 3.2. The
# documented bootstrap (`curl -fsSL .../install.sh | bash`) is exactly the
# zero-argument path, so the unguarded form aborts a macOS user's very first
# command with `ARGS[@]: unbound variable`.
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

if [ -n "$CANONICAL_INSTALLER" ] && [ -f "$CANONICAL_INSTALLER" ]; then
    exec "$CANONICAL_INSTALLER" ${ARGS[@]+"${ARGS[@]}"}
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "Error: curl is required to fetch the canonical installer" >&2
    exit 1
fi

TMP_INSTALLER="$(mktemp)"
trap 'rm -f "$TMP_INSTALLER"' EXIT
curl -fsSL "$CANONICAL_INSTALLER_URL" -o "$TMP_INSTALLER"
exec bash "$TMP_INSTALLER" ${ARGS[@]+"${ARGS[@]}"}
