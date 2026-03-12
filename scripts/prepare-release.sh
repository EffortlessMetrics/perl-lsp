#!/usr/bin/env bash
set -euo pipefail

# Legacy compatibility wrapper for historical local release-prep entrypoint.
#
# Authoritative release flow is PR-driven via scripts/release-turnkey-pr.sh.
# This wrapper preserves the old command name while forwarding to the
# supported orchestration script.

usage() {
  cat <<'USAGE'
Usage:
  scripts/prepare-release.sh <0.x.y> [--dry-run]
  scripts/prepare-release.sh --version <0.x.y> [--dry-run]

Notes:
  - This is a compatibility wrapper.
  - It forwards to scripts/release-turnkey-pr.sh.
  - For full options, run scripts/release-turnkey-pr.sh --help.
USAGE
}

if (($# == 0)); then
  usage
  exit 1
fi

VERSION=""
DRY_RUN=0
PASSTHROUGH=()

while (($#)); do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    --version)
      VERSION="${2:-}"
      if [[ -z "$VERSION" ]]; then
        echo "error: --version requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --*)
      PASSTHROUGH+=("$1")
      if [[ $# -gt 1 && "$2" != --* ]]; then
        PASSTHROUGH+=("$2")
        shift 2
      else
        shift
      fi
      ;;
    *)
      if [[ -z "$VERSION" ]]; then
        VERSION="$1"
      else
        PASSTHROUGH+=("$1")
      fi
      shift
      ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  echo "error: missing release version" >&2
  usage
  exit 1
fi

if ! [[ "$VERSION" =~ ^0\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  echo "error: invalid 0.x.y release version: $VERSION" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TURNKEY_SCRIPT="$SCRIPT_DIR/release-turnkey-pr.sh"

if [[ ! -x "$TURNKEY_SCRIPT" ]]; then
  echo "error: expected executable not found: $TURNKEY_SCRIPT" >&2
  exit 1
fi

printf '[prepare-release] forwarding to release-turnkey-pr.sh for v%s\n' "$VERSION"

CMD=("$TURNKEY_SCRIPT" --version "$VERSION")
if (( DRY_RUN )); then
  CMD+=(--dry-run)
fi
if ((${#PASSTHROUGH[@]} > 0)); then
  CMD+=("${PASSTHROUGH[@]}")
fi

exec "${CMD[@]}"
