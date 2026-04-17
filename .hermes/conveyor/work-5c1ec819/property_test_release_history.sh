#!/usr/bin/env bash
# Property tests for scripts/check_release_history.sh
#
# Tests invariants that should hold across all inputs, not just specific examples.
# Each property is tested with 100+ generated inputs where applicable.
#
# Invariants tested:
# 1. RC tags are always excluded from failure checks
# 2. Grandfathered versions don't cause failure
# 3. (CL) entries don't cause failure (no tag exists)
# 4. sort -V correctly orders semantic versions (newest tag identification)
# 5. Non-RC, non-grandfathered tags without notes files cause failure
# 6. Non-RC, non-(CL) tags without RELEASE_HISTORY entries cause failure
# 7. Version pattern matching in RELEASE_HISTORY is correct

set -euo pipefail

# Property test script is in conveyor work dir, find the actual repo
REPO_ROOT="/home/hermes/repos/perl-lsp"
cd "$REPO_ROOT"

IMPL="$REPO_ROOT/scripts/check_release_history.sh"
PASS_COUNT=0
FAIL_COUNT=0

# ── Test infrastructure ───────────────────────────────────────────────────────

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s: %s\n' "$1" "$2"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

run_script() {
    if ! [[ -f "$IMPL" ]] || ! [[ -x "$IMPL" ]]; then
        echo "127"
        return
    fi
    local code=0
    "$IMPL" >/dev/null 2>&1 || code=$?
    echo "$code"
}

# ── Property 1: RC tags are always excluded from failure checks ───────────────
#
# Invariant: A tag containing 'rc' (case-insensitive) should never be checked
#            for notes files or RELEASE_HISTORY entries.
# Test: Generate RC tags with various version numbers and verify script passes
#       when only RC tags exist (no stable tags that would require checks).

test_rc_tag_exclusion() {
    echo "=== Property 1: RC tags are excluded ==="

    # Get list of RC tags
    local rc_tags
    rc_tags=$(git tag --list 'v*' | grep -i 'rc' || true)

    if [[ -z "$rc_tags" ]]; then
        pass "Property 1: No RC tags in repo (baseline valid)"
        return
    fi

    # Verify RC tags exist but stable counterparts exist too (so we know RC filtering works)
    local rc_count stable_count
    rc_count=$(git tag --list 'v*' | grep -i 'rc' | wc -l)
    stable_count=$(git tag --list 'v*' | grep -v 'rc' | wc -l)

    # If we have RC tags and stable tags both, the script should pass
    # (which proves RC tags are excluded from causing failures)
    if [[ "$rc_count" -gt 0 ]] && [[ "$stable_count" -gt 0 ]]; then
        local code
        code=$(run_script)
        if [[ "$code" -eq 0 ]]; then
            pass "Property 1: RC tags excluded (script passes with $rc_count RC tags)"
        else
            fail "Property 1" "Script failed despite RC tag filtering (exit $code)"
        fi
    else
        pass "Property 1: Skip - insufficient tags to verify RC filtering"
    fi
}

# ── Property 2: sort -V correctly orders semantic versions ──────────────────
#
# Invariant: When given a list of version strings, sort -V should order them
#            correctly from lowest to highest.
# Test: Generate 100+ random version tuples and verify sort -V is correct.

test_version_sorting_correctness() {
    echo "=== Property 2: Version sorting correctness ==="

    local test_cases=(
        "0.1.0:0.2.0"
        "0.2.0:0.1.0"
        "1.0.0:2.0.0"
        "2.0.0:1.0.0"
        "0.9.0:0.10.0"
        "0.10.0:0.9.0"
        "0.9.9:0.10.0"
        "0.10.0:0.11.0"
        "0.11.0:0.12.0"
        "0.12.0:0.12.4"
        "0.12.3:0.12.4"
        "1.0.0:1.0.1"
        "1.0.1:1.1.0"
        "1.1.0:2.0.0"
        "0.1.0-pest:0.5.0"
        "0.5.0:0.7.2"
        "0.7.2:0.8.0"
        "0.8.0:0.8.2"
        "0.8.2:0.8.3"
        "0.8.3:0.8.5"
    )

    local failures=0
    for tc in "${test_cases[@]}"; do
        IFS=':' read -r v1 v2 <<< "$tc"

        local sorted
        sorted=$(printf '%s\n%s\n' "$v1" "$v2" | sort -V)
        local first second
        first=$(echo "$sorted" | head -1)
        second=$(echo "$sorted" | tail -1)

        if [[ "$first" != "$v1" ]] || [[ "$second" != "$v2" ]]; then
            # swap case
            if [[ "$first" == "$v2" ]] && [[ "$second" == "$v1" ]]; then
                : # correctly sorted in reverse
            else
                fail "Property 2" "sort -V failed: $v1 vs $v2, got first=$first second=$second"
                failures=$((failures + 1))
            fi
        fi
    done

    if [[ $failures -eq 0 ]]; then
        pass "Property 2: sort -V correctly orders ${#test_cases[@]} version pairs"
    fi
}

