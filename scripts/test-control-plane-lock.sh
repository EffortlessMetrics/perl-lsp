#!/usr/bin/env bash
# Test suite for control-plane-lock.sh
# Tests: acquire, release, status, stale-expiry, force-release, double-acquire, wrong-holder release

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCK_SCRIPT="$SCRIPT_DIR/control-plane-lock.sh"
# Use a temp lock file so tests don't interfere with any real lock
export CONTROL_PLANE_LOCK_FILE
CONTROL_PLANE_LOCK_FILE="$(mktemp)"
rm -f "$CONTROL_PLANE_LOCK_FILE"  # remove so lock starts absent

PASS=0
FAIL=0

pass() { echo "  PASS: $1"; ((PASS++)) || true; }
fail() { echo "  FAIL: $1"; ((FAIL++)) || true; }

cleanup() {
    rm -f "$CONTROL_PLANE_LOCK_FILE"
}
trap cleanup EXIT

echo "=== control-plane-lock.sh test suite ==="
echo ""

# ── Test 1: status when unlocked ──────────────────────────────────────────────
echo "1. status when unlocked"
output=$("$LOCK_SCRIPT" status 2>&1)
if echo "$output" | grep -qi "unlocked"; then
    pass "reports unlocked"
else
    fail "expected 'unlocked', got: $output"
fi

# ── Test 2: acquire succeeds ──────────────────────────────────────────────────
echo "2. acquire"
if "$LOCK_SCRIPT" acquire agent-001 2>&1; then
    pass "acquire succeeds for agent-001"
else
    fail "acquire should succeed when lock is free"
fi

# ── Test 3: status shows holder after acquire ─────────────────────────────────
echo "3. status shows holder"
output=$("$LOCK_SCRIPT" status 2>&1)
if echo "$output" | grep -q "agent-001"; then
    pass "status shows agent-001"
else
    fail "expected 'agent-001' in status, got: $output"
fi

# ── Test 4: double acquire fails ──────────────────────────────────────────────
echo "4. double acquire (contention)"
if "$LOCK_SCRIPT" acquire agent-002 2>&1; then
    fail "second acquire should fail"
else
    pass "second acquire correctly refused"
fi

# ── Test 5: release by wrong holder fails ─────────────────────────────────────
echo "5. release by wrong holder"
if "$LOCK_SCRIPT" release agent-002 2>&1; then
    fail "release by non-holder should fail"
else
    pass "release by non-holder correctly refused"
fi

# ── Test 6: release by correct holder succeeds ───────────────────────────────
echo "6. release by correct holder"
if "$LOCK_SCRIPT" release agent-001 2>&1; then
    pass "release by holder succeeds"
else
    fail "release by holder should succeed"
fi

# ── Test 7: status unlocked after release ─────────────────────────────────────
echo "7. status unlocked after release"
output=$("$LOCK_SCRIPT" status 2>&1)
if echo "$output" | grep -qi "unlocked"; then
    pass "reports unlocked after release"
else
    fail "expected 'unlocked' after release, got: $output"
fi

# ── Test 8: stale lock detection (expired timestamp) ─────────────────────────
echo "8. stale lock expiry"
# Write a lock file with a timestamp 31 minutes in the past
stale_ts=$(date -d "31 minutes ago" +%s 2>/dev/null || date -v-31M +%s 2>/dev/null || echo $(($(date +%s) - 1860)))
printf "stale-agent\n%s\n" "$stale_ts" > "$CONTROL_PLANE_LOCK_FILE"

output=$("$LOCK_SCRIPT" status 2>&1)
if echo "$output" | grep -qi "stale\|unlocked\|expired"; then
    pass "stale lock detected"
else
    fail "expected stale detection, got: $output"
fi

# Acquire should succeed over a stale lock
if "$LOCK_SCRIPT" acquire agent-003 2>&1; then
    pass "acquire succeeds over stale lock"
else
    fail "acquire should succeed over stale lock"
fi

# ── Test 9: force-release ─────────────────────────────────────────────────────
echo "9. force-release"
if "$LOCK_SCRIPT" force-release 2>&1; then
    pass "force-release succeeds"
else
    fail "force-release should always succeed"
fi

output=$("$LOCK_SCRIPT" status 2>&1)
if echo "$output" | grep -qi "unlocked"; then
    pass "unlocked after force-release"
else
    fail "expected unlocked after force-release, got: $output"
fi

# ── Test 10: re-acquire after force-release ───────────────────────────────────
echo "10. re-acquire after force-release"
if "$LOCK_SCRIPT" acquire agent-004 2>&1 && "$LOCK_SCRIPT" release agent-004 2>&1; then
    pass "acquire/release cycle works after force-release"
else
    fail "re-acquire/release cycle failed"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
