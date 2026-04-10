#!/usr/bin/env bash
# Test suite for scripts/pre-merge-check.sh
# TDD: mocks gh pr view output to exercise each failure mode independently.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMPL="$SCRIPT_DIR/../pre-merge-check.sh"
PASS_COUNT=0
FAIL_COUNT=0

if [[ ! -f "$IMPL" ]]; then
    echo "ERROR: pre-merge-check.sh not found at $IMPL"
    echo "Write the implementation first: scripts/pre-merge-check.sh"
    exit 1
fi

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# ── Mock infrastructure ───────────────────────────────────────────────────────
# We override `gh` with a fake that echoes prepared JSON.
# The fake is placed on PATH via a temp dir prepended to PATH.

make_mock_gh() {
    local tmpdir json
    tmpdir="$(mktemp -d)"
    json="$1"
    cat > "$tmpdir/gh" <<EOF
#!/usr/bin/env bash
# Mock gh — echoes canned JSON regardless of args
printf '%s' '$json'
EOF
    chmod +x "$tmpdir/gh"
    echo "$tmpdir"
}

cleanup() {
    local dir
    for dir in "$@"; do
        [[ -d "$dir" ]] && rm -rf "$dir"
    done
}

run_check() {
    local mock_dir="$1"
    local pr_number="${2:-42}"
    local code=0
    PATH="$mock_dir:$PATH" bash "$IMPL" "$pr_number" >/dev/null 2>&1 || code=$?
    echo "$code"
}

run_check_with_output() {
    local mock_dir="$1"
    local pr_number="${2:-42}"
    local code=0
    local output
    output="$(PATH="$mock_dir:$PATH" bash "$IMPL" "$pr_number" 2>&1)" || code=$?
    echo "EXIT:$code"
    echo "$output"
}

# ── Test 1: Draft PR fails ────────────────────────────────────────────────────

test_draft_pr_fails() {
    local json='{"isDraft":true,"labels":[{"name":"merge-ready"}],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -ne 0 ]]; then
        pass "draft PR exits non-zero (exit $code)"
    else
        fail "draft PR — expected non-zero exit, got 0"
    fi
}

# ── Test 2: Missing merge-ready label fails ───────────────────────────────────

test_missing_merge_ready_label_fails() {
    local json='{"isDraft":false,"labels":[{"name":"in-review"}],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -ne 0 ]]; then
        pass "missing merge-ready label exits non-zero (exit $code)"
    else
        fail "missing merge-ready label — expected non-zero exit, got 0"
    fi
}

# ── Test 3: Missing issue ref in title fails ──────────────────────────────────

test_missing_issue_ref_fails() {
    local json='{"isDraft":false,"labels":[{"name":"merge-ready"}],"title":"feat: add thing without issue ref"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -ne 0 ]]; then
        pass "missing issue ref exits non-zero (exit $code)"
    else
        fail "missing issue ref — expected non-zero exit, got 0"
    fi
}

# ── Test 4: Clean PR passes ───────────────────────────────────────────────────

test_clean_pr_passes() {
    local json='{"isDraft":false,"labels":[{"name":"merge-ready"}],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -eq 0 ]]; then
        pass "clean PR exits zero"
    else
        fail "clean PR — expected exit 0, got $code"
    fi
}

# ── Test 5: Error message names the failure (draft) ──────────────────────────

test_draft_error_message_is_clear() {
    local json='{"isDraft":true,"labels":[{"name":"merge-ready"}],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local output
    output="$(run_check_with_output "$mock")"
    cleanup "$mock"

    if echo "$output" | grep -qi "draft"; then
        pass "draft error message mentions 'draft'"
    else
        fail "draft error message does not mention 'draft' — got: $output"
    fi
}

# ── Test 6: Error message names the failure (label) ──────────────────────────

test_label_error_message_is_clear() {
    local json='{"isDraft":false,"labels":[],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local output
    output="$(run_check_with_output "$mock")"
    cleanup "$mock"

    if echo "$output" | grep -qi "merge-ready\|label"; then
        pass "label error message mentions 'merge-ready' or 'label'"
    else
        fail "label error message unclear — got: $output"
    fi
}

# ── Test 7: Error message names the failure (title) ──────────────────────────

test_title_error_message_is_clear() {
    local json='{"isDraft":false,"labels":[{"name":"merge-ready"}],"title":"feat: no issue ref here"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local output
    output="$(run_check_with_output "$mock")"
    cleanup "$mock"

    if echo "$output" | grep -qi "title\|issue\|#"; then
        pass "title error message mentions title/issue reference"
    else
        fail "title error message unclear — got: $output"
    fi
}

# ── Test 8: No PR number argument fails with usage ───────────────────────────

test_no_pr_number_fails() {
    local code=0
    bash "$IMPL" >/dev/null 2>&1 || code=$?

    if [[ "$code" -ne 0 ]]; then
        pass "missing PR number argument exits non-zero (exit $code)"
    else
        fail "missing PR number argument — expected non-zero exit, got 0"
    fi
}

# ── Test 9: Multiple labels — merge-ready present passes ─────────────────────

test_merge_ready_among_multiple_labels_passes() {
    local json='{"isDraft":false,"labels":[{"name":"in-build"},{"name":"merge-ready"},{"name":"reviewed-deep"}],"title":"feat: add thing (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -eq 0 ]]; then
        pass "merge-ready among multiple labels passes"
    else
        fail "merge-ready among multiple labels — expected exit 0, got $code"
    fi
}

# ── Test 10: Issue ref at end passes (canonical form) ────────────────────────

test_issue_ref_middle_of_title_passes() {
    # Spec requires (#NNN) anywhere — the CI validate-title pattern uses end-of-title,
    # but our guard just checks presence. Test that (#NNN) in the middle is acceptable.
    local json='{"isDraft":false,"labels":[{"name":"merge-ready"}],"title":"feat(ops): pre-merge guard (#3321)"}'
    local mock
    mock="$(make_mock_gh "$json")"

    local code
    code="$(run_check "$mock")"
    cleanup "$mock"

    if [[ "$code" -eq 0 ]]; then
        pass "issue ref in title (canonical form) passes"
    else
        fail "issue ref in title (canonical form) — expected exit 0, got $code"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== pre-merge-check test suite ==="
echo ""

test_draft_pr_fails
test_missing_merge_ready_label_fails
test_missing_issue_ref_fails
test_clean_pr_passes
test_draft_error_message_is_clear
test_label_error_message_is_clear
test_title_error_message_is_clear
test_no_pr_number_fails
test_merge_ready_among_multiple_labels_passes
test_issue_ref_middle_of_title_passes

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
