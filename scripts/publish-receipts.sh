#!/usr/bin/env bash
# Compatibility wrapper for review receipt publication.
# Canonical implementation: cargo xtask publish-receipts.

set -euo pipefail

if [ $# -gt 0 ]; then
  exec cargo xtask publish-receipts "$1"
else
  exec cargo xtask publish-receipts
fi
