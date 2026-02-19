# Introduction

Welcome to the **perl-lsp** documentation! This project provides a comprehensive Perl parsing and IDE support ecosystem built with modern Rust technologies.

## What is perl-lsp?

perl-lsp is a production-ready Language Server Protocol (LSP) implementation for Perl 5, offering:

- **Fast Native Parser**: Built with Rust for near-complete Perl 5 syntax coverage (~100%)
- **LSP Server**: Full-featured language server with autocompletion, go-to-definition, refactoring, and more
- **Debug Adapter**: DAP support for debugging Perl applications
- **Multiple Crates**: Modular architecture with specialized components for parsing, lexing, and IDE integration
- **Enterprise Quality**: Comprehensive testing, mutation hardening, and API documentation enforcement

## Key Features

### Parser (perl-parser)
- Near-complete Perl 5 syntax coverage
- Tree-sitter compatible S-expression output
- Incremental parsing with sub-millisecond updates
- Robust error recovery

### LSP Server (perl-lsp)
- Autocompletion and signature help
- Go-to-definition and find-references
- Workspace-wide symbol navigation
- Rename refactoring with dual indexing
- Hover documentation
- Diagnostic reporting
- Code actions and workspace commands

### Debug Adapter (perl-dap)
- Native CLI debugging interface
- BridgeAdapter for IDE integration
- Breakpoint management and validation
- Stack trace inspection
- Enterprise security features

## Project Status

**Latest Release**: v0.9.0
**API Stability**: See [Stability Statement](./reference/stability.md)
**Current Milestone**: v1.0.0 - Boring Promises

The project is actively developed with a focus on production readiness, comprehensive testing, and enterprise-grade quality assurance.

## Architecture

The perl-lsp ecosystem consists of five specialized crates:

1. **perl-parser**: Core parsing library with LSP provider traits
2. **perl-lsp**: Standalone LSP server binary
3. **perl-dap**: Debug Adapter Protocol implementation
4. **perl-lexer**: Context-aware tokenizer
5. **perl-corpus**: Comprehensive test corpus

See the [Architecture section](./architecture/overview.md) for detailed design documentation.

## Who Should Use This Documentation?

This documentation is organized to serve different audiences:

- **Users**: Learn how to install and use the LSP server in your editor
- **Developers**: Understand the architecture and contribute to the project
- **LSP Implementers**: Dive deep into the LSP provider system and protocols
- **Quality Engineers**: Explore testing, benchmarking, and CI infrastructure

## Documentation Structure

The documentation follows the [Diataxis framework](https://diataxis.fr/):

- **Tutorials**: Step-by-step learning paths (Getting Started)
- **How-to Guides**: Task-oriented practical guides (User Guides, Developer Guides)
- **Explanations**: Conceptual understanding (Architecture, Advanced Topics)
- **Reference**: Technical specifications and API documentation (Reference)

## Getting Started

Ready to dive in? Here's where to go next:

1. [Quick Start](./quick-start.md) - Get up and running in 5 minutes
2. [Installation](./getting-started/installation.md) - Detailed installation instructions
3. [Editor Setup](./getting-started/editor-setup.md) - Configure your editor
4. [LSP Features](./user-guides/lsp-features.md) - Explore available features

## Need Help?

- **Issues**: Report bugs or request features on [GitHub](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Troubleshooting**: See the [Troubleshooting Guide](./user-guides/troubleshooting.md)
- **Contributing**: Read the [Contributing Guide](./developer/contributing.md)

## License

perl-lsp is dual-licensed under MIT and Apache 2.0 licenses.
