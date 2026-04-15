#!/usr/bin/env bash
# Self-test for scripts/release-notes.sh.
#
# Behavior verified (BDD style — one scenario per test):
#
#   1. Given a curated notes file with YAML front-matter,
#      When the extractor runs,
#      Then the front-matter is stripped and the body starts on the first
#      non-blank line (typically `# v<version>`).
#
#   2. Given a notes file with no front-matter,
#      When the extractor runs,
#      Then every line including the first is emitted verbatim.
#
#   3. Given a missing notes file,
#      When the extractor runs,
#      Then it exits 1 with a diagnostic mentioning the missing path — the
#      release workflow must refuse to publish without curated notes.
#
#   4. Given an unterminated YAML front-matter,
#      When the extractor runs,
#      Then it exits non-zero rather than silently emitting an empty body.
#
#   5. Given only front-matter and no body,
#      When the extractor runs,
#      Then it exits non-zero (we never publish an empty release body).
#
#   6. Given an invalid version argument,
#      When the extractor runs,
#      Then it exits 2 (usage error, distinct from "file missing").
#
#   7. Given a --file argument pointing at a non-standard path,
#      When the extractor runs,
#      Then that file is read directly without requiring docs/releases layout.
#
#   8. Given a version prefixed with `v`,
#      When the extractor runs,
#      Then the leading `v` is tolerated (caller convenience).
#
#   9. Given the real repository checkout,
#      When the extractor runs against an existing release (v0.12.4),
#      Then the output is non-empty and contains the heading `# v0.12.4`.
#
# Usage:  bash scripts/tests/test-release-notes.sh
# Exit:   0 on full pass, 1 on any failure.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
IMPL="${REPO_ROOT}/scripts/release-notes.sh"

PASS=0
FAIL=0

pass() { printf 'PASS  %s\n' "$1"; PASS=$((PASS + 1)); }
fail() { printf 'FAIL  %s\n' "$1"; FAIL=$((FAIL + 1)); }

if [[ ! -x "${IMPL}" ]]; then
  echo "ERROR: ${IMPL} is not executable (or missing)" >&2
  exit 1
fi

# Shared temp root, cleaned on exit.
TMPROOT="$(mktemp -d)"
cleanup() { rm -rf "${TMPROOT}"; }
trap cleanup EXIT

# Build an isolated fake repo root under $TMPROOT/<name> with a single
# docs/releases/v<version>.md containing $content, and echo the root path.
make_fake_root() {
  local name="$1" version="$2" content="$3"
  local root="${TMPROOT}/${name}"
  mkdir -p "${root}/docs/releases"
  printf '%s' "${content}" > "${root}/docs/releases/v${version}.md"
  printf '%s' "${root}"
}

# ---------------------------------------------------------------------------
# 1. Front-matter stripped, body starts on first non-blank line.
# ---------------------------------------------------------------------------
test_frontmatter_stripped() {
  local root
  root="$(make_fake_root fm_strip 1.2.3 '---
version: "1.2.3"
tag: "v1.2.3"
---

# v1.2.3

Body line.
')"
  local out
  out="$(bash "${IMPL}" --root "${root}" 1.2.3)"
  if [[ "${out}" == *'version: "1.2.3"'* ]]; then
    fail "front-matter stripped: output still contains front-matter"
    return
  fi
  local first_line
  first_line="$(printf '%s\n' "${out}" | head -1)"
  if [[ "${first_line}" != "# v1.2.3" ]]; then
    fail "front-matter stripped: first line is '${first_line}', expected '# v1.2.3'"
    return
  fi
  if [[ "${out}" != *"Body line."* ]]; then
    fail "front-matter stripped: body content missing from output"
    return
  fi
  pass "front-matter is stripped and body starts on first non-blank line"
}

# ---------------------------------------------------------------------------
# 2. No front-matter — every line emitted verbatim.
# ---------------------------------------------------------------------------
test_no_frontmatter() {
  local root
  root="$(make_fake_root no_fm 2.0.0 '# v2.0.0

Notes without front-matter.
')"
  local out
  out="$(bash "${IMPL}" --root "${root}" 2.0.0)"
  local first_line
  first_line="$(printf '%s\n' "${out}" | head -1)"
  if [[ "${first_line}" != "# v2.0.0" ]]; then
    fail "no front-matter: first line is '${first_line}', expected '# v2.0.0'"
    return
  fi
  if [[ "${out}" != *"Notes without front-matter."* ]]; then
    fail "no front-matter: body missing from output"
    return
  fi
  pass "files without front-matter emitted verbatim"
}

# ---------------------------------------------------------------------------
# 3. Missing file — exit 1 with diagnostic.
# ---------------------------------------------------------------------------
test_missing_file() {
  local root="${TMPROOT}/missing_root"
  mkdir -p "${root}/docs/releases"
  local stderr_log="${TMPROOT}/missing_root.stderr"
  set +e
  bash "${IMPL}" --root "${root}" 9.9.9 >/dev/null 2>"${stderr_log}"
  local rc=$?
  set -e
  if [[ "${rc}" -ne 1 ]]; then
    fail "missing file: expected exit 1, got ${rc}"
    return
  fi
  if ! grep -q 'release notes file not found' "${stderr_log}"; then
    fail "missing file: diagnostic missing 'release notes file not found'"
    return
  fi
  if ! grep -q 'v9.9.9.md' "${stderr_log}"; then
    fail "missing file: diagnostic did not mention the expected path"
    return
  fi
  pass "missing notes file exits 1 with a clear diagnostic"
}

