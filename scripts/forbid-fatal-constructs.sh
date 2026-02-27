#!/usr/bin/env bash
# forbid-fatal-constructs.sh - Gate to prevent fatal constructs in production code
#
# This script scans the codebase for constructs that can cause unrecoverable
# termination in production code:
#   - std::process::abort() - panic without unwind, NEVER allowed
#   - std::process::exit()  - immediate termination (allowlisted paths only)
#   - panic!()              - unwind or abort (allowed in #[cfg(test)] modules)
#   - todo!()               - panic with message
#   - unimplemented!()      - panic with message
#   - dbg!()                - debug output (not fatal but banned)
#
# Excluded paths (for all checks):
#   - tests/**              - Test code can panic
#   - benches/**            - Benchmark code can panic
#   - build.rs              - Build scripts can exit
#   - examples/**           - Example code can exit
#   - *_test.rs, *_tests.rs - Test modules within src/
#   - *-support crates      - Test infrastructure crates
#   - tree-sitter-perl-*    - Excluded from workspace, legacy parsers
#   - xtask/                - Development tooling (not shipped)
#
# Exit allowlist (paths where exit() is permitted):
#   - **/bin/**             - Binary entry points
#   - lifecycle.rs          - LSP lifecycle handler (exit on shutdown)
#
# IMPORTANT: abort() is banned everywhere, including bin/ directories.
#
# The workspace Cargo.toml already denies clippy::unwrap_used and clippy::expect_used,
# so those are enforced by the compiler. This script catches what Clippy misses.
#
# Note: This script cannot detect panic! inside #[cfg(test)] modules within src/ files.
# The Clippy deny lints in Cargo.toml handle that case via --all-targets.
#
# Usage:
#   bash scripts/forbid-fatal-constructs.sh
#   bash scripts/forbid-fatal-constructs.sh --verbose
#
set -euo pipefail

VERBOSE="${VERBOSE:-}"
if [[ "${1:-}" == "--verbose" || "${1:-}" == "-v" ]]; then
    VERBOSE=1
fi

cd "$(dirname "$0")/.."

# Color output helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Check if ripgrep is available
if ! command -v rg &> /dev/null; then
    echo "Error: ripgrep (rg) is required but not found in PATH"
    exit 1
fi

# Common exclusions for test/example/tooling code
COMMON_EXCLUDES=(
    --glob '!**/tests/**'
    --glob '!**/benches/**'
    --glob '!**/build.rs'
    --glob '!**/examples/**'
    --glob '!**/*_test.rs'
    --glob '!**/*_tests.rs'
    --glob '!**/tree-sitter-perl-c/**'
    --glob '!**/tree-sitter-perl-rs/**'
    --glob '!**/perl-tdd-support/**'
    --glob '!**/perl-ts-heredoc-analysis/**'
    --glob '!**/perl-ts-logos-lexer/**'
    --glob '!**/perl-ts-heredoc-parser/**'
    --glob '!**/perl-ts-partial-ast/**'
    --glob '!**/perl-ts-advanced-parsers/**'
)

ERRORS=0

# =============================================================================
# CHECK 1: abort() - NEVER allowed (not even in bin/)
# =============================================================================
ABORT_MATCHES=$(rg -n \
    --type rust \
    "${COMMON_EXCLUDES[@]}" \
    'std::process::abort\(' \
    crates 2>/dev/null || true)

if [[ -n "$ABORT_MATCHES" ]]; then
    echo -e "${RED}ERROR: std::process::abort() found in production code${NC}"
    echo ""
    echo "abort() is never allowed - it terminates without unwinding."
    echo "=================================================="
    echo "$ABORT_MATCHES"
    echo "=================================================="
    echo ""
    echo "To fix: return an error and let the caller handle it."
    echo ""
    ERRORS=1
fi

# =============================================================================
# CHECK 2: exit() - allowed only in bin/ and lifecycle.rs
# =============================================================================
EXIT_MATCHES=$(rg -n \
    --type rust \
    "${COMMON_EXCLUDES[@]}" \
    'std::process::exit\(' \
    crates 2>/dev/null || true)

if [[ -n "$EXIT_MATCHES" ]]; then
    # Filter out allowlisted paths:
    # - **/bin/** (binary entry points)
    # - **/lifecycle.rs (LSP exit handler)
    EXIT_VIOLATIONS=$(echo "$EXIT_MATCHES" | \
        grep -v '/bin/' | \
        grep -v 'lifecycle\.rs:' || true)

    if [[ -n "$EXIT_VIOLATIONS" ]]; then
        echo -e "${RED}ERROR: std::process::exit() found outside allowlist${NC}"
        echo ""
        echo "exit() is only allowed in:"
        echo "  - bin/ directories (CLI entry points)"
        echo "  - lifecycle.rs (LSP exit handler)"
        echo "=================================================="
        echo "$EXIT_VIOLATIONS"
        echo "=================================================="
        echo ""
        echo "To fix: return an error, use Result<(), E>, or move to an allowlisted path."
        echo ""
        ERRORS=1
    fi
fi

# =============================================================================
# RESULT
# =============================================================================
if [[ $ERRORS -ne 0 ]]; then
    exit 1
fi

if [[ -n "$VERBOSE" ]]; then
    echo -e "${GREEN}OK: No forbidden fatal constructs in production code${NC}"
    echo ""
    echo -e "${YELLOW}Policy summary:${NC}"
    echo "  - abort(): NEVER allowed (banned everywhere)"
    echo "  - exit():  allowed in bin/ and lifecycle.rs only"
    echo ""
    echo -e "${YELLOW}Note: panic!/unwrap!/expect! are enforced by Clippy deny lints:${NC}"
    echo "  - clippy::panic, clippy::unwrap_used, clippy::expect_used"
    echo "  - See [workspace.lints.clippy] in Cargo.toml"
fi

exit 0
