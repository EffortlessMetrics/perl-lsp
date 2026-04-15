#!/usr/bin/env bash
# Extract curated release notes for a given version from docs/releases/vX.Y.Z.md.
#
# The notes file contains a YAML front-matter block followed by the prose body
# that should ship as the GitHub Release description. This script strips the
# front-matter and emits the body to stdout so the release workflow can feed it
# to softprops/action-gh-release as `body_path`.
#
# Usage:
#   scripts/extract-release-notes.sh <version>            # writes body to stdout
#   scripts/extract-release-notes.sh <version> <outfile>  # writes body to <outfile>
#
# Arguments:
#   <version>  Semantic version without leading 'v' (e.g. 0.12.4, 1.0.0-beta.1)
#   <outfile>  Optional output path. When omitted the body is printed to stdout.
#
# Environment:
#   RELEASE_NOTES_DIR  Override the notes directory (default: docs/releases).
#   REPO_ROOT          Override the repo root (default: git rev-parse result).
#
# Exit codes:
#   0  success
#   1  unknown flag or usage error
#   2  notes file does not exist
#   3  front-matter block is malformed (opened with `---` but never closed)
#
# The script is deliberately dependency-free (pure bash + sed/awk) so it works
# in the minimal Ubuntu image used by release.yml without installing anything.

set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: extract-release-notes.sh <version> [outfile]

Reads docs/releases/v<version>.md, strips the YAML front-matter block, and
writes the remaining body to stdout (or <outfile> when provided).

Exit codes:
  0 success
  1 usage error
  2 notes file missing
  3 malformed front-matter (unterminated --- block)
USAGE
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 1
fi

case "$1" in
  -h|--help)
    usage
    exit 0
    ;;
esac

VERSION="$1"
OUTFILE="${2:-}"

if [[ -z "${VERSION}" ]]; then
  echo "error: <version> must be non-empty" >&2
  usage
  exit 1
fi

# Strip a leading 'v' if the caller passed 'v0.12.4' by accident, so the script
# is forgiving in both directions.
VERSION="${VERSION#v}"

# Resolve repo root so the script works when invoked from any cwd.
if [[ -n "${REPO_ROOT:-}" ]]; then
  ROOT="${REPO_ROOT}"
elif ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  # Fallback: assume the script lives in <root>/scripts/
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi

NOTES_DIR="${RELEASE_NOTES_DIR:-${ROOT}/docs/releases}"
NOTES_FILE="${NOTES_DIR}/v${VERSION}.md"

if [[ ! -f "${NOTES_FILE}" ]]; then
  cat >&2 <<EOF
error: release notes file not found: ${NOTES_FILE}

Every tagged release must ship with a curated notes file at
docs/releases/v<version>.md. See RELEASE.md ("Release History Updates") for the
template. Create the file, commit it, and re-run the release workflow.
EOF
  exit 2
fi

# Strip YAML front-matter with a small awk program. The rule:
#   - if line 1 is exactly '---', skip lines until the next '---', then emit
#     everything after it;
#   - otherwise emit the file verbatim (no front-matter present).
#
# We also leading-trim blank lines after the closing '---' so the body starts
# at the first real content line (usually `# vX.Y.Z`).
BODY="$(
  awk '
    BEGIN { state = "start" }
    state == "start" {
      if (NR == 1 && $0 == "---") { state = "in_fm"; next }
      else { state = "body" }
    }
    state == "in_fm" {
      if ($0 == "---") { state = "post_fm"; next }
      next
    }
    state == "post_fm" {
      if ($0 ~ /^[[:space:]]*$/) next
      state = "body"
    }
    state == "body" { print }
    END {
      if (state == "in_fm") exit 3
    }
  ' "${NOTES_FILE}"
)" || {
  status=$?
  if [[ "${status}" -eq 3 ]]; then
    echo "error: front-matter in ${NOTES_FILE} opens with '---' but is never closed" >&2
    exit 3
  fi
  exit "${status}"
}

if [[ -n "${OUTFILE}" ]]; then
  printf '%s\n' "${BODY}" > "${OUTFILE}"
else
  printf '%s\n' "${BODY}"
fi
