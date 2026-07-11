#!/usr/bin/env bash
# Test suite for scripts/reviews/lease (#3693 R1, FILE 5).
#
# This is the relocated home of fixture 11 (expired-lease → block +
# route-takeover). Lease state is NOT GitHub PR data and "route-takeover" is
# not a 0/1/2 convergence verdict, so it does NOT belong in the
# check-pr-review-convergence closeout — it lives here, keyed on branch, in
# .ops-perl-lsp/review-leases/<branch>.json. All cases run offline against a
# temp REVIEW_LEASES_DIR; no network, no real lease store touched.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LEASE="$SCRIPT_DIR/../reviews/lease"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

if [[ ! -f "$LEASE" ]]; then echo "ERROR: lease script not found at $LEASE"; exit 1; fi
if ! command -v jq >/dev/null 2>&1; then echo "ERROR: jq not found on PATH"; exit 1; fi

TMPDIR_LEASE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_LEASE"' EXIT
export REVIEW_LEASES_DIR="$TMPDIR_LEASE/review-leases"

run() { local e=0; RUN_OUT="$(REVIEW_LEASES_DIR="$REVIEW_LEASES_DIR" bash "$LEASE" "$@" 2>&1)" || e=$?; RUN_EXIT=$e; }

# ── acquire → verify passes ─────────────────────────────────────────────────
test_acquire_then_verify() {
    run acquire --branch feat/3693-x --owner alice --pr 42
    local a=$RUN_EXIT
    run verify --branch feat/3693-x
    if [[ "$a" -eq 0 && "$RUN_EXIT" -eq 0 ]]; then
        pass "acquire then verify: unexpired lease verifies (exit 0)"
    else
        fail "acquire/verify — acquire exit=$a verify exit=$RUN_EXIT out=$RUN_OUT"
    fi
}

# ── verify on an absent branch fails (exit 1) ──────────────────────────────
test_verify_absent_fails() {
    run verify --branch never-leased
    if [[ "$RUN_EXIT" -eq 1 ]]; then
        pass "verify on an absent lease fails (exit 1)"
    else
        fail "verify absent — expected exit 1, got $RUN_EXIT out=$RUN_OUT"
    fi
}

# ── expired lease: verify fails + audit emits a takeover-candidate line ─────
# THIS is fixture 11: an expired lease is the takeover trigger, surfaced by
# the lease suite (not the convergence closeout).
test_expired_lease_blocks_and_audits() {
    # Acquire with a 0-minute TTL so it is already expired.
    run acquire --branch stale-branch --owner bob --ttl-min 0
    # Force the epoch clearly into the past to avoid same-second flakiness.
    local path="$REVIEW_LEASES_DIR/stale-branch.json"
    local past=$(( $(date -u +%s) - 3600 ))
    jq --argjson e "$past" '.expires_at_epoch = $e' "$path" > "$path.tmp" && mv "$path.tmp" "$path"

    run verify --branch stale-branch
    local v=$RUN_EXIT
    run audit
    if [[ "$v" -eq 1 && "$RUN_EXIT" -eq 0 ]] && echo "$RUN_OUT" | grep -q "TAKEOVER-CANDIDATE.*stale-branch"; then
        pass "expired lease: verify fails (exit 1) AND audit emits a takeover-candidate line (route-takeover)"
    else
        fail "expired lease — verify exit=$v audit exit=$RUN_EXIT audit out=$RUN_OUT"
    fi
}

# ── a different owner cannot steal an unexpired lease ───────────────────────
test_acquire_refuses_other_owner() {
    run acquire --branch owned-branch --owner alice --ttl-min 120
    local a=$RUN_EXIT
    run acquire --branch owned-branch --owner mallory --ttl-min 120
    if [[ "$a" -eq 0 && "$RUN_EXIT" -eq 1 ]]; then
        pass "acquire refuses a different owner while the lease is unexpired (exit 1)"
    else
        fail "acquire-steal — first exit=$a second exit=$RUN_EXIT out=$RUN_OUT"
    fi
}

# ── same owner may refresh its own lease ────────────────────────────────────
test_same_owner_refreshes() {
    run acquire --branch refresh-branch --owner alice --ttl-min 120
    local a=$RUN_EXIT
    run acquire --branch refresh-branch --owner alice --ttl-min 120
    if [[ "$a" -eq 0 && "$RUN_EXIT" -eq 0 ]]; then
        pass "same owner may refresh its own unexpired lease (exit 0)"
    else
        fail "same-owner-refresh — first exit=$a second exit=$RUN_EXIT out=$RUN_OUT"
    fi
}

# ── release by the holder, then verify fails ────────────────────────────────
test_release_then_verify_fails() {
    run acquire --branch rel-branch --owner alice --ttl-min 120
    run release --branch rel-branch --owner alice
    local r=$RUN_EXIT
    run verify --branch rel-branch
    if [[ "$r" -eq 0 && "$RUN_EXIT" -eq 1 ]]; then
        pass "release by holder then verify fails (release exit 0, verify exit 1)"
    else
        fail "release/verify — release exit=$r verify exit=$RUN_EXIT out=$RUN_OUT"
    fi
}

# ── release by a non-holder is refused ──────────────────────────────────────
test_release_non_holder_refused() {
    run acquire --branch held2 --owner alice --ttl-min 120
    run release --branch held2 --owner mallory
    if [[ "$RUN_EXIT" -eq 1 ]]; then
        pass "release by a non-holder is refused (exit 1)"
    else
        fail "release-non-holder — expected exit 1, got $RUN_EXIT out=$RUN_OUT"
    fi
}

# ── written lease conforms to the review-lease schema shape ────────────────
test_lease_json_shape() {
    run acquire --branch shape-branch --owner alice --pr 7 --base-sha abc123
    local path="$REVIEW_LEASES_DIR/shape-branch.json"
    if jq -e '.v == 1 and .branch == "shape-branch" and .owner == "alice" and .pr == 7 and (.expires_at_epoch > .acquired_at_epoch) and (.base_sha == "abc123")' "$path" >/dev/null; then
        pass "written lease has the expected schema shape (v/branch/owner/pr/epochs/base_sha)"
    else
        fail "lease shape — $(cat "$path")"
    fi
}

echo "=== review-lease test suite ==="
echo ""
test_acquire_then_verify
test_verify_absent_fails
test_expired_lease_blocks_and_audits
test_acquire_refuses_other_owner
test_same_owner_refreshes
test_release_then_verify_fails
test_release_non_holder_refused
test_lease_json_shape
echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then exit 1; fi
exit 0
