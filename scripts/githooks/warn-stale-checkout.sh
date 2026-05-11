#!/usr/bin/env bash
# warn-stale-checkout.sh — opt-in git hook for stale checkout warnings.
#
# Calls `cargo xtask freshness-check --mode warn` and parses the JSON receipt.
# When `safe_for_code_state_claims == false`, prints a one-line warning to
# stderr with the `behind_by` count and the remediation command.
#
# Always exits 0 — this is a warning hook, not a blocking hook.
#
# Installation (opt-in):
#   bash scripts/install-githooks.sh
#
# Or link manually:
#   ln -sf ../../scripts/githooks/warn-stale-checkout.sh .git/hooks/post-checkout
#
# Reference: docs/devex/freshness-check.md

set -euo pipefail

# Locate cargo — fall back gracefully if not available.
if ! command -v cargo &>/dev/null; then
    exit 0
fi

# Run the freshness check in warn mode, capturing JSON output.
RECEIPT="$(cargo xtask freshness-check --mode warn --no-fetch 2>/dev/null)" || {
    # If xtask itself fails (not built yet, etc.), silently skip.
    exit 0
}

# Parse the receipt. Prefer jq; fall back to python; fall back to grep.
if command -v jq &>/dev/null; then
    SAFE="$(printf '%s' "$RECEIPT" | jq -r '.safe_for_code_state_claims')"
    BEHIND="$(printf '%s' "$RECEIPT" | jq -r '.behind_by')"
    BASE_REF="$(printf '%s' "$RECEIPT" | jq -r '.base_ref')"
elif command -v python3 &>/dev/null; then
    SAFE="$(printf '%s' "$RECEIPT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(str(d['safe_for_code_state_claims']).lower())")"
    BEHIND="$(printf '%s' "$RECEIPT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['behind_by'])")"
    BASE_REF="$(printf '%s' "$RECEIPT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['base_ref'])")"
elif command -v python &>/dev/null; then
    SAFE="$(printf '%s' "$RECEIPT" | python -c "import sys,json; d=json.load(sys.stdin); print(str(d['safe_for_code_state_claims']).lower())")"
    BEHIND="$(printf '%s' "$RECEIPT" | python -c "import sys,json; d=json.load(sys.stdin); print(d['behind_by'])")"
    BASE_REF="$(printf '%s' "$RECEIPT" | python -c "import sys,json; d=json.load(sys.stdin); print(d['base_ref'])")"
else
    # Cannot parse JSON — skip the warning silently.
    exit 0
fi

if [ "$SAFE" = "false" ]; then
    echo "⚠  stale checkout: HEAD is ${BEHIND} commit(s) behind ${BASE_REF}" >&2
    echo "   run: git pull --rebase" >&2
fi

exit 0
