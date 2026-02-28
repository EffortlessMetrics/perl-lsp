#!/usr/bin/env bash
# check-version-sync.sh - Verify all version strings in the project agree.
# Exit 0 if all match, exit 1 if any disagree.

set -euo pipefail

# Resolve repo root relative to this script
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Extract versions from source-of-truth files ---
read -r V_CARGO V_FEATURES V_VSCODE <<<"$(python3 - "$REPO_ROOT" <<'PY'
import json
import pathlib
import sys
try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11 compatibility
    import tomli as tomllib

root = pathlib.Path(sys.argv[1])

with open(root / "Cargo.toml", "rb") as f:
    cargo = tomllib.load(f)

with open(root / "features.toml", "rb") as f:
    features = tomllib.load(f)

with open(root / "vscode-extension/package.json", "r", encoding="utf-8") as f:
    vscode = json.load(f)

print(cargo["workspace"]["package"]["version"])
print(features["meta"]["version"])
print(vscode["version"])
PY
)"

# --- Compare ---

ALL_MATCH=true
REFERENCE="$V_CARGO"

if [ -z "$V_CARGO" ] || [ -z "$V_FEATURES" ] || [ -z "$V_VSCODE" ]; then
    ALL_MATCH=false
fi

for v in "$V_FEATURES" "$V_VSCODE"; do
    if [ -z "$v" ] || [ "$v" != "$REFERENCE" ]; then
        ALL_MATCH=false
        break
    fi
done

if [ "$ALL_MATCH" = true ]; then
    echo "Version sync check: all sources agree on $REFERENCE"
    echo "  Cargo.toml [workspace]:  $V_CARGO"
    echo "  features.toml:           $V_FEATURES"
    echo "  vscode-extension:        $V_VSCODE"
    exit 0
else
    echo "ERROR: Version mismatch detected!"
    echo "  Cargo.toml [workspace]:  $V_CARGO"
    echo "  features.toml:           $V_FEATURES"
    echo "  vscode-extension:        $V_VSCODE"
    exit 1
fi
