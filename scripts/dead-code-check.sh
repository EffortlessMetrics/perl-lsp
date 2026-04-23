#!/usr/bin/env bash
# Compatibility wrapper for dead code detection.
# Canonical implementation: cargo xtask dead-code.

set -euo pipefail

if [ "$#" -eq 0 ]; then
  set -- check
fi

if [ "${DEAD_CODE_STRICT:-false}" = "true" ] && [[ "$*" != *"--strict"* ]]; then
  set -- "$@" --strict
fi

exec cargo xtask dead-code "$@"
