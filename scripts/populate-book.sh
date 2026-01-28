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
copy_doc "$DOCS_DIR/START_HERE.md" "$BOOK_SRC/getting-started/first-steps.md"

# Create installation guide from README
cat > "$BOOK_SRC/getting-started/installation.md" << 'EOF'
# Installation

## From crates.io

The recommended way to install perl-lsp is via crates.io:

```bash
cargo install perl-lsp
```

## From Source

Clone the repository and build:

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl.git
cd tree-sitter-perl
cargo build --release -p perl-lsp
```

The binary will be located at `target/release/perl-lsp`.

## System Requirements

- Rust toolchain 1.70 or later
- Perl 5 installation (for testing)
- Sufficient memory for workspace indexing (typically 512MB+)

## Verifying Installation

Check that perl-lsp is properly installed:

```bash
perl-lsp --version
```

## Next Steps

- [Editor Setup](./editor-setup.md)
- [Configuration](./configuration.md)
- [First Steps](./first-steps.md)
EOF

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

# Create parser design doc
cat > "$BOOK_SRC/architecture/parser-design.md" << 'EOF'
# Parser Design

The perl-parser crate implements a recursive descent parser for Perl 5 syntax.

## Key Features

- Near-complete Perl 5 syntax coverage (~100%)
- Tree-sitter compatible output
- Incremental parsing support
- Robust error recovery
- Context-aware lexing

## Architecture

The parser follows a multi-stage pipeline:

1. **Lexical Analysis**: Context-aware tokenization
2. **Parsing**: Recursive descent with error recovery
3. **AST Construction**: Build abstract syntax tree
4. **Serialization**: Output S-expressions for Tree-sitter compatibility

See the [LSP Implementation Guide](../lsp/implementation-guide.md) for integration details.
EOF

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
echo "Setting up Additional Resources..."
cat > "$BOOK_SRC/resources/adr.md" << 'EOF'
# Architecture Decision Records (ADRs)

Architecture Decision Records document significant architectural decisions made in the project.

## Available ADRs

See the [adr/ directory](https://github.com/EffortlessMetrics/tree-sitter-perl/tree/master/docs/adr) for all ADRs.

Key ADRs:

- ADR 002: API Documentation Infrastructure
- Additional ADRs are available in the repository

## ADR Format

Each ADR follows this structure:

1. Context
2. Decision
3. Consequences
4. Status
EOF

cat > "$BOOK_SRC/resources/benchmarks.md" << 'EOF'
# Benchmarks

Performance benchmarks are tracked in the repository.

## Benchmark Reports

- [Benchmark Framework](https://github.com/EffortlessMetrics/tree-sitter-perl/blob/master/docs/benchmarks/BENCHMARK_FRAMEWORK.md)
- [Benchmark Report](https://github.com/EffortlessMetrics/tree-sitter-perl/blob/master/docs/benchmarks/BENCHMARK_REPORT.md)

## Running Benchmarks

```bash
cargo bench
```

See the [Performance Guide](../advanced/performance-guide.md) for details.
EOF

cat > "$BOOK_SRC/resources/forensics.md" << 'EOF'
# Forensics

PR archaeology and investigation documentation.

## Forensics Schema

See the [Forensics Schema](https://github.com/EffortlessMetrics/tree-sitter-perl/blob/master/docs/FORENSICS_SCHEMA.md) for the investigation template.

## Examples

Forensics examples are available in the [docs/forensics](https://github.com/EffortlessMetrics/tree-sitter-perl/tree/master/docs/forensics) directory.
EOF

cat > "$BOOK_SRC/resources/issue-tracking.md" << 'EOF'
# Issue Tracking

Issue status and tracking documentation.

## Current Issues

See [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl/issues) for active issues.

## Milestones

Active milestones:
- v0.9.1: Close-out
- v1.0.0: Boring Promises

See [Milestones](../reference/milestones.md) for details.
EOF

copy_doc "$DOCS_DIR/GA_RUNBOOK.md" "$BOOK_SRC/resources/ga-runbook.md"

echo "Documentation population complete!"
echo "Next steps:"
echo "  1. Review the populated files"
echo "  2. Run: mdbook build book"
echo "  3. Run: mdbook serve book"
