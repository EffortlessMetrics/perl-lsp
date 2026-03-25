#!/usr/bin/env bash
set -euo pipefail

# Canonical implementation: cargo xtask update-homebrew.
# Usage: ./scripts/update-homebrew.sh <v0.8.3>

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <version>" >&2
  echo "Example: $0 v0.8.3" >&2
  exit 1
fi

VERSION="$1"
shift

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

exec cargo xtask update-homebrew --version "$VERSION" "$@"
