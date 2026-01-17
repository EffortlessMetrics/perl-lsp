# tree-sitter-perl

[![CI](https://github.com/EffortlessSteven/tree-sitter-perl/actions/workflows/ci.yml/badge.svg)](https://github.com/EffortlessSteven/tree-sitter-perl/actions/workflows/ci.yml)
[![Tests](.github/badges/tests.svg)](https://github.com/EffortlessSteven/tree-sitter-perl/actions)
[![Coverage](.github/badges/coverage.svg)](https://github.com/EffortlessSteven/tree-sitter-perl/actions)
[![Benchmarks](https://github.com/EffortlessSteven/tree-sitter-perl/actions/workflows/benchmark.yml/badge.svg)](https://github.com/EffortlessSteven/tree-sitter-perl/actions/workflows/benchmark.yml)
[![Crates.io](https://img.shields.io/crates/v/perl-parser.svg)](https://crates.io/crates/perl-parser)
[![Documentation](https://docs.rs/perl-parser/badge.svg)](https://docs.rs/perl-parser)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

> **Production-Ready Perl Parsing Ecosystem - Five specialized crates for parsing, corpus testing, and IDE support**

This project provides a **complete Perl parsing ecosystem** with Tree-sitter compatibility:

### 📦 Published Crates

1. **perl-parser** ⭐ - Native Rust parser with ~100% Perl 5 coverage, 98% reference coverage improvement, and enhanced dual indexing LSP provider logic
2. **perl-lsp** 🔧 - Standalone Language Server binary with 99.5% performance optimization, Unicode enhancement, and production-ready CLI interface
3. **perl-lexer** - Context-aware tokenizer with enhanced Unicode processing, atomic performance tracking, and delimiter support
4. **perl-corpus** - Comprehensive test corpus and property testing
5. **perl-parser-pest** - Legacy Pest-based parser (use perl-parser for production)

All parsers output tree-sitter compatible S-expressions for seamless integration.

## 📚 Documentation (Diataxis)

Documentation is organized using the [Diataxis](https://diataxis.fr/) framework with comprehensive quality enforcement.

- **[Tutorials](docs/tutorials/)** – Quick start and hands-on guidance.
  - [Workspace Refactoring Tutorial](docs/tutorials/WORKSPACE_REFACTORING_TUTORIAL.md)
  - [Execute Command Tutorial](docs/tutorials/EXECUTE_COMMAND_TUTORIAL.md)
- **[How-to Guides](docs/how-to/)** – Problem-oriented solutions.
  - [Build and Test](docs/how-to/DEVELOPMENT.md)
  - [Commands Reference](docs/how-to/COMMANDS_REFERENCE.md)
  - [Comprehensive Testing Guide](docs/how-to/COMPREHENSIVE_TESTING_GUIDE.md)
  - [Security Development Guide](docs/how-to/SECURITY_DEVELOPMENT_GUIDE.md)
- **[Explanations](docs/explanation/)** – Design decisions and concepts.
  - [Architecture Overview](docs/explanation/ARCHITECTURE_OVERVIEW.md)
  - [Modern Architecture](docs/explanation/MODERN_ARCHITECTURE.md)
  - [Edge Case Handling](docs/explanation/EDGE_CASES.md)
- **[Reference](docs/reference/)** – Specifications and API docs.
  - [LSP Actual Status](LSP_ACTUAL_STATUS.md)
  - [Workspace Refactor API Reference](docs/reference/WORKSPACE_REFACTOR_API_REFERENCE.md)

See the [Documentation Guide](docs/how-to/DOCUMENTATION_GUIDE.md) for a complete map.

## 📦 Latest Release: v0.9.0-semantic-lsp-ready

See [CHANGELOG.md](CHANGELOG.md) for full details.

### Highlights

- **Semantic Analyzer Phase 1**: Complete core node handlers for precise semantic analysis (Variable declarations, Control flow, Data structures).
- **LSP Semantic Definition Integration**: Precise symbol resolution using semantic analysis instead of text search.
- **Statement Tracker & Heredoc Support**: 100% complete heredoc tracking and statement boundary detection.
- **Robustness**: Advanced fuzz testing, mutation hardening, and enhanced quote parser.

---

## 🚀 Quick Start

### Install the LSP Server

#### Option 1: Quick Install (Linux/macOS)
```bash
# One-liner installer
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
```

#### Option 2: Quick Install (Windows PowerShell)
```powershell
irm https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.ps1 | iex
```

#### Option 3: Download Binary
Download pre-built binaries from the [latest release](https://github.com/EffortlessSteven/tree-sitter-perl/releases/latest).

#### Option 4: Build from Source
```bash
# Install the perl-lsp binary from crates.io
cargo install perl-lsp

# Or, build from this repository
git clone https://github.com/EffortlessSteven/tree-sitter-perl
cd tree-sitter-perl
cargo build --release -p perl-lsp
# The binary will be in target/release/perl-lsp
```

### Use the Parser Library

```toml
# In your Cargo.toml
[dependencies]
perl-parser = "0.8.8"
```

```rust
use perl_parser::Parser;

let source = "my $x = 42;";
let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();
println!("AST: {:?}", ast);
```

---

## 🖥️ Language Server Protocol (LSP) Support

The v3 parser includes a **production-ready Language Server Protocol implementation** for Perl.

See [docs/reference/LSP_CAPABILITY_POLICY.md](docs/reference/LSP_CAPABILITY_POLICY.md) for capability policies and [LSP_ACTUAL_STATUS.md](LSP_ACTUAL_STATUS.md) for current feature matrix.

**Key Features:**
*   **Diagnostics:** Production-stable hash key context detection.
*   **Go to Definition:** Enhanced Package::subroutine support.
*   **Find References:** Enhanced dual-pattern search.
*   **Workspace Symbols:** Fast index search.
*   **Rename:** Cross-file support.
*   **Code Actions:** `use strict`, `use warnings`, perltidy.

---

## 📊 Performance

| Metric | Performance | Details |
|--------|-------------|---------|
| **Average Update Time** | **65µs** | For simple, single-line edits. |
| **Node Reuse Rate** | **96.8% - 99.7%** | High AST reuse efficiency. |
| **Statistical Consistency** | **<0.5 CoV** | Highly predictable performance. |

See [docs/explanation/BENCHMARK_FRAMEWORK.md](docs/benchmarks/BENCHMARK_FRAMEWORK.md) for comprehensive methodology.

---

## 🔧 Build and Test

### Prerequisites
* Rust 1.89+ (2024 edition)
* Cargo

### Quick Commands

```shell
# Build
cargo xtask build --release

# Run tests
cargo xtask test

# Run benchmarks
cargo xtask bench
```

See [docs/how-to/DEVELOPMENT.md](docs/how-to/DEVELOPMENT.md) for detailed development workflow.

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

---

## 📄 License

Licensed under either of
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

**Status**: Production-ready with comprehensive test coverage, CI/CD pipeline, and advanced Rust components.
