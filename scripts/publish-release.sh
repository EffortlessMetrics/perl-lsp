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

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

exec cargo xtask publish-release "$@"
