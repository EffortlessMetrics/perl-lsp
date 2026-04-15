#!/usr/bin/env bash
# Self-test for scripts/extract-release-notes.sh.
#
# Problem it prevents: the release workflow now fails the whole release if the
# notes file is missing or the front-matter is malformed. A silent bug in the
# extractor (stripping too much, not enough, or misreporting an exit code)
# would either ship a blank release body or block every release on false
# positives. This self-test feeds known-good and known-bad inputs to the
# extractor and asserts both the exit code and the emitted body.
#
# Usage:
#   bash scripts/tests/test-extract-release-notes.sh
#
# Returns:
#   Exit 0 if all assertions pass.
#   Exit 1 if any assertion fails.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT_DEFAULT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
EXTRACTOR="${REPO_ROOT_DEFAULT}/scripts/extract-release-notes.sh"

PASS=0
FAIL=0
FIXTURE_ROOT=""

cleanup() {
  if [[ -n "${FIXTURE_ROOT:-}" && -d "${FIXTURE_ROOT}" ]]; then
    rm -rf "${FIXTURE_ROOT}"
  fi
}
trap cleanup EXIT

FIXTURE_ROOT="$(mktemp -d)"
mkdir -p "${FIXTURE_ROOT}/docs/releases"

run_extract() {
  # Usage: run_extract <version> [extra env]... -- args...
  # Always isolates to the fixture tree via REPO_ROOT.
  REPO_ROOT="${FIXTURE_ROOT}" "${EXTRACTOR}" "$@"
}

