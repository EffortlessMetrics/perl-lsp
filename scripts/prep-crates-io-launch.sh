#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/prep-crates-io-launch.sh [--core|--all]

Runs crates.io launch readiness checks through xtask:
  1) cargo check --locked for selected crates
  2) cargo package --no-verify dry-run validation for selected crates

Options:
  --core      Validate public launch crates (default)
  --all       Validate every crate in [workspace.metadata.publish.allow]
  -h, --help  Show help
USAGE
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

MODE="${1:---core}"
shift || true
if [[ $# -gt 0 ]]; then
  echo "Too many arguments" >&2
  usage
  exit 2
fi

case "$MODE" in
  --core|core)
    exec cargo xtask prep-crates-io-launch --mode core
    ;;
  --all|all)
    exec cargo xtask prep-crates-io-launch --mode all
    ;;
  *)
    echo "Unknown mode: $MODE" >&2
    usage
    exit 2
    ;;
esac
