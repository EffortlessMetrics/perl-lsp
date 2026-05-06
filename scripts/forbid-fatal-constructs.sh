#!/usr/bin/env bash
# Compatibility wrapper for forbidden-fatal-construct checks.
# Canonical implementation: cargo xtask forbid-fatal-constructs.

set -euo pipefail

exec cargo xtask forbid-fatal-constructs -- "$@"
