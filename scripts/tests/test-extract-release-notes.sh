#!/usr/bin/env bash
# Self-test for scripts/extract-release-notes.sh.
#
# Problem it prevents: the release workflow sources its GitHub Release body
# from docs/releases/vX.Y.Z.md. If the front-matter stripper silently
# malfunctions (e.g. eats the first paragraph, fails to detect a missing
# file, or passes through the YAML block into the release body) the public
# release notes are wrong on a channel that is painful to correct.
#
# This test verifies five properties of the extractor:
#
#   CASE 1 - Valid file with front-matter: body is emitted, front-matter is
#             stripped, and the first output line is the markdown heading.
#
#   CASE 2 - Missing file: script exits with code 2 and a clear error.
#
#   CASE 3 - Body-only file (no front-matter): body is emitted verbatim.
#
#   CASE 4 - Front-matter only (empty body): script exits with code 3.
#
#   CASE 5 - "v" prefix is tolerated: "v0.12.4" resolves the same file as
#             "0.12.4".
#
# The extractor path is resolved relative to this test's location so the
# test runs in CI, under Nix, and locally without environment setup.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
EXTRACTOR="${REPO_ROOT}/scripts/extract-release-notes.sh"

PASS=0
FAIL=0
TMPDIR_BASE=""

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" && -d "${TMPDIR_BASE}" ]]; then
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

TMPDIR_BASE="$(mktemp -d)"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

assert_eq() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" == "$actual" ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      expected: ${expected}"
    echo "      actual:   ${actual}"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" -eq "$actual" ]]; then
    echo "PASS  ${label} (exit ${actual})"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label} (expected exit ${expected}, got ${actual})"
    FAIL=$((FAIL + 1))
  fi
}

assert_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      expected to contain: ${needle}"
    echo "      got:"
    printf '      %s\n' "$haystack" | head -5
    FAIL=$((FAIL + 1))
  fi
}

assert_not_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      expected NOT to contain: ${needle}"
    FAIL=$((FAIL + 1))
  fi
}

# Build a fake repo root with controlled fixtures so tests don't depend on
# the real docs/releases/ content (which changes over time).
FAKE_REPO="${TMPDIR_BASE}/repo"
mkdir -p "${FAKE_REPO}/docs/releases"

# CASE 1 fixture: valid front-matter + body
cat > "${FAKE_REPO}/docs/releases/v1.2.3.md" <<'FIXTURE_EOF'
---
version: "1.2.3"
tag: "v1.2.3"
release_date_utc: "2026-01-01"
---

# v1.2.3

## Summary

First release of the fixture.

## Highlights

- Thing one
- Thing two
FIXTURE_EOF

# CASE 3 fixture: body only, no front-matter
cat > "${FAKE_REPO}/docs/releases/v4.5.6.md" <<'FIXTURE_EOF'
# v4.5.6

Body with no front-matter at all.
FIXTURE_EOF

# CASE 4 fixture: front-matter only, no body
cat > "${FAKE_REPO}/docs/releases/v7.8.9.md" <<'FIXTURE_EOF'
---
version: "7.8.9"
---
FIXTURE_EOF

# ---------------------------------------------------------------------------
# CASE 1: valid file with front-matter
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 1: valid file with front-matter ==="

CASE1_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" 1.2.3 2>&1)"
CASE1_EXIT=$?

assert_exit "exits 0 on valid file" 0 "${CASE1_EXIT}"
assert_contains "body contains markdown heading" "# v1.2.3" "${CASE1_OUT}"
assert_contains "body contains Summary section" "## Summary" "${CASE1_OUT}"
assert_contains "body contains Highlights bullet" "- Thing one" "${CASE1_OUT}"
assert_not_contains "front-matter version key is stripped" 'version: "1.2.3"' "${CASE1_OUT}"
assert_not_contains "front-matter tag key is stripped" 'tag: "v1.2.3"' "${CASE1_OUT}"
assert_not_contains "front-matter delimiter is stripped" $'---\n' "${CASE1_OUT}"

FIRST_LINE="$(printf '%s\n' "${CASE1_OUT}" | head -1)"
assert_eq "first line is the markdown heading" "# v1.2.3" "${FIRST_LINE}"

# ---------------------------------------------------------------------------
# CASE 2: missing file
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 2: missing release notes file ==="

CASE2_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" 0.0.0 2>&1)"
CASE2_EXIT=$?

assert_exit "exits 2 on missing file" 2 "${CASE2_EXIT}"
assert_contains "error mentions the missing path" "docs/releases/v0.0.0.md" "${CASE2_OUT}"
assert_contains "error points to RELEASE.md" "RELEASE.md" "${CASE2_OUT}"

# ---------------------------------------------------------------------------
# CASE 3: body-only file (no front-matter)
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 3: body-only file (no front-matter) ==="

CASE3_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" 4.5.6 2>&1)"
CASE3_EXIT=$?

assert_exit "exits 0 on body-only file" 0 "${CASE3_EXIT}"
assert_contains "body passes through verbatim" "# v4.5.6" "${CASE3_OUT}"
assert_contains "body text is preserved" "Body with no front-matter at all." "${CASE3_OUT}"

# ---------------------------------------------------------------------------
# CASE 4: front-matter only (empty body)
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 4: front-matter only (empty body) ==="

CASE4_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" 7.8.9 2>&1)"
CASE4_EXIT=$?

assert_exit "exits 3 on empty body" 3 "${CASE4_EXIT}"
assert_contains "error mentions no body content" "no body content" "${CASE4_OUT}"

# ---------------------------------------------------------------------------
# CASE 5: tag-prefixed version is accepted
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 5: tag-prefixed version (v1.2.3) ==="

CASE5_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" v1.2.3 2>&1)"
CASE5_EXIT=$?

assert_exit "exits 0 for v-prefixed version" 0 "${CASE5_EXIT}"
assert_eq "v-prefixed and bare version produce identical output" "${CASE1_OUT}" "${CASE5_OUT}"

# ---------------------------------------------------------------------------
# CASE 6: outfile argument is honoured
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 6: outfile argument ==="

OUTFILE="${TMPDIR_BASE}/case6.md"
REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" 1.2.3 "${OUTFILE}" > /dev/null 2>&1
CASE6_EXIT=$?

assert_exit "exits 0 when writing to outfile" 0 "${CASE6_EXIT}"
if [[ -s "${OUTFILE}" ]]; then
  echo "PASS  outfile is non-empty"
  PASS=$((PASS + 1))
  FILE_CONTENT="$(cat "${OUTFILE}")"
  assert_contains "outfile contains body heading" "# v1.2.3" "${FILE_CONTENT}"
else
  echo "FAIL  outfile is empty or missing: ${OUTFILE}"
  FAIL=$((FAIL + 1))
fi

# ---------------------------------------------------------------------------
# CASE 7: malformed version rejected with usage-class exit
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 7: malformed version argument ==="

CASE7_OUT="$(REPO_ROOT="${FAKE_REPO}" bash "${EXTRACTOR}" "not-a-version" 2>&1)"
CASE7_EXIT=$?

assert_exit "exits 1 on malformed version" 1 "${CASE7_EXIT}"
assert_contains "error mentions invalid format" "Invalid version format" "${CASE7_OUT}"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "${FAIL}" -gt 0 ]]; then
  echo "FAIL: ${FAIL} assertion(s) failed."
  exit 1
fi

echo "All assertions passed — the release notes extractor does what it claims."
exit 0
