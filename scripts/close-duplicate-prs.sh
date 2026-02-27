#!/bin/bash
# Script to close PRs that duplicate already-implemented functionality
# Based on comprehensive evaluation completed January 2025

set -e

echo "🔍 Closing duplicate/completed PRs based on evaluation..."
echo "See PR_EVALUATION_SUMMARY.md for detailed analysis"
echo

# PRs that duplicate already-implemented functionality
DUPLICATE_PRS=(
    "54:LSP real I/O testing - superseded by comprehensive lsp_harness.rs with JSON-RPC testing"
    "43:Generic I/O streams - already implemented via LspServer::with_output() constructor"  
    "26:Incremental node reuse - already working with 87.5% tree reuse demonstrated"
    "6:Line/column mapping - already implemented in position_mapper.rs with UTF-16/UTF-8 conversion"
    "11:Real LSP harness - already implemented with thread-safe communication and timeout support"
    "10:Real LSP responses - already implemented in comprehensive E2E tests (33+ tests)"
    "4:Incremental metrics - already enabled and working in production"
)

echo "The following PRs will be closed as duplicates of existing functionality:"
for pr_info in "${DUPLICATE_PRS[@]}"; do
    pr_num=$(echo "$pr_info" | cut -d: -f1)
    description=$(echo "$pr_info" | cut -d: -f2-)
    echo "  - PR #$pr_num: $description"
done
echo

read -p "Continue with closing these PRs? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Close each PR with appropriate comment
for pr_info in "${DUPLICATE_PRS[@]}"; do
    pr_num=$(echo "$pr_info" | cut -d: -f1)
    description=$(echo "$pr_info" | cut -d: -f2-)
    
    echo "Closing PR #$pr_num..."
    
    gh pr close "$pr_num" --comment "Closing this PR as the functionality has already been implemented.

**Implementation Status**: $description

**Evidence**: 
- See PR_EVALUATION_SUMMARY.md for detailed analysis
- Verified against current codebase (v0.8.7+ with comprehensive testing)
- All advertised LSP features have 100% E2E test coverage

This closure is part of repository cleanup to focus on actionable PRs. Thank you for the contribution!"

    echo "✅ Closed PR #$pr_num"
    echo
done

echo "🎉 Successfully closed ${#DUPLICATE_PRS[@]} duplicate PRs"
echo
echo "Next steps:"
echo "1. Review high-value implementation PRs: #12 (import optimizer), #40 (workspace refactor), #7 (refactor tools)"
echo "2. Evaluate enhancement PRs based on current feature gaps"  
echo "3. See PR_EVALUATION_SUMMARY.md for detailed priority recommendations"