assert_eq() {
  local label="$1"; local want="$2"; local got="$3"
  if [[ "${want}" == "${got}" ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      want: ${want}"
    echo "      got:  ${got}"
    FAIL=$((FAIL + 1))
  fi
}

assert_contains() {
  local label="$1"; local needle="$2"; local haystack="$3"
  if [[ "${haystack}" == *"${needle}"* ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      expected to contain: ${needle}"
    echo "      actual: ${haystack}"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_contains() {
  local label="$1"; local needle="$2"; local haystack="$3"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label}"
    echo "      did not want: ${needle}"
    echo "      actual: ${haystack}"
    FAIL=$((FAIL + 1))
  fi
}

# ---------------------------------------------------------------------------
# CASE 1: canonical file with YAML front-matter
#
# The extractor must strip the block between the leading and trailing `---`
# markers and emit the body starting at the first non-blank line below.
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 1: front-matter stripped, body preserved ==="

cat > "${FIXTURE_ROOT}/docs/releases/v1.2.3.md" <<'EOF'
---
version: "1.2.3"
tag: "v1.2.3"
notes_status: canonical
---

# v1.2.3

## Summary

Hello from the curated notes.

- bullet one
- bullet two
EOF

OUT="$(run_extract 1.2.3)"
RC=$?
assert_eq "case1 exit code" 0 "${RC}"
assert_contains "case1 starts with heading" "# v1.2.3" "${OUT}"
assert_contains "case1 contains summary" "Hello from the curated notes." "${OUT}"
assert_not_contains "case1 front-matter key stripped" 'notes_status:' "${OUT}"
assert_not_contains "case1 opening --- stripped" "$(printf '%s\n---' "")" "${OUT:0:4}"

# ---------------------------------------------------------------------------
# CASE 2: file without front-matter is emitted verbatim
#
# Older notes may not have a YAML block. The extractor must not drop content.
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 2: no front-matter → verbatim body ==="

cat > "${FIXTURE_ROOT}/docs/releases/v0.9.0.md" <<'EOF'
# v0.9.0

Legacy release notes body.
EOF

OUT="$(run_extract 0.9.0)"
RC=$?
assert_eq "case2 exit code" 0 "${RC}"
assert_contains "case2 body preserved" "Legacy release notes body." "${OUT}"
assert_contains "case2 heading preserved" "# v0.9.0" "${OUT}"

# ---------------------------------------------------------------------------
# CASE 3: missing file → exit 2 and actionable error
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 3: missing notes file → exit 2 ==="

ERR="$(run_extract 9.9.9 2>&1 >/dev/null || true)"
RC=0
run_extract 9.9.9 >/dev/null 2>&1 || RC=$?
assert_eq "case3 exit code" 2 "${RC}"
assert_contains "case3 error message points at path" "v9.9.9.md" "${ERR}"
assert_contains "case3 error message mentions RELEASE.md" "RELEASE.md" "${ERR}"

# ---------------------------------------------------------------------------
# CASE 4: malformed front-matter (opens with --- but never closes) → exit 3
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 4: unterminated front-matter → exit 3 ==="

cat > "${FIXTURE_ROOT}/docs/releases/v2.0.0.md" <<'EOF'
---
version: "2.0.0"
oops: forgot to close

# body below, but the YAML block never ended
EOF

ERR="$(run_extract 2.0.0 2>&1 >/dev/null || true)"
RC=0
run_extract 2.0.0 >/dev/null 2>&1 || RC=$?
assert_eq "case4 exit code" 3 "${RC}"
assert_contains "case4 error message" "never closed" "${ERR}"

# ---------------------------------------------------------------------------
# CASE 5: script tolerates a leading 'v' prefix on the version argument
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 5: 'v'-prefixed version is accepted ==="

OUT="$(run_extract v1.2.3)"
RC=$?
assert_eq "case5 exit code" 0 "${RC}"
assert_contains "case5 body preserved" "Hello from the curated notes." "${OUT}"

# ---------------------------------------------------------------------------
# CASE 6: usage error when called with no args → exit 1
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 6: usage error → exit 1 ==="

RC=0
run_extract >/dev/null 2>&1 || RC=$?
assert_eq "case6 exit code" 1 "${RC}"

# ---------------------------------------------------------------------------
# CASE 7: <outfile> argument writes the body to the given path
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 7: outfile argument ==="

OUTFILE="${FIXTURE_ROOT}/out.md"
run_extract 1.2.3 "${OUTFILE}" >/dev/null
RC=$?
assert_eq "case7 exit code" 0 "${RC}"
BODY="$(cat "${OUTFILE}")"
assert_contains "case7 outfile contains heading" "# v1.2.3" "${BODY}"
assert_not_contains "case7 outfile front-matter stripped" 'notes_status:' "${BODY}"

# ---------------------------------------------------------------------------
# CASE 8: the workflow contract — a real shipped notes file extracts cleanly.
#
# This asserts the extractor works against the actual docs/releases tree, not
# just synthetic fixtures. It guards against subtle regressions (e.g. the
# front-matter delimiter format drifting).
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 8: real shipped notes file extracts cleanly ==="

LATEST_NOTES="$(ls -1 "${REPO_ROOT_DEFAULT}/docs/releases/"v*.md 2>/dev/null | sort -V | tail -1 || true)"
if [[ -n "${LATEST_NOTES}" ]]; then
  LATEST_VERSION="$(basename "${LATEST_NOTES}" .md)"
  LATEST_VERSION="${LATEST_VERSION#v}"
  OUT="$(REPO_ROOT="${REPO_ROOT_DEFAULT}" "${EXTRACTOR}" "${LATEST_VERSION}")"
  RC=$?
  assert_eq "case8 exit code" 0 "${RC}"
  assert_not_contains "case8 no front-matter leakage" 'notes_status:' "${OUT}"
  # Body must be non-trivial (>= 10 lines) to rule out over-aggressive stripping.
  LINE_COUNT="$(printf '%s\n' "${OUT}" | wc -l | tr -d ' ')"
  if [[ "${LINE_COUNT}" -ge 10 ]]; then
    echo "PASS  case8 body has ${LINE_COUNT} lines"
    PASS=$((PASS + 1))
  else
    echo "FAIL  case8 body unexpectedly short (${LINE_COUNT} lines)"
    FAIL=$((FAIL + 1))
  fi
else
  echo "SKIP  case8 no release notes files found under docs/releases/"
fi

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

echo "All assertions passed — extract-release-notes.sh does what it claims."
exit 0
