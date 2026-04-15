#!/usr/bin/env bash
# Emit the curated release-notes body for a given version.
#
# Reads `docs/releases/v<version>.md`, strips its YAML front-matter, and prints
# the remaining markdown to stdout. Intended to be consumed by the Release
# workflow (.github/workflows/release.yml) so the GitHub Release body mirrors
# the committed per-release notes file rather than an auto-generated PR dump.
#
# Behavior:
#   - Fails with exit 1 (and a clear error message on stderr) when the notes
#     file is missing. The release workflow must refuse to publish without
#     curated notes — see RELEASE.md "Before publishing" checklist.
#   - Fails with exit 2 on usage errors (missing/invalid arguments).
#   - Strips the leading YAML front-matter block (a `---` line, arbitrary
#     content, and a closing `---` line). Files without front-matter are
#     emitted verbatim.
#   - Trims leading blank lines from the body so the emitted notes start on the
#     first meaningful line (typically the `# v<version>` heading).
#
# Usage:
#   scripts/release-notes.sh <version>           # e.g. 0.12.4
#   scripts/release-notes.sh --file <path>       # explicit file (for tests)
#
# Flags:
#   --root <dir>   Resolve `docs/releases/` under <dir> instead of $PWD.
#   --file <path>  Bypass version lookup and use <path> directly.
#   -h, --help     Print this help and exit 0.

set -euo pipefail

usage() {
  sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

die_usage() {
  printf 'error: %s\n' "$*" >&2
  printf 'run `%s --help` for usage.\n' "$0" >&2
  exit 2
}

VERSION=""
ROOT="."
FILE=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --root)
      [ "$#" -ge 2 ] || die_usage "--root requires a directory argument"
      ROOT="$2"
      shift 2
      ;;
    --file)
      [ "$#" -ge 2 ] || die_usage "--file requires a path argument"
      FILE="$2"
      shift 2
      ;;
    --)
      shift
      break
      ;;
    -*)
      die_usage "unknown flag: $1"
      ;;
    *)
      if [ -n "$VERSION" ]; then
        die_usage "unexpected positional argument: $1"
      fi
      VERSION="$1"
      shift
      ;;
  esac
done

if [ -z "$FILE" ]; then
  [ -n "$VERSION" ] || die_usage "version argument required (e.g. 0.12.4)"
  # Accept an optional leading `v` for caller convenience.
  VERSION="${VERSION#v}"
  case "$VERSION" in
    [0-9]*.[0-9]*.[0-9]*) : ;;
    *) die_usage "invalid version: $VERSION (expected MAJOR.MINOR.PATCH)" ;;
  esac
  FILE="${ROOT%/}/docs/releases/v${VERSION}.md"
fi

if [ ! -f "$FILE" ]; then
  printf 'error: release notes file not found: %s\n' "$FILE" >&2
  printf 'hint: create the curated notes file before publishing.\n' >&2
  printf '      see RELEASE.md "Before publishing" checklist.\n' >&2
  exit 1
fi

if [ ! -s "$FILE" ]; then
  die "release notes file is empty: $FILE"
fi

# Strip YAML front-matter (a leading `---` line through the next `---` line)
# and print the remaining body, trimming leading blank lines.
awk '
  BEGIN { state = "start"; started = 0 }
  state == "start" {
    if (NR == 1 && $0 == "---") { state = "frontmatter"; next }
    state = "body"
  }
  state == "frontmatter" {
    if ($0 == "---") { state = "body"; next }
    next
  }
  state == "body" {
    if (!started) {
      if ($0 ~ /^[[:space:]]*$/) next
      started = 1
    }
    print
  }
  END {
    if (state == "frontmatter") {
      print "error: release notes file has unterminated YAML front-matter" > "/dev/stderr"
      exit 1
    }
    if (!started) {
      print "error: release notes file has no body content after front-matter" > "/dev/stderr"
      exit 1
    }
  }
' "$FILE"
