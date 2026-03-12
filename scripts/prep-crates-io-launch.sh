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

filter_cargo_package_noise() {
  awk '
    skip_help_lines > 0 {
      skip_help_lines--
      next
    }
    /^warning: ignoring (test|example|benchmark) / {
      next
    }
    /^warning: patch .* was not used in the crate graph$/ {
      next
    }
    /^help: Check that the patched package version and available features are compatible$/ {
      skip_help_lines = 3
      next
    }
    {
      print
    }
  '
}

for crate in "${CRATES[@]}"; do
  echo ""
  echo "==> ${crate}"
  (
    cd "$ROOT_DIR"
    cargo check --locked -p "$crate"
    if ! package_output="$(CARGO_PACKAGE_NO_VERIFY=1 scripts/cargo-package-workspace-dry-run.sh "$crate" 2>&1)"; then
      printf '%s\n' "$package_output"
      exit 1
    fi
    printf '%s\n' "$package_output" | filter_cargo_package_noise
  )
done

echo ""
echo "✅ crates.io launch prep completed (${MODE})"
