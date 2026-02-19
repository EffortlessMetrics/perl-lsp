# perl-lsp

[![Crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![Documentation](https://docs.rs/perl-lsp/badge.svg)](https://docs.rs/perl-lsp)

Standalone **Language Server Protocol (LSP) server for Perl**, providing IDE features such as diagnostics, completion, navigation, formatting, rename, semantic tokens, code actions, and inlay hints. Communicates over stdio or TCP and integrates with any LSP-compatible editor.

## Installation

```bash
cargo install perl-lsp
```

## Usage

```bash
perl-lsp --stdio          # stdio mode (default, for editor integration)
perl-lsp --socket --port 9257  # TCP socket mode
perl-lsp --health         # health check
perl-lsp --version        # version info
```

## Public API

The `perl_lsp` library re-exports `LspServer`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, and a convenience `run_stdio()` entry point for embedding.

## Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace

Tier 6 executable crate. Delegates parsing to `perl-parser` and dispatches LSP features through dedicated provider crates (`perl-lsp-completion`, `perl-lsp-navigation`, `perl-lsp-diagnostics`, etc.).

## License

MIT OR Apache-2.0
