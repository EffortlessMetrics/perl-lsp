#!/usr/bin/env bash
# Extract the curated release notes body for a given version from
# docs/releases/vX.Y.Z.md, stripping the YAML front-matter so the output is
# ready to use as a GitHub Release body.
#
# Why this exists:
#   The release workflow historically generated a generic "Docs home / Install"
#   body for every GitHub Release. The repo now publishes curated per-release
#   notes at docs/releases/vX.Y.Z.md with structured summaries, highlights, and
#   upgrade guidance. This script is the single source of truth for how that
#   file is turned into release-body markdown — both the release workflow and
#   the self-test consume it.
#
# Usage:
#   scripts/extract-release-notes.sh <version>           # writes body to stdout
#   scripts/extract-release-notes.sh <version> <outfile> # writes body to outfile
#
# <version> may be bare ("0.12.4") or tag-prefixed ("v0.12.4").
#
# Exit codes:
#   0  success
#   1  missing arguments / malformed version
#   2  docs/releases/vX.Y.Z.md does not exist
#   3  notes file is empty or contained only front-matter
#
# The repo root is resolved relative to the script so callers can invoke it
# from any working directory (workflow steps, local shells, etc).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"

usage() {
  cat >&2 <<'USAGE'
Usage: extract-release-notes.sh <version> [outfile]

  <version>  Release version, e.g. 0.12.4 or v0.12.4
  [outfile]  Optional path to write the extracted body to (default: stdout)
USAGE
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 1
fi

RAW_VERSION="$1"
OUTFILE="${2:-}"

# Accept either "0.12.4" or "v0.12.4" and normalise to a bare version.
VERSION="${RAW_VERSION#v}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  echo "::error::Invalid version format: ${RAW_VERSION}" >&2
  exit 1
fi

NOTES_FILE="${REPO_ROOT}/docs/releases/v${VERSION}.md"

if [[ ! -f "$NOTES_FILE" ]]; then
  echo "::error::Release notes file not found: docs/releases/v${VERSION}.md" >&2
  echo "::error::Every release must ship a curated notes file. See RELEASE.md #release-history-updates." >&2
  exit 2
fi

# Strip an optional YAML front-matter block delimited by lines that contain
# only `---`. The front-matter is the block opening on line 1 (if present);
# anything after the closing `---` is the body we want to publish.
#
# awk state machine:
#   state=0  haven't seen any `---` yet and first non-blank line is `---`
#            -> switch to state=1 (inside front-matter)
#   state=0  first non-blank line is NOT `---`
#            -> switch to state=2 (no front-matter, print everything from here)
#   state=1  inside front-matter, waiting for closing `---`
#            -> on match, switch to state=2; otherwise skip
#   state=2  print every line
BODY="$(awk '
  BEGIN { state = 0 }
  state == 0 {
    if ($0 ~ /^---[[:space:]]*$/) { state = 1; next }
    if ($0 ~ /^[[:space:]]*$/)    { next }        # skip leading blanks
    state = 2
  }
  state == 1 {
    if ($0 ~ /^---[[:space:]]*$/) { state = 2; next }
    next
  }
  state == 2 { print }
' "$NOTES_FILE")"

# Trim leading blank lines so the body starts at its first real line.
BODY="$(printf '%s\n' "$BODY" | awk 'NF { found = 1 } found')"

if [[ -z "$BODY" ]]; then
  echo "::error::Release notes file has no body content: docs/releases/v${VERSION}.md" >&2
  exit 3
fi

if [[ -n "$OUTFILE" ]]; then
  printf '%s\n' "$BODY" > "$OUTFILE"
else
  printf '%s\n' "$BODY"
fi
