#!/usr/bin/env bash
# verify-docs-rs.sh
# Run `cargo doc --no-deps` for every publishable crate and report failures.
# Simulates docs.rs build conditions locally.
#
# Usage: bash scripts/verify-docs-rs.sh [--fast]
#   --fast: skip large crates (perl-parser, perl-lsp-rs) to reduce runtime
#
# Exit codes:
#   0: all crates documented successfully
#   1: one or more crates failed

set -euo pipefail

FAST_MODE=0
for arg in "$@"; do
    case "$arg" in
        --fast) FAST_MODE=1 ;;
    esac
done

# Full publishable list in topological order (from workspace Cargo.toml metadata)
ALL_CRATES=(
    # Tier 1 — Leaf crates
    perl-position-tracking
    perl-token
    perl-builtins
    perl-builtins-phf
    perl-content-length-framing
    perl-percentile
    perl-subprocess-runtime
    perl-lsp-protocol
    # (perl-lsp-symbol-query absorbed into perl-lsp-rs-core::providers — Wave G1a)
    perl-keywords
    perl-regex
    perl-quote
    perl-pod
    perl-heredoc-anti-patterns
    tree-sitter-perl-c
    # Tier 1b
    perl-ast
    perl-ast-v2
    perl-lexer
    perl-heredoc
    # Tier 1c
    perl-error
    perl-pragma
    perl-tokenizer
    # Tier 2 — Core infrastructure
    perl-parser-core
    perl-test-must
    perl-tdd-support
    perl-edit
    perl-incremental-parsing
    perl-symbol
    perl-uri-classify
    perl-uri
    perl-workspace-index-state-machine
    perl-path-normalize
    perl-workspace-index-slo
    # Tier 3
    perl-workspace-index
    perl-semantic-analyzer
    perl-lsp-diagnostic-types
    # (perl-lsp-diagnostics absorbed into perl-lsp-rs-core::providers::diagnostics — Wave G1b)
    perl-lsp-text-utils
    # (perl-lsp-rename absorbed into perl-lsp-rs-core::providers::rename — Wave G1b)
    # (perl-lsp-code-actions absorbed into perl-lsp-rs-core::providers::code_actions — Wave G1b)
    # (perl-lsp-folding absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-selection-range absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-completion absorbed into perl-lsp-rs-core::providers::completion — Wave G1b)
    # (perl-lsp-file-completion absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-completion-item absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-inline-completion absorbed into perl-lsp-rs-core::providers::inline_completion — Wave G1b)
    # (perl-lsp-ai-provider absorbed into perl-lsp-rs-core::providers::ai — Wave G1b)
    # (perl-lsp-inlay-hints absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-code-lens absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-color-provider absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # Module resolution chain
    perl-module-name
    perl-module-path
    perl-module-token-core
    perl-module-boundary
    perl-module-token
    perl-module-import
    # Tier 4 — LSP providers
    # (perl-lsp-navigation absorbed into perl-lsp-rs-core::providers::navigation — Wave G1b)
    # (perl-lsp-type-hierarchy absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-document-highlight absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-document-links absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-workspace-symbols absorbed into perl-lsp-rs-core::providers — Wave G1a)
    perl-lsp-tooling
    perl-lsp-perltidy
    perl-lsp-performance
    perl-lsp-critic-parser
    # (perl-lsp-formatting-types absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-formatting absorbed into perl-lsp-rs-core::providers::formatting — Wave G1b)
    # (perl-lsp-on-type-formatting absorbed into perl-lsp-rs-core::providers — Wave G1a)
    # (perl-lsp-semantic-tokens absorbed into perl-lsp-rs-core::providers::semantic_tokens — Wave G1b)
    # (perl-lsp-providers absorbed into perl-lsp-rs-core::providers::lsp_compat — Wave G1b)
    perl-ast-utils
    # (perl-lsp-import-management absorbed into perl-lsp-rs-core::providers — Wave G1a)
    perl-qualified-name
    perl-refactoring
    # Tier 5
    perl-dead-code
    perl-parser
    perl-parser-pest
    perl-corpus
    # DAP
    perl-dap-breakpoint
    perl-dap-eval
    perl-dap-config
    perl-dap-command-args
    perl-dap-platform
    perl-dap-shell
    perl-dap-types
    perl-dap-security
    perl-dap-stack
    perl-dap-value
    perl-dap-variables
    perl-path-security
    perl-feature-catalog
    perl-dap
    # LSP governance
    perl-lsp-cancellation
    perl-lsp-input-validation
    perl-lsp-feature-ids
    perl-lsp-capability-map
    perl-lsp-feature-contracts
    perl-lsp-feature-flags
    perl-lsp-feature-profile
    perl-lsp-feature-profile-cli
    perl-lsp-feature-policy
    perl-lsp-feature-grid
    perl-lsp-feature-governance
    perl-lsp-launcher
    perl-lsp-diagnostic-catalog
    perl-lsp-limits
    perl-lsp-uri
    perl-lsp-config
    perl-lsp-transport
    # Tier 6
    perl-module-token-parser
    perl-text-line
    perl-line-index
    perl-module-reference
    perl-module-import-match
    perl-module-rename
    perl-module-resolution-path
    perl-workspace-folder
    perl-workspace-ignore
    perl-module-resolution-uri
    perl-module-resolution
    perl-source-file
    perl-workspace-discovery
    perl-diagnostics-codes
    # Tier 7
    perl-lsp-rs
    perllsp
    # tree-sitter facade (added in #3255, merged after this PR was cut)
    tree-sitter-perl-rs
)

# Large crates to skip in fast mode
SLOW_CRATES=(perl-parser perl-lsp-rs perl-corpus perllsp perl-dap perl-workspace-index)

PASSED=()
FAILED=()
SKIPPED=()

is_slow() {
    local crate="$1"
    for slow in "${SLOW_CRATES[@]}"; do
        if [[ "$crate" == "$slow" ]]; then
            return 0
        fi
    done
    return 1
}

echo "=== docs.rs verification (fast=$FAST_MODE) ==="
echo ""

for crate in "${ALL_CRATES[@]}"; do
    if [[ "$FAST_MODE" == "1" ]] && is_slow "$crate"; then
        SKIPPED+=("$crate")
        echo "  SKIP  $crate (fast mode)"
        continue
    fi

    printf "  %-45s " "$crate"
    if cargo doc --no-deps -p "$crate" --quiet 2>&1; then
        PASSED+=("$crate")
        echo "OK"
    else
        FAILED+=("$crate")
        echo "FAIL"
    fi
done

echo ""
echo "=== Results ==="
echo "  Passed:  ${#PASSED[@]}"
echo "  Failed:  ${#FAILED[@]}"
echo "  Skipped: ${#SKIPPED[@]}"

if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo ""
    echo "=== FAILURES ==="
    for crate in "${FAILED[@]}"; do
        echo "  $crate"
    done
    exit 1
fi

echo ""
echo "All crates documented successfully."
