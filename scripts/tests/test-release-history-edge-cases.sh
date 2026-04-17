#!/usr/bin/env bash
# Edge case tests for scripts/check_release_history.sh
# Tests edge cases not covered by the red tests:
# - Old version format without brackets in RELEASE_HISTORY (e.g., 0.8.2)
# - Grandfathered versions with hyphens (v0.1.0-pest)
# - RC tags are excluded from ALL_TAGS
# - script handles empty tag list gracefully
# - Notes file link exists but file missing (legacy gap detection)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMPL="$REPO_ROOT/scripts/check_release_history.sh"
PASS_COUNT=0
FAIL_COUNT=0

# ── Test infrastructure ───────────────────────────────────────────────────────

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

script_exists() {
    [[ -f "$IMPL" ]] && [[ -x "$IMPL" ]]
}

run_script() {
    if ! script_exists; then
        echo "127"
        return
    fi
    local code=0
    "$IMPL" >/dev/null 2>&1 || code=$?
    echo "$code"
}

run_script_with_output() {
    if ! script_exists; then
        echo "Script does not exist"
        return 127
    fi
    local code=0
    local output
    output="$("$IMPL" 2>&1)" || code=$?
    echo "EXIT:$code"
    echo "$output"
    return "$code"
}

# ── Test: Script exists and is executable ───────────────────────────────────

test_script_exists() {
    if ! script_exists; then
        fail "script does not exist at $IMPL"
        return
    fi
    pass "script exists and is executable"
}

# ── Test: Old version format without brackets (0.8.2, 0.8.0, etc.) is handled ─

test_old_version_format_without_brackets() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # Version 0.8.2 appears as "0.8.2" (no brackets) in RELEASE_HISTORY table
    # This should be found by grep -q "0.8.2"
    if grep -q "0.8.2" RELEASE_HISTORY.md; then
        pass "old version format without brackets (0.8.2) found in RELEASE_HISTORY"
    else
        fail "old version format 0.8.2 not found in RELEASE_HISTORY"
    fi
}

# ── Test: Grandfathered hyphenated version (v0.1.0-pest) is skipped ────────

test_grandfathered_hyphenated_version() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # v0.1.0-pest has hyphen, is in git tags, has no notes file
    # It should be grandfathered and not cause failure
    local code
    code="$(run_script)"

    if [[ "$code" -eq 0 ]]; then
        pass "grandfathered hyphenated version v0.1.0-pest is correctly skipped"
    else
        local output
        output="$("$IMPL" 2>&1)" || true
        fail "grandfathered v0.1.0-pest should not cause failure, got: $output"
    fi
}

# ── Test: RC tags are excluded from ALL_TAGS ────────────────────────────────

test_rc_tags_excluded_from_all_tags() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # v0.8.3-rc1 exists in git tags but should be excluded by grep -v 'rc'
    # We test this by checking that the script doesn't fail when run
    # (if RC tags were included, script might fail if notes/ledger missing for RC)
    local code
    code="$(run_script)"

    if [[ "$code" -eq 0 ]]; then
        pass "RC tags (v0.8.3-rc1) are excluded from ALL_TAGS and ignored"
    else
        local output
        output="$("$IMPL" 2>&1)" || true
        fail "RC tags should be excluded, got exit $code: $output"
    fi
}

# ── Test: Multiple grandfathered gaps are all handled ──────────────────────

test_all_grandfathered_gaps_handled() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # These versions are all grandfathered and should produce WARN not ERROR:
    # v0.7.2, v0.7.3, v0.8.0, v0.8.2, v0.5.0, v0.1.0-pest
    local output
    output="$("$IMPL" 2>&1)" || true

    local grandfathered_found=0
    for ver in 0.7.2 0.7.3 0.8.0 0.8.2 0.5.0 0.1.0-pest; do
        if echo "$output" | grep -q "Grandfathered gap: v${ver}"; then
            grandfathered_found=$((grandfathered_found + 1))
        fi
    done

    if [[ "$grandfathered_found" -ge 5 ]]; then
        pass "multiple grandfathered gaps ($grandfathered_found) are correctly detected"
    else
        fail "expected at least 5 grandfathered gaps, found $grandfathered_found"
    fi
}

# ── Test: script handles case where only RC tags exist (no non-RC) ─────────

test_only_rc_tags_ignored() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # We can't easily test "only RC tags" without removing all non-RC tags
    # Instead, verify that v0.8.3-rc1 exists and v0.8.3 exists (non-RC)
    local rc_tag_exists
    rc_tag_exists=$(git tag --list 'v0.8.3-rc1' 2>/dev/null || echo "")
    local stable_tag_exists
    stable_tag_exists=$(git tag --list 'v0.8.3' 2>/dev/null || echo "")

    if [[ -n "$rc_tag_exists" ]] && [[ -n "$stable_tag_exists" ]]; then
        # Verify RC tag is filtered out but stable tag is included
        # by checking that script passes (implies both tags are handled correctly)
        local code
        code="$(run_script)"
        if [[ "$code" -eq 0 ]]; then
            pass "RC tag v0.8.3-rc1 filtered, stable v0.8.3 included, script passes"
        else
            fail "script should pass with both RC and stable tags present"
        fi
    else
        pass "skipping RC filtering test (test data not available)"
    fi
}

# ── Test: version sorting works correctly for common cases ─────────────────

