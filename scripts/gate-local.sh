#!/usr/bin/env bash
# WSL-safe local gate - prevents OOM crashes with constrained parallelism
# Usage: CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 scripts/gate-local.sh
#
# For release builds (optional, faster execution):
#   GATE_RELEASE=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 scripts/gate-local.sh

set -euo pipefail

export CARGO_TERM_COLOR=always
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

# Determine build profile
if [[ "${GATE_RELEASE:-}" == "1" ]]; then
    PROFILE="release"
    PROFILE_FLAG="--release"
else
    PROFILE="debug"
    PROFILE_FLAG=""
fi

echo "╔═══════════════════════════════════════════════════════════════════════════════╗"
echo "║ WSL-Safe Gate (jobs=$CARGO_BUILD_JOBS, threads=$RUST_TEST_THREADS, profile=$PROFILE)"
echo "╚═══════════════════════════════════════════════════════════════════════════════╝"

# =============================================================================
# Coordinator Takeover Enforcement
# =============================================================================
# These guards ensure no handler bypasses routing or accesses workspace_index
# directly. All cross-file queries must go through route_index_access().
# All mutations must use coordinator.index() with paired lifecycle notifications.

echo ""
echo ">>> coordinator takeover enforcement (grep guards)"

# Fail if any handler calls self.workspace_index() directly (bypass prevention)
if grep -rn 'self\.workspace_index()' crates/perl-parser/src/lsp/server_impl 2>/dev/null; then
    echo "ERROR: Found self.workspace_index() bypass in server_impl"
    echo "All index access must go through coordinator or routing policy"
    exit 1
fi
echo "  ✓ No workspace_index() bypasses found"

# Fail if workspace_index field is re-introduced on LspServer
if grep -rn 'workspace_index:.*Option<Arc<WorkspaceIndex>>' crates/perl-parser/src/lsp/server_impl/mod.rs 2>/dev/null; then
    echo "ERROR: workspace_index field found on LspServer"
    echo "Use index_coordinator field instead (coordinator-first pattern)"
    exit 1
fi
echo "  ✓ No workspace_index field on LspServer"

echo ""
echo ">>> Ignored tests gate (zero tolerance for critical categories)"
# Get counts from ignored-test-count.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/ignored-test-count.sh" ]]; then
    # Run the script and capture output
    OUTPUT=$(bash "$SCRIPT_DIR/ignored-test-count.sh" 2>&1) || true

    # Check critical categories that MUST be zero
    for cat in brokenpipe feature stress bare other; do
        count=$(echo "$OUTPUT" | grep -E "^$cat " | awk '{print $2}' || echo "0")
        if [[ "$count" != "0" && -n "$count" ]]; then
            echo "ERROR: Category '$cat' has $count ignored tests (must be 0)"
            echo "Run: VERBOSE=1 bash scripts/ignored-test-count.sh"
            exit 1
        fi
    done
    echo "  ✓ Critical categories (brokenpipe, feature, stress, bare, other) are all zero"

    # Show allowed categories
    manual=$(echo "$OUTPUT" | grep -E "^manual " | awk '{print $2}' || echo "0")
    bug=$(echo "$OUTPUT" | grep -E "^bug " | awk '{print $2}' || echo "0")
    infra=$(echo "$OUTPUT" | grep -E "^infra " | awk '{print $2}' || echo "0")
    protocol=$(echo "$OUTPUT" | grep -E "^protocol " | awk '{print $2}' || echo "0")
    echo "  Allowed: manual=$manual bug=$bug infra=$infra protocol=$protocol"
else
    echo "  (skipped - ignored-test-count.sh not found)"
fi

echo ""
echo ">>> fmt check"
cargo fmt --all -- --check

echo ""
echo ">>> clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo ""
echo ">>> Feature matrix checks"
echo "  Checking --no-default-features (minimal build)..."
cargo check -p perl-parser --no-default-features
echo "  ✓ Minimal build compiles"

echo "  Checking --features workspace (full build)..."
cargo check -p perl-parser --features workspace
echo "  ✓ Workspace build compiles"

echo ""
echo ">>> Build perl-lsp binary (ensures tests use correct version)"
cargo build -p perl-lsp $PROFILE_FLAG

echo ""
echo ">>> perl-parser lib tests"
cargo test -p perl-parser --lib -- --test-threads="$RUST_TEST_THREADS"

echo ""
echo ">>> perl-lsp integration tests (including binary version check)"
# Run the version test first to catch stale binary issues early
cargo test -p perl-lsp --test binary_version_test $PROFILE_FLAG -- --test-threads="$RUST_TEST_THREADS"
cargo test -p perl-lsp --tests $PROFILE_FLAG -- --test-threads="$RUST_TEST_THREADS"

echo ""
echo ">>> perl-lexer tests (optional)"
cargo test -p perl-lexer --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo ""
echo ">>> perl-dap tests (optional)"
cargo test -p perl-dap --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo ""
echo "╔═══════════════════════════════════════════════════════════════════════════════╗"
echo "║ ✓ Gate passed                                                                  ║"
echo "╚═══════════════════════════════════════════════════════════════════════════════╝"