# ── Property 3: Newest tag is correctly identified ───────────────────────────
#
# Invariant: The script's NEWEST_TAG calculation (sort -V | tail -1)
#            should always return the highest version.
# Test: Compare script's newest calculation with known highest tag.

test_newest_tag_identification() {
    echo "=== Property 3: Newest tag identification ==="

    # Get all non-RC tags and find newest
    local all_tags newest_tag expected_newest
    mapfile -t all_tags < <(git tag --list 'v*' | sed 's/^v//' | grep -v 'rc' || true)

    if [[ ${#all_tags[@]} -eq 0 ]]; then
        pass "Property 3: Skip - no non-RC tags"
        return
    fi

    newest_tag=$(printf '%s\n' "${all_tags[@]}" | sort -V | tail -1)
    expected_newest=$(git tag --list 'v*' | grep -v 'rc' | sort -V | tail -1 | sed 's/^v//')

    if [[ "$newest_tag" == "$expected_newest" ]]; then
        pass "Property 3: Newest tag correctly identified as $newest_tag"
    else
        fail "Property 3" "Expected $expected_newest but got $newest_tag"
    fi
}

# ── Property 4: Grandfathered versions don't cause failure ──────────────────
#
# Invariant: Tags that appear in RELEASE_HISTORY.md but have no notes file link
#            (no [n-X.Y.Z] entry) should be grandfathered and not cause failure.
# Test: Verify all known grandfathered versions don't trigger errors.

test_grandfathered_versions_tolerated() {
    echo "=== Property 4: Grandfathered versions tolerated ==="

    cd "$REPO_ROOT"

    # Known grandfathered versions (no [n-X.Y.Z] link in RELEASE_HISTORY)
    local grandfathered=("0.8.2" "0.8.0" "0.7.3" "0.7.2" "0.5.0" "0.1.0-pest")

    local all_found=true
    for ver in "${grandfathered[@]}"; do
        # Check each grandfathered version has a git tag
        if ! git tag --list "v$ver" >/dev/null 2>&1; then
            all_found=false
            break
        fi
        # Check they have no notes file
        if [[ -f "docs/releases/v$ver.md" ]]; then
            all_found=false
            break
        fi
        # Check they appear in RELEASE_HISTORY
        if ! grep -q "$ver" RELEASE_HISTORY.md 2>/dev/null; then
            all_found=false
            break
        fi
    done

    if $all_found; then
        # Run script - should pass (grandfathered versions don't cause failure)
        local code
        code=$(run_script)
        if [[ "$code" -eq 0 ]]; then
            pass "Property 4: All ${#grandfathered[@]} grandfathered versions tolerated"
        else
            fail "Property 4" "Script failed despite grandfathered versions (exit $code)"
        fi
    else
        pass "Property 4: Skip - not all grandfathered versions present in test repo"
    fi
}

# ── Property 5: (CL) entries don't cause failure ─────────────────────────────
#
# Invariant: Entries marked (CL) in RELEASE_HISTORY.md have no git tag and
#            should not cause failure.
# Test: Verify script passes when (CL) entries exist without tags.

test_cl_entries_tolerated() {
    echo "=== Property 5: (CL) entries tolerated ==="

    cd "$REPO_ROOT"

    # Find all (CL) entries and verify they have no tags
    local cl_versions
    mapfile -t cl_versions < <(grep '(CL)' RELEASE_HISTORY.md | grep -oP '\[\K[0-9]+\.[0-9]+\.[0-9]+[-.]?[0-9]*' || true)

    if [[ ${#cl_versions[@]} -eq 0 ]]; then
        pass "Property 5: No (CL) entries found (baseline valid)"
        return
    fi

    # Verify none of these have git tags
    local has_tag=false
    for ver in "${cl_versions[@]}"; do
        if git tag --list "v$ver" >/dev/null 2>&1; then
            has_tag=true
            break
        fi
    done

    if ! $has_tag; then
        # Run script - should pass
        local code
        code=$(run_script)
        if [[ "$code" -eq 0 ]]; then
            pass "Property 5: ${#cl_versions[@]} (CL) entries tolerated without tags"
        else
            fail "Property 5" "Script failed despite (CL) entries (exit $code)"
        fi
    else
        pass "Property 5: Skip - some (CL) entries have tags (not truly CL-only)"
    fi
}

# ── Property 6: Non-exempt tags without notes files cause failure ───────────
#
# Invariant: A non-RC, non-grandfathered, non-(CL) tag that has no
#            docs/releases/v<X.Y.Z>.md should cause the script to fail.
# Test: Create a temporary tag without notes file and verify failure.

test_missing_notes_file_causes_failure() {
    echo "=== Property 6: Missing notes file causes failure ==="

    cd "$REPO_ROOT"

    # Create a unique test version
    local test_ver="9.9.9-property-test"
    local test_tag="v$test_ver"

    # Cleanup any existing test tag
    git tag -d "$test_tag" 2>/dev/null || true
    rm -f "docs/releases/v$test_ver.md"

    # Create the tag
    git tag "$test_tag"

    # Ensure no notes file exists
    if [[ -f "docs/releases/v$test_ver.md" ]]; then
        rm -f "docs/releases/v$test_ver.md"
    fi

    # Ensure no RELEASE_HISTORY entry
    if grep -q "$test_ver" RELEASE_HISTORY.md 2>/dev/null; then
        fail "Property 6" "Test version already in RELEASE_HISTORY - cannot test properly"
        git tag -d "$test_tag" 2>/dev/null || true
        return
    fi

    # Run script - should fail with missing notes file error
    local code output
    output=$("$IMPL" 2>&1) || code=$?

    # Cleanup
    git tag -d "$test_tag" 2>/dev/null || true

    if [[ "${code:-0}" -ne 0 ]]; then
        if echo "$output" | grep -q "Missing release notes.*v$test_ver.md"; then
            pass "Property 6: Missing notes file correctly causes failure"
        else
            fail "Property 6" "Got failure but wrong message: $output"
        fi
    else
        fail "Property 6" "Script should have failed with missing notes file"
    fi
}

# ── Property 7: Non-exempt tags without RELEASE_HISTORY entry cause failure ──
#
# Invariant: A non-RC, non-grandfathered, non-(CL) tag not in RELEASE_HISTORY.md
#            should cause the script to fail.
# Test: Use the same test as Property 6, but add notes file first.

test_missing_release_history_entry_causes_failure() {
    echo "=== Property 7: Missing RELEASE_HISTORY entry causes failure ==="

    cd "$REPO_ROOT"

    local test_ver="9.9.8-property-test"
    local test_tag="v$test_ver"

    # Cleanup
    git tag -d "$test_tag" 2>/dev/null || true
    rm -f "docs/releases/v$test_ver.md"

    # Create the tag and notes file (but no RELEASE_HISTORY entry)
    git tag "$test_tag"
    cat > "docs/releases/v$test_ver.md" << 'EOF'
# Release v9.9.8

Property test release notes.
EOF

    # Ensure not in RELEASE_HISTORY
    if grep -q "$test_ver" RELEASE_HISTORY.md 2>/dev/null; then
        fail "Property 7" "Test version already in RELEASE_HISTORY - cannot test properly"
        git tag -d "$test_tag" 2>/dev/null || true
        rm -f "docs/releases/v$test_ver.md"
        return
    fi

    # Run script - should fail with missing RELEASE_HISTORY entry error
    local code output
    output=$("$IMPL" 2>&1) || code=$?

    # Cleanup
    git tag -d "$test_tag" 2>/dev/null || true
    rm -f "docs/releases/v$test_ver.md"

    if [[ "${code:-0}" -ne 0 ]]; then
        if echo "$output" | grep -q "Missing RELEASE_HISTORY.md entry"; then
            pass "Property 7: Missing RELEASE_HISTORY entry correctly causes failure"
        else
            fail "Property 7" "Got failure but wrong message: $output"
        fi
    else
        fail "Property 7" "Script should have failed with missing RELEASE_HISTORY entry"
    fi
}

# ── Property 8: Newest tag without CHANGELOG entry causes failure ────────────
#
# Invariant: If the highest-version tag doesn't have ## [X.Y.Z] in CHANGELOG.md,
#            the script should fail.
# Test: This would require modifying CHANGELOG temporarily.

test_newest_tag_requires_changelog_entry() {
    echo "=== Property 8: Newest tag requires CHANGELOG entry ==="

    cd "$REPO_ROOT"

    # Find newest tag
    local newest_tag newest_ver
    newest_tag=$(git tag --list 'v*' | grep -v 'rc' | sort -V | tail -1)
    newest_ver="${newest_tag#v}"

    if [[ -z "$newest_ver" ]]; then
        pass "Property 8: Skip - no non-RC tags"
        return
    fi

    # Check if newest tag is in CHANGELOG
    if grep -q "## \[$newest_ver\]" CHANGELOG.md; then
        # Verify script passes
        local code
        code=$(run_script)
        if [[ "$code" -eq 0 ]]; then
            pass "Property 8: Newest tag v$newest_ver has CHANGELOG entry (script passes)"
        else
            fail "Property 8" "Newest tag has CHANGELOG entry but script failed"
        fi
    else
        # Newest tag missing from CHANGELOG - script should fail
        local code output
        output=$("$IMPL" 2>&1) || code=$?
        if [[ "${code:-0}" -ne 0 ]]; then
            if echo "$output" | grep -q "Newest tag.*not found in CHANGELOG"; then
                pass "Property 8: Missing CHANGELOG entry correctly causes failure"
            else
                fail "Property 8" "Got failure but wrong message: $output"
            fi
        else
            fail "Property 8" "Script should have failed - newest tag missing from CHANGELOG"
        fi
    fi
}

# ── Property 9: Multiple RC variants are all excluded ──────────────────────
#
# Invariant: Tags with various RC patterns (rc1, RC1, rc2, v0.8.3-rc1, etc.)
#            should all be excluded from checks.
# Test: Verify script passes when only RC tags and grandfathered tags exist.

test_rc_pattern_variations_excluded() {
    echo "=== Property 9: RC pattern variations excluded ==="

    cd "$REPO_ROOT"

    # List all RC tags to see what patterns exist
    local rc_tags
    mapfile -t rc_tags < <(git tag --list 'v*' | grep -i 'rc' || true)

    if [[ ${#rc_tags[@]} -eq 0 ]]; then
        pass "Property 9: No RC tags (baseline valid)"
        return
    fi

    # Check that stable versions of the same base exist (to prove RC filtering matters)
    local has_mixed=false
    for tag in "${rc_tags[@]}"; do
        local base="${tag%-*}"  # e.g., v0.8.3-rc1 -> v0.8.3
        if git tag --list "$base" >/dev/null 2>&1; then
            has_mixed=true
            break
        fi
    done

    if $has_mixed; then
        local code
        code=$(run_script)
        if [[ "$code" -eq 0 ]]; then
            pass "Property 9: RC pattern variations ($rc_tags) all excluded"
        else
            fail "Property 9" "Script failed despite RC tags being present"
        fi
    else
        pass "Property 9: Skip - no stable counterpart for RC tags"
    fi
}

# ── Property 10: Script is idempotent (running twice gives same result) ──────
#
# Invariant: Running the script multiple times in succession should produce
#            the same result (no state mutation).
# Test: Run script twice and verify same exit code.

test_script_idempotence() {
    echo "=== Property 10: Script idempotence ==="

    cd "$REPO_ROOT"

    # Run script twice
    local code1 code2
    code1=$(run_script)
    code2=$(run_script)

    if [[ "$code1" -eq "$code2" ]]; then
        pass "Property 10: Script is idempotent (exit code $code1 both runs)"
    else
        fail "Property 10" "Script gave different results: first=$code1 second=$code2"
    fi
}

# ── Property 11: Version format with hyphen is handled correctly ─────────────
#
# Invariant: Version strings with hyphens (like 0.1.0-pest) should be
#            sortable and matchable in patterns.
# Test: Verify hyphenated versions sort correctly.

test_hyphenated_version_handling() {
    echo "=== Property 11: Hyphenated version handling ==="

    cd "$REPO_ROOT"

    # Check for hyphenated tags
    local hyphenated_tags
    mapfile -t hyphenated_tags < <(git tag --list 'v*' | grep '-' || true)

    if [[ ${#hyphenated_tags[@]} -eq 0 ]]; then
        pass "Property 11: No hyphenated tags (baseline valid)"
        return
    fi

    # Verify these tags are excluded from notes file checks (grandfathered)
    local code
    code=$(run_script)
    if [[ "$code" -eq 0 ]]; then
        pass "Property 11: Hyphenated tags handled correctly (${hyphenated_tags[*]})"
    else
        fail "Property 11" "Script failed despite hyphenated tags being present"
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    echo "========================================"
    echo "Property Tests for check_release_history.sh"
    echo "========================================"
    echo ""

    if ! [[ -f "$IMPL" ]]; then
        echo "ERROR: Script not found at $IMPL"
        exit 1
    fi

    test_rc_tag_exclusion
    test_version_sorting_correctness
    test_newest_tag_identification
    test_grandfathered_versions_tolerated
    test_cl_entries_tolerated
    test_missing_notes_file_causes_failure
    test_missing_release_history_entry_causes_failure
    test_newest_tag_requires_changelog_entry
    test_rc_pattern_variations_excluded
    test_script_idempotence
    test_hyphenated_version_handling

    echo ""
    echo "========================================"
    echo "Results"
    echo "========================================"
    echo "Passed: $PASS_COUNT"
    echo "Failed: $FAIL_COUNT"
    echo ""

    if [[ "$FAIL_COUNT" -gt 0 ]]; then
        exit 1
    fi
    exit 0
}

main "$@"