# ---------------------------------------------------------------------------
# 4. Unterminated front-matter — fails non-zero.
# ---------------------------------------------------------------------------
test_unterminated_frontmatter() {
  local root
  root="$(make_fake_root unterm 3.0.0 '---
version: "3.0.0"
tag: "v3.0.0"
# body missing closing fence
')"
  set +e
  bash "${IMPL}" --root "${root}" 3.0.0 >/dev/null 2>"${TMPROOT}/unterm.stderr"
  local rc=$?
  set -e
  if [[ "${rc}" -eq 0 ]]; then
    fail "unterminated front-matter: expected non-zero exit, got 0"
    return
  fi
  pass "unterminated YAML front-matter exits non-zero"
}

# ---------------------------------------------------------------------------
# 5. Front-matter only, no body — fails non-zero.
# ---------------------------------------------------------------------------
test_frontmatter_only() {
  local root
  root="$(make_fake_root fm_only 4.0.0 '---
version: "4.0.0"
---
')"
  set +e
  bash "${IMPL}" --root "${root}" 4.0.0 >/dev/null 2>"${TMPROOT}/fm_only.stderr"
  local rc=$?
  set -e
  if [[ "${rc}" -eq 0 ]]; then
    fail "front-matter only: expected non-zero exit, got 0"
    return
  fi
  pass "front-matter-only file exits non-zero (never publish empty body)"
}

# ---------------------------------------------------------------------------
# 6. Invalid version — exit 2 (usage error).
# ---------------------------------------------------------------------------
test_invalid_version() {
  set +e
  bash "${IMPL}" --root "${TMPROOT}" "not-a-version" >/dev/null 2>"${TMPROOT}/invalid.stderr"
  local rc=$?
  set -e
  if [[ "${rc}" -ne 2 ]]; then
    fail "invalid version: expected exit 2, got ${rc}"
    return
  fi
  pass "invalid version argument exits 2 (usage error)"
}

# ---------------------------------------------------------------------------
# 7. --file bypasses version lookup.
# ---------------------------------------------------------------------------
test_file_flag() {
  local custom="${TMPROOT}/custom-notes.md"
  cat > "${custom}" <<'EOF'
---
version: "5.0.0"
---

# v5.0.0

From explicit --file path.
EOF
  local out
  out="$(bash "${IMPL}" --file "${custom}")"
  if [[ "${out}" != *"From explicit --file path."* ]]; then
    fail "--file flag: body content missing"
    return
  fi
  if [[ "${out}" == *'version: "5.0.0"'* ]]; then
    fail "--file flag: front-matter not stripped"
    return
  fi
  pass "--file flag reads arbitrary paths and strips front-matter"
}

# ---------------------------------------------------------------------------
# 8. Leading `v` on the version argument is tolerated.
# ---------------------------------------------------------------------------
test_version_v_prefix() {
  local root
  root="$(make_fake_root v_prefix 6.1.2 '---
version: "6.1.2"
---

# v6.1.2

Body.
')"
  local out
  out="$(bash "${IMPL}" --root "${root}" v6.1.2)"
  if [[ "${out}" != *"# v6.1.2"* ]]; then
    fail "v prefix: output missing expected heading"
    return
  fi
  pass "leading 'v' on version argument is tolerated"
}

# ---------------------------------------------------------------------------
# 9. Real repo — existing release file extracts cleanly.
# ---------------------------------------------------------------------------
test_real_repo() {
  if [[ ! -f "${REPO_ROOT}/docs/releases/v0.12.4.md" ]]; then
    pass "real repo check skipped (v0.12.4.md missing in this checkout)"
    return
  fi
  local out
  out="$(bash "${IMPL}" --root "${REPO_ROOT}" 0.12.4)"
  if [[ -z "${out}" ]]; then
    fail "real repo: output was empty"
    return
  fi
  local first_line
  first_line="$(printf '%s\n' "${out}" | head -1)"
  if [[ "${first_line}" != "# v0.12.4" ]]; then
    fail "real repo: first line is '${first_line}', expected '# v0.12.4'"
    return
  fi
  if [[ "${out}" == *'release_date_utc:'* ]]; then
    fail "real repo: front-matter leaked into output"
    return
  fi
  pass "real repo v0.12.4.md extracts to non-empty body starting with '# v0.12.4'"
}

# ---------------------------------------------------------------------------
# Run all scenarios.
# ---------------------------------------------------------------------------
test_frontmatter_stripped
test_no_frontmatter
test_missing_file
test_unterminated_frontmatter
test_frontmatter_only
test_invalid_version
test_file_flag
test_version_v_prefix
test_real_repo

echo
echo "Summary: ${PASS} passed, ${FAIL} failed"
if [[ "${FAIL}" -gt 0 ]]; then
  exit 1
fi
exit 0
