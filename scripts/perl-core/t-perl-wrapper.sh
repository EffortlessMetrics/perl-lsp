#!/usr/bin/env bash
set -euo pipefail

if [ -z "${PERL_LSP_CORE_TEST_RUNNER:-}" ]; then
  echo "PERL_LSP_CORE_TEST_RUNNER is not set" >&2
  exit 127
fi

exec "$PERL_LSP_CORE_TEST_RUNNER" "$@"
