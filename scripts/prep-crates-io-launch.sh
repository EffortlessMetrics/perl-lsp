#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

MODE="core"

usage() {
  cat <<'USAGE'
Usage: scripts/prep-crates-io-launch.sh [--core|--all]

Runs crates.io launch readiness checks:
  1) cargo check --locked for selected crates
  2) cargo package dry-run validation for selected crates

Options:
  --core      Validate public launch crates (default)
  --all       Validate every crate in [workspace.metadata.publish.allow]
  -h, --help  Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --core)
      MODE="core"
      ;;
    --all)
      MODE="all"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ "$MODE" == "core" ]]; then
  mapfile -t CRATES < <(printf '%s\n' \
    perl-parser \
    perl-lexer \
    perl-lsp \
    perl-dap \
    perl-corpus)
else
  mapfile -t CRATES < <(
    cd "$ROOT_DIR"
    python3 - <<'PY'
from pathlib import Path
import tomllib

data = tomllib.loads(Path("Cargo.toml").read_text())
for crate in data["workspace"]["metadata"]["publish"]["allow"]:
    print(crate)
PY
  )
fi

echo "🚀 crates.io launch prep (${MODE})"
echo "📦 Running cargo check + cargo package dry-run for ${#CRATES[@]} crate(s)"

for crate in "${CRATES[@]}"; do
  echo ""
  echo "==> ${crate}"
  (
    cd "$ROOT_DIR"
    cargo check --locked -p "$crate"
    CARGO_PACKAGE_NO_VERIFY=1 scripts/cargo-package-workspace-dry-run.sh "$crate" >/dev/null
  )
done

echo ""
echo "✅ crates.io launch prep completed (${MODE})"
