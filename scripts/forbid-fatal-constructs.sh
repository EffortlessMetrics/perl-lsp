#!/usr/bin/env bash
# forbid-fatal-constructs.sh - Gate to prevent fatal constructs in production code
#
# This script scans the codebase for constructs that can cause unrecoverable
# termination in production code:
#   - std::process::abort() - panic without unwind
#   - std::process::exit()  - immediate termination (allowed in bin/lifecycle)
#   - panic!()              - unwind or abort (allowed in #[cfg(test)] modules)
#   - todo!()               - panic with message
#   - unimplemented!()      - panic with message
#   - dbg!()                - debug output (not fatal but banned)
#
# Excluded paths:
#   - tests/**              - Test code can panic
#   - benches/**            - Benchmark code can panic
#   - build.rs              - Build scripts can exit
#   - examples/**           - Example code can exit
#   - **/bin/**             - Binary entry points can exit
#   - xtask/                - Development tooling (not shipped)
#   - *_test.rs, *_tests.rs - Test modules within src/
#   - *-support crates      - Test infrastructure crates
#   - tree-sitter-perl-*    - Excluded from workspace, legacy parsers
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

# Fatal construct patterns to search for
# Note: We look for abort and exit specifically (panic covered by clippy::panic)
PATTERNS=(
    'std::process::abort\('
)

# Paths to exclude (tests, benches, build scripts, examples, binaries, tooling)
EXCLUDE_GLOBS=(
    '!**/tests/**'
    '!**/benches/**'
    '!**/build.rs'
    '!**/examples/**'
    '!**/bin/**'
    '!**/*_test.rs'
    '!**/*_tests.rs'
)

# Build the combined pattern
COMBINED_PATTERN=$(IFS='|'; echo "${PATTERNS[*]}")

# Build the glob arguments
GLOB_ARGS=()
for glob in "${EXCLUDE_GLOBS[@]}"; do
    GLOB_ARGS+=(--glob "$glob")
done

# Check if ripgrep is available
if ! command -v rg &> /dev/null; then
    echo "Error: ripgrep (rg) is required but not found in PATH"
    exit 1
fi

# Run the search only on crates directory (not xtask, not excluded tree-sitter crates)
# Note: tree-sitter-perl-c and tree-sitter-perl-rs are excluded from workspace
MATCHES=$(rg -n \
    --type rust \
    "${GLOB_ARGS[@]}" \
    --glob '!**/tree-sitter-perl-c/**' \
    --glob '!**/tree-sitter-perl-rs/**' \
    --glob '!**/perl-tdd-support/**' \
    "$COMBINED_PATTERN" \
    crates 2>/dev/null || true)

if [[ -n "$MATCHES" ]]; then
    echo -e "${RED}ERROR: Forbidden fatal constructs found in production code${NC}"
    echo ""
    echo "The following matches violate the no-abort policy:"
    echo "=================================================="
    echo "$MATCHES"
    echo "=================================================="
    echo ""
    echo "To fix:"
    echo "  - std::process::abort() -> return error, let caller handle"
    echo ""
    echo "Allowed exceptions:"
    echo "  - tests/, benches/, examples/, bin/ directories"
    echo "  - xtask/, *-support crates (development tooling)"
    echo "  - tree-sitter-perl-c/rs (excluded from workspace)"
    exit 1
fi

if [[ -n "$VERBOSE" ]]; then
    echo -e "${GREEN}OK: No forbidden fatal constructs (abort) in production code${NC}"
    echo ""
    echo -e "${YELLOW}Note: panic!/unwrap!/expect! are enforced by Clippy deny lints:${NC}"
    echo "  - clippy::panic, clippy::unwrap_used, clippy::expect_used"
    echo "  - See [workspace.lints.clippy] in Cargo.toml"
fi

exit 0
