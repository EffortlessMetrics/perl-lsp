#!/usr/bin/env bash
# check-version-sync.sh - Verify all version strings in the project agree.
# Exit 0 if all match, exit 1 if any disagree.

set -euo pipefail

# Resolve repo root relative to this script
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Extract versions from each source ---

# 1. features.toml: version = "X.Y.Z" under [meta]
V_FEATURES=$(grep -m1 '^version\s*=' "$REPO_ROOT/features.toml" | sed 's/.*"\(.*\)".*/\1/')

# 2. crates/perl-lsp/Cargo.toml: version = "X.Y.Z"
V_CARGO=$(grep -m1 '^version\s*=' "$REPO_ROOT/crates/perl-lsp/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')

# 3. vscode-extension/package.json: "version": "X.Y.Z"
V_VSCODE=$(grep -m1 '"version"' "$REPO_ROOT/vscode-extension/package.json" | sed 's/.*"\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)".*/\1/')

# 4. crates/perl-lsp/build.rs: fallback VERSION constant in the minimal catalog
#    Match the literal string line (not the format! template line)
V_BUILDRS=$(grep 'pub const VERSION: &str = \\"' "$REPO_ROOT/crates/perl-lsp/build.rs" | sed 's/.*\\"\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)\\".*/\1/' | head -1)

# --- Compare ---

ALL_MATCH=true
REFERENCE="$V_FEATURES"

for v in "$V_CARGO" "$V_VSCODE" "$V_BUILDRS"; do
    if [ "$v" != "$REFERENCE" ]; then
        ALL_MATCH=false
        break
    fi
done

if [ "$ALL_MATCH" = true ]; then
    echo "Version sync check: all sources agree on $REFERENCE"
    echo "  features.toml:           $V_FEATURES"
    echo "  perl-lsp/Cargo.toml:     $V_CARGO"
    echo "  vscode-extension:        $V_VSCODE"
    echo "  build.rs fallback:       $V_BUILDRS"
    exit 0
else
    echo "ERROR: Version mismatch detected!"
    echo "  features.toml:           $V_FEATURES"
    echo "  perl-lsp/Cargo.toml:     $V_CARGO"
    echo "  vscode-extension:        $V_VSCODE"
    echo "  build.rs fallback:       $V_BUILDRS"
    exit 1
fi
