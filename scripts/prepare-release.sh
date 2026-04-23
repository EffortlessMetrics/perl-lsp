#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/prepare-release.sh <0.x.y> [release-turnkey options]
  scripts/prepare-release.sh --version <0.x.y> [release-turnkey options]

Examples:
  scripts/prepare-release.sh 0.11.0
  scripts/prepare-release.sh 0.11.0 --dry-run
  scripts/prepare-release.sh 0.11.0 --skip-docker --no-auto-merge
USAGE
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

validate_version() {
  local version="$1"
  if ! [[ "$version" =~ ^0\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
    die "invalid 0.x.y release version: $version"
  fi
}

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

case "${1:-}" in
  --help|-h)
    usage
    exit 0
    ;;
  --version)
    [[ $# -ge 2 ]] || die "missing value for --version"
    VERSION="$2"
    shift 2
    ;;
  -* )
    die "first argument must be <version> or --version <version>"
    ;;
  *)
    VERSION="$1"
    shift
    ;;
esac

validate_version "$VERSION"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

exec cargo xtask release-turnkey --version "$VERSION" "$@"
