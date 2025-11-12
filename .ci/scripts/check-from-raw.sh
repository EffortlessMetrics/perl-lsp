#!/usr/bin/env bash
# CI policy check: forbid direct ExitStatus::from_raw() except via helper
#
# Usage:
#   ./.ci/scripts/check-from-raw.sh
#
# Exit codes:
#   0 - All ExitStatus::from_raw() calls use helper
#   1 - Found direct from_raw() violations

set -Euo pipefail
shopt -s nullglob

viol="$(
  git grep -nE '\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(' -- \
    'crates/**/*.rs' 'xtask/**/*.rs' 'examples/**/*.rs' 'tests/**/*.rs' \
    ':!**/target/**' ':!**/generated/**' \
  | grep -Ev '::from_raw\([[:space:]]*raw[_ ]?exit[[:space:]]*\(' \
  || true
)"

if [[ -n "$viol" ]]; then
  echo "$viol" | sed 's/^/::error::Disallowed direct from_raw(): /' 1>&2
  exit 1
fi

echo "✅ ExitStatus policy check passed"
