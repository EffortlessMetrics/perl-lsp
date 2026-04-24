#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/smoke-test-release.sh <0.x.y>

Example:
  scripts/smoke-test-release.sh 0.11.0
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

if [[ "${XTASK_SMOKE_TEST_RELEASE:-0}" == "1" ]]; then
  if [[ $# -ne 1 ]]; then
    usage
    exit 1
  fi

  if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    usage
    exit 0
  fi

  RELEASE_VERSION="$1"
  validate_version "$RELEASE_VERSION"

  TEMP_DIR="$(mktemp -d)"
  INSTALL_ROOT="$TEMP_DIR/install"
  trap 'rm -rf "$TEMP_DIR"' EXIT

  export PATH="$INSTALL_ROOT/bin:$PATH"

  printf 'Smoke testing release %s\n' "$RELEASE_VERSION"
  printf 'Temporary install root: %s\n\n' "$INSTALL_ROOT"

  cargo install perllsp --version "$RELEASE_VERSION" --locked --root "$INSTALL_ROOT"
  cargo install perl-dap --version "$RELEASE_VERSION" --locked --root "$INSTALL_ROOT"

  LSP_VERSION="$(perllsp --version | head -n 1)"
  DAP_VERSION="$(perl-dap --version | head -n 1)"

  [[ "$LSP_VERSION" == *"$RELEASE_VERSION"* ]] || die "perllsp version mismatch: $LSP_VERSION"
  [[ "$DAP_VERSION" == *"$RELEASE_VERSION"* ]] || die "perl-dap version mismatch: $DAP_VERSION"

  perllsp --help >/dev/null
  perl-dap --help >/dev/null

  printf 'Installed versions:\n'
  printf '  perllsp: %s\n' "$LSP_VERSION"
  printf '  perl-dap: %s\n\n' "$DAP_VERSION"

  printf 'Sparse index checks:\n'
  cargo search perllsp --limit 1
  cargo search perl-dap --limit 1
  cargo search perl-parser --limit 1

  cat <<EOF

Manual follow-up:
1. Verify GitHub release assets for v${RELEASE_VERSION}.
2. Verify the VS Code Marketplace and Open VSX list version ${RELEASE_VERSION}.
3. Open VS Code against a Perl workspace and confirm the extension downloads or locates perllsp successfully.
EOF
  exit 0
fi

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

exec cargo xtask smoke-test-release "$@"
