#!/usr/bin/env bash
# Populate mdBook with existing documentation
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BOOK_SRC="$REPO_ROOT/book/src"
DOCS_DIR="$REPO_ROOT/docs"

echo "Populating mdBook with existing documentation..."

# Create directory structure
echo "Creating directory structure..."
mkdir -p "$BOOK_SRC/getting-started"
mkdir -p "$BOOK_SRC/user-guides"
mkdir -p "$BOOK_SRC/architecture"
mkdir -p "$BOOK_SRC/developer"
mkdir -p "$BOOK_SRC/lsp"
mkdir -p "$BOOK_SRC/advanced"
mkdir -p "$BOOK_SRC/reference"
mkdir -p "$BOOK_SRC/dap"
mkdir -p "$BOOK_SRC/ci"
mkdir -p "$BOOK_SRC/process"
mkdir -p "$BOOK_SRC/resources"

# Function to copy and adapt a doc file
copy_doc() {
    local source="$1"
    local dest="$2"

    if [ -f "$source" ]; then
        echo "  Copying $(basename "$source") to $dest"
        cp "$source" "$dest"
    else
        echo "  Warning: Source file not found: $source"
    fi
}

# Getting Started section
echo "Setting up Getting Started..."
copy_doc "$DOCS_DIR/EDITOR_SETUP.md" "$BOOK_SRC/getting-started/editor-setup.md"
copy_doc "$DOCS_DIR/CONFIG.md" "$BOOK_SRC/getting-started/configuration.md"
copy_doc "$DOCS_DIR/ORIENTATION.md" "$BOOK_SRC/getting-started/first-steps.md"

copy_doc "$DOCS_DIR/GETTING_STARTED.md" "$BOOK_SRC/getting-started/installation.md"

# User Guides section
echo "Setting up User Guides..."
copy_doc "$DOCS_DIR/LSP_FEATURES.md" "$BOOK_SRC/user-guides/lsp-features.md"
copy_doc "$DOCS_DIR/WORKSPACE_NAVIGATION_GUIDE.md" "$BOOK_SRC/user-guides/workspace-navigation.md"
copy_doc "$DOCS_DIR/DEBUGGING.md" "$BOOK_SRC/user-guides/debugging.md"
copy_doc "$DOCS_DIR/TROUBLESHOOTING.md" "$BOOK_SRC/user-guides/troubleshooting.md"
copy_doc "$DOCS_DIR/KNOWN_LIMITATIONS.md" "$BOOK_SRC/user-guides/known-limitations.md"

# Architecture section
echo "Setting up Architecture..."
copy_doc "$DOCS_DIR/ARCHITECTURE_OVERVIEW.md" "$BOOK_SRC/architecture/overview.md"
copy_doc "$DOCS_DIR/CRATE_ARCHITECTURE_GUIDE.md" "$BOOK_SRC/architecture/crate-structure.md"
copy_doc "$DOCS_DIR/MODERN_ARCHITECTURE.md" "$BOOK_SRC/architecture/modern-architecture.md"

copy_doc "$DOCS_DIR/ARCHITECTURE_OVERVIEW.md" "$BOOK_SRC/architecture/parser-design.md"

copy_doc "$DOCS_DIR/LSP_IMPLEMENTATION_GUIDE.md" "$BOOK_SRC/architecture/lsp-implementation.md"
copy_doc "$DOCS_DIR/CRATE_ARCHITECTURE_DAP.md" "$BOOK_SRC/architecture/dap-implementation.md"

# Developer Guides section
echo "Setting up Developer Guides..."
copy_doc "$REPO_ROOT/CONTRIBUTING.md" "$BOOK_SRC/developer/contributing.md"
copy_doc "$DOCS_DIR/COMMANDS_REFERENCE.md" "$BOOK_SRC/developer/commands-reference.md"
copy_doc "$DOCS_DIR/COMPREHENSIVE_TESTING_GUIDE.md" "$BOOK_SRC/developer/testing-guide.md"
copy_doc "$DOCS_DIR/TEST_INFRASTRUCTURE_GUIDE.md" "$BOOK_SRC/developer/test-infrastructure.md"
copy_doc "$DOCS_DIR/API_DOCUMENTATION_STANDARDS.md" "$BOOK_SRC/developer/api-documentation-standards.md"
copy_doc "$DOCS_DIR/DEVELOPMENT.md" "$BOOK_SRC/developer/development-workflow.md"

# LSP Development section
echo "Setting up LSP Development..."
copy_doc "$DOCS_DIR/LSP_IMPLEMENTATION_GUIDE.md" "$BOOK_SRC/lsp/implementation-guide.md"
copy_doc "$DOCS_DIR/LSP_PROVIDERS_REFERENCE.md" "$BOOK_SRC/lsp/providers-reference.md"
copy_doc "$DOCS_DIR/LSP_FEATURE_IMPLEMENTATION_BEST_PRACTICES.md" "$BOOK_SRC/lsp/feature-implementation.md"
copy_doc "$DOCS_DIR/LSP_CANCELLATION_PROTOCOL.md" "$BOOK_SRC/lsp/cancellation-system.md"
copy_doc "$DOCS_DIR/ERROR_HANDLING_STRATEGY.md" "$BOOK_SRC/lsp/error-handling.md"