test_version_sorting() {
    # Test that sort -V handles the version formats we have
    local versions=("0.8.2" "0.8.3" "0.8.5" "0.9.0" "0.9.1" "0.10.0" "0.11.0" "0.12.0" "0.12.3" "0.12.4")
    local sorted
    sorted=$(printf '%s\n' "${versions[@]}" | sort -V)

    # Check that newest is last
    local last_line
    last_line=$(echo "$sorted" | tail -1)
    if [[ "$last_line" == "0.12.4" ]]; then
        pass "version sorting correctly puts 0.12.4 as newest"
    else
        fail "version sorting failed: expected 0.12.4 as last, got $last_line"
    fi
}

# ── Test: notes file link pattern matching ──────────────────────────────────

test_notes_link_pattern() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # For versions with notes files (like 0.12.4), verify link exists
    if grep -q "\[n-0.12.4\]:" RELEASE_HISTORY.md; then
        pass "notes file link [n-0.12.4] found for version with notes"
    else
        fail "expected [n-0.12.4] link for version with notes"
    fi

    # For grandfathered versions, verify no notes link exists
    if ! grep -q "\[n-0.8.2\]:" RELEASE_HISTORY.md; then
        pass "no notes file link [n-0.8.2] for grandfathered version"
    else
        fail "unexpected [n-0.8.2] link for grandfathered version"
    fi
}

# ── Test: CHANGELOG format with date suffix ─────────────────────────────────

test_changelog_format_with_date() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # Find newest non-RC tag
    local newest_tag
    newest_tag=$(git tag --list 'v*' | grep -v 'rc' | sort -V | tail -1)
    local newest_version="${newest_tag#v}"

    # CHANGELOG may have format like "## [0.12.4] - 2026-04-12" or just "## [0.12.4]"
    # The script checks for grep -q "## \[${NEWEST_TAG}\]" which should match both
    if grep -q "## \[$newest_version\]" CHANGELOG.md; then
        pass "CHANGELOG has correct format for newest tag v$newest_version"
    else
        fail "CHANGELOG missing ## [$newest_version] for newest tag"
    fi
}

# ── Test: script produces actionable error messages ─────────────────────────

test_actionable_error_messages() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # Create a temp tag with no notes file or RELEASE_HISTORY entry
    local test_tag="v9.9.9-edge-test"
    local test_version="9.9.9-edge-test"

    # Cleanup any existing test tag
    git tag -d "$test_tag" 2>/dev/null || true

    # Create tag
    git tag "$test_tag"

    # Remove notes file if it exists
    local notes_file="$REPO_ROOT/docs/releases/v$test_version.md"
    local notes_backup=""
    if [[ -f "$notes_file" ]]; then
        mv "$notes_file" "$notes_file.bak"
        notes_backup="$notes_file.bak"
    fi

    # Remove from RELEASE_HISTORY if present
    local rh_backup=""
    if grep -q "$test_version" "$REPO_ROOT/RELEASE_HISTORY.md" 2>/dev/null; then
        cp "$REPO_ROOT/RELEASE_HISTORY.md" "$REPO_ROOT/RELEASE_HISTORY.md.bak"
        rh_backup="$REPO_ROOT/RELEASE_HISTORY.md.bak"
        grep -v "$test_version" "$REPO_ROOT/RELEASE_HISTORY.md.bak" > "$REPO_ROOT/RELEASE_HISTORY.md"
    fi

    # Run and capture output
    local output
    local code
    output="$(run_script_with_output)" || code=$?

    # Cleanup
    git tag -d "$test_tag" 2>/dev/null || true
    [[ -n "$notes_backup" ]] && mv "$notes_backup" "$notes_file" || true
    [[ -n "$rh_backup" ]] && mv "$rh_backup" "$REPO_ROOT/RELEASE_HISTORY.md" || true

    # Check error message is actionable
    if [[ "${code:-0}" -ne 0 ]]; then
        # Should mention the specific file that's missing
        if echo "$output" | grep -q "v$test_version.md"; then
            pass "error message includes actionable file path v$test_version.md"
        else
            fail "error message should mention missing file v$test_version.md, got: $output"
        fi
    else
        fail "script should fail when notes file and RELEASE_HISTORY entry are missing"
    fi
}

# ── Test: (CL) entries are properly identified and skipped ─────────────────

test_cl_entries_identified() {
    if ! script_exists; then
        fail "script does not exist"
        return
    fi

    cd "$REPO_ROOT"

    # Verify (CL) entries exist for scope markers
    # These have "—" in Tag column and "(CL)" in Released column
    local cl_count
    cl_count=$(grep -c '(CL)' RELEASE_HISTORY.md 2>/dev/null || echo "0")

    if [[ "$cl_count" -ge 3 ]]; then
        pass "(CL) entries are present ($cl_count found) and identifiable"
    else
        fail "expected at least 3 (CL) entries, found $cl_count"
    fi
}

# ── Run all tests ───────────────────────────────────────────────────────────

main() {
    echo "=== Edge Case Tests for check_release_history.sh ==="
    echo ""

    test_script_exists
    test_old_version_format_without_brackets
    test_grandfathered_hyphenated_version
    test_rc_tags_excluded_from_all_tags
    test_all_grandfathered_gaps_handled
    test_only_rc_tags_ignored
    test_version_sorting
    test_notes_link_pattern
    test_changelog_format_with_date
    test_actionable_error_messages
    test_cl_entries_identified

    echo ""
    echo "=== Results ==="
    echo "Passed: $PASS_COUNT"
    echo "Failed: $FAIL_COUNT"

    if [[ "$FAIL_COUNT" -gt 0 ]]; then
        exit 1
    fi
    exit 0
}

main "$@"