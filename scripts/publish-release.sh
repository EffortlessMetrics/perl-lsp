#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/publish-release.sh <0.x.y> [--dry-run] [--ref <git-ref>]

Examples:
  scripts/publish-release.sh 0.11.0
  scripts/publish-release.sh 0.11.0 --dry-run
  scripts/publish-release.sh 0.11.0 --ref master
USAGE
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
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
  -* )
    die "first argument must be the release version"
    ;;
  *)
    VERSION="$1"
    shift
    ;;
esac

DRY_RUN=false
REF=""

while (($#)); do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --ref)
      [[ $# -ge 2 ]] || die "missing value for --ref"
      REF="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

validate_version "$VERSION"
need gh

if [[ -z "$REF" ]]; then
  REF="v${VERSION}"
fi

gh workflow run "Publish to crates.io" \
  --ref "$REF" \
  -f version="$VERSION" \
  -f dry_run="$DRY_RUN"

cat <<EOF
Dispatched "Publish to crates.io" for ${VERSION} on ref ${REF}.

Next steps:
1. gh run list --workflow "Publish to crates.io" --limit 5
2. cargo search perl-lsp --limit 1
3. cargo search perl-dap --limit 1
4. scripts/smoke-test-release.sh ${VERSION}