# Advanced Topics section
echo "Setting up Advanced Topics..."
copy_doc "$DOCS_DIR/PERFORMANCE_PRESERVATION_GUIDE.md" "$BOOK_SRC/advanced/performance-guide.md"
copy_doc "$DOCS_DIR/INCREMENTAL_PARSING_GUIDE.md" "$BOOK_SRC/advanced/incremental-parsing.md"
copy_doc "$DOCS_DIR/THREADING_CONFIGURATION_GUIDE.md" "$BOOK_SRC/advanced/threading-configuration.md"
copy_doc "$DOCS_DIR/SECURITY_DEVELOPMENT_GUIDE.md" "$BOOK_SRC/advanced/security-development.md"
copy_doc "$DOCS_DIR/MUTATION_TESTING_METHODOLOGY.md" "$BOOK_SRC/advanced/mutation-testing.md"

# Reference section
echo "Setting up Reference..."
copy_doc "$DOCS_DIR/CURRENT_STATUS.md" "$BOOK_SRC/reference/current-status.md"
copy_doc "$DOCS_DIR/ROADMAP.md" "$BOOK_SRC/reference/roadmap.md"
copy_doc "$DOCS_DIR/MILESTONES.md" "$BOOK_SRC/reference/milestones.md"
copy_doc "$DOCS_DIR/STABILITY.md" "$BOOK_SRC/reference/stability.md"
copy_doc "$DOCS_DIR/UPGRADING.md" "$BOOK_SRC/reference/upgrading.md"
copy_doc "$DOCS_DIR/ERROR_HANDLING_API_CONTRACTS.md" "$BOOK_SRC/reference/error-handling-contracts.md"
copy_doc "$DOCS_DIR/LSP_MISSING_FEATURES_REPORT.md" "$BOOK_SRC/reference/lsp-missing-features.md"

# DAP section
echo "Setting up DAP..."
copy_doc "$DOCS_DIR/DAP_USER_GUIDE.md" "$BOOK_SRC/dap/user-guide.md"
copy_doc "$DOCS_DIR/DAP_IMPLEMENTATION_SPECIFICATION.md" "$BOOK_SRC/dap/implementation.md"
copy_doc "$DOCS_DIR/DAP_SECURITY_SPECIFICATION.md" "$BOOK_SRC/dap/security.md"
copy_doc "$DOCS_DIR/DAP_BRIDGE_SETUP_GUIDE.md" "$BOOK_SRC/dap/bridge-setup.md"
copy_doc "$DOCS_DIR/DAP_PROTOCOL_SCHEMA.md" "$BOOK_SRC/dap/protocol-schema.md"

# CI & Quality section
echo "Setting up CI & Quality..."
copy_doc "$DOCS_DIR/CI.md" "$BOOK_SRC/ci/overview.md"
copy_doc "$DOCS_DIR/CI_LOCAL_VALIDATION.md" "$BOOK_SRC/ci/local-validation.md"
copy_doc "$DOCS_DIR/CI_TEST_LANES.md" "$BOOK_SRC/ci/test-lanes.md"
copy_doc "$DOCS_DIR/CI_COST_TRACKING.md" "$BOOK_SRC/ci/cost-tracking.md"
copy_doc "$DOCS_DIR/DEBT_TRACKING.md" "$BOOK_SRC/ci/debt-tracking.md"

# Process & Governance section
echo "Setting up Process & Governance..."
copy_doc "$DOCS_DIR/AGENTIC_DEV.md" "$BOOK_SRC/process/agentic-dev.md"
copy_doc "$DOCS_DIR/LESSONS.md" "$BOOK_SRC/process/lessons.md"
copy_doc "$DOCS_DIR/CASEBOOK.md" "$BOOK_SRC/process/casebook.md"
copy_doc "$DOCS_DIR/DOCUMENTATION_TRUTH_SYSTEM.md" "$BOOK_SRC/process/documentation-truth.md"
copy_doc "$DOCS_DIR/QUALITY_SURFACES.md" "$BOOK_SRC/process/quality-surfaces.md"

# Additional Resources section
# Static resource files (adr.md, benchmarks.md, forensics.md, issue-tracking.md)
# are committed in book/src/resources/ and do not need to be generated.
echo "Setting up Additional Resources..."
copy_doc "$DOCS_DIR/GA_RUNBOOK.md" "$BOOK_SRC/resources/ga-runbook.md"

echo "Documentation population complete!"
echo "Next steps:"
echo "  1. Review the populated files"
echo "  2. Run: mdbook build book"
echo "  3. Run: mdbook serve book"
