# Perl LSP v0.10.0 - Public Alpha

## Release Date
February 28, 2026

## Overview

Perl LSP v0.10.0 continues the **Public Alpha** series with version consistency improvements, expanded documentation, and release infrastructure hardening. This release transitions from the initial v0.10.0 alpha to a more polished, release-candidate-quality package while maintaining the project's trajectory toward the v0.15.0 Stability Contract milestone.

## 🚀 Key Highlights

### Initial Public Alpha
- **Fast & Native**: Recursive descent parser written in pure Rust (1-150μs typical).
- **Substantially Complete**: 99% coverage of the LSP 3.18 methods (alpha-validated).
- **High Performance**: sub-millisecond incremental updates and sub-50ms LSP responses.
- **Experimental Protocol**: Wire protocol and APIs are subject to change based on feedback.

### Performance
- **21μs Mean Parse Time**: Native recursive descent parser.
- **0.31s Test Suite**: Optimized execution through adaptive threading.
- **<1ms Incremental Updates**: Real-time parsing with 70-99% node reuse.

### Complete Semantic Analysis
- **100% AST Node Coverage**: All NodeKind handlers implemented.
- **Lexical Scoping**: Proper handling of nested scopes and package boundaries.
- **Cross-File Navigation**: Dual indexing for qualified and bare function calls.

### Security Focused
- **Hardened Foundations**: UTF-16 boundary fixes and path traversal prevention.
- **Memory Safe**: Full Rust memory safety guarantees.
- **Process Isolation**: Controlled execution for external tool integration.

## 🎯 What's New in v0.10.0

### Complete Semantic Analyzer
The semantic analyzer now provides a deep understanding of Perl code:
- All NodeKind handlers implemented.
- Proper lexical scoping with nested scope support.
- Package-qualified call resolution (`Package::function`).
- Shadowed variable detection.

### Debug Adapter Protocol (DAP) Support
Initial debugging capabilities available in VS Code and DAP-compatible editors:
- Phase 1 bridge to Perl::LanguageServer.
- Cross-platform support (Windows, macOS, Linux, WSL).
- <50ms breakpoint operations.

### Enhanced LSP Cancellation System
Thread-safe cancellation infrastructure for improved responsiveness:
- <100μs Check Latency.
- Global Registry for concurrent request coordination.
- JSON-RPC 2.0 compliance for `$/cancelRequest`.

## 🛠️ Installation & Setup

### Quick Install

```bash
# Install LSP server
cargo install perl-lsp

# Install DAP server (optional)
cargo install perl-dap

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

## 📊 Platform Support

| Platform | Architecture | Status | Binary |
|----------|-------------|--------|--------|
| Linux (GNU) | x86_64 | ✅ Tier 1 | Pre-built |
| Linux (musl) | x86_64 | ✅ Tier 1 | Pre-built |
| Linux (GNU) | aarch64 | ✅ Tier 1 | Pre-built |
| macOS | x86_64 | ✅ Tier 1 | Pre-built |
| macOS | aarch64 | ✅ Tier 1 | Pre-built |
| Windows | x86_64 | ✅ Tier 1 | Pre-built |

## 🔄 Origins & History

- **Project Start**: Q2 2025.
- **Code Fork**: Initially forked on July 15th, 2025 from [tree-sitter-perl-better](https://github.com/tree-sitter-perl/tree-sitter-perl).
- **Evolution**: Transitioned from a tree-sitter based system to a pure-Rust recursive descent architecture for performance and security.

## 📋 Roadmap & Stability

### v0.10.0 (Planned: April 2026)
- Enhanced DAP native implementation (Phase 2).
- Moo/Moose semantic depth (field recognition).
- Performance optimizations and refactoring refinements.

### v0.15.0 (Future Milestone)
- **Stability Contract**: Formal API stability and contract-locked wire protocol.
- Full protocol compliance audit.
- Package manager distribution.

## 🎯 Support & Community

- **GitHub Issues**: [Report bugs and request features](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [Community discussions and Q&A](https://github.com/EffortlessMetrics/perl-lsp/discussions)

## 📜 License

Dual licensed under [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).

---

**Try Perl LSP v0.10.0 today and help shape the future of Perl development!**

🚀 [Get Started Now](docs/GETTING_STARTED.md) | 📖 [Documentation](docs/INDEX.md) | 💬 [Community](https://github.com/EffortlessMetrics/perl-lsp/discussions)
