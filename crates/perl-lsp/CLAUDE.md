# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp` is the **top-level LSP server binary** providing a complete Language Server Protocol implementation for Perl.

**Purpose**: Perl Language Server (LSP) — Tree-sitter-compatible with comprehensive IDE features including completion, navigation, diagnostics, formatting, and refactoring.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lsp                   # Build this crate
cargo build -p perl-lsp --release         # Build optimized
cargo install --path crates/perl-lsp      # Install from source
cargo test -p perl-lsp                    # Run tests
cargo clippy -p perl-lsp                  # Lint
cargo doc -p perl-lsp --open              # View documentation

# Run with threading constraints (recommended for tests)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

## Running the Server

```bash
# Stdio mode (for editor integration)
./target/release/perl-lsp --stdio

# With logging
RUST_LOG=info ./target/release/perl-lsp --stdio
```

## Architecture

### Role in Workspace

This is a **Tier 6 executable crate** — the main entry point for Perl IDE support.

### Key Dependencies

**Core**:
- `perl-parser` (with `test-performance` feature)
- `perl-lsp-providers` - Provider aggregation
- `perl-lsp-protocol` - JSON-RPC/LSP types
- `perl-lsp-transport` - Message framing
- `perl-lsp-formatting` - perltidy integration
- `perl-lsp-tooling` - External tool support

**All LSP Feature Crates**:
- `perl-lsp-completion`, `perl-lsp-navigation`, `perl-lsp-diagnostics`
- `perl-lsp-code-actions`, `perl-lsp-rename`, `perl-lsp-semantic-tokens`
- `perl-lsp-inlay-hints`

### Main Modules

| Path | Purpose |
|------|---------|
| `main.rs` | Server entry point |
| `lib.rs` | Core server library |
| `dispatch.rs` | Request/notification routing |
| `state/` | Server state management |
| `state/config.rs` | Configuration handling |
| `state/document.rs` | Document tracking |
| `state/limits.rs` | Resource limits |
| `runtime/` | Async runtime handling |
| `runtime/routing.rs` | Message routing |
| `runtime/text_sync.rs` | Text document sync |
| `runtime/refresh.rs` | Capability refresh |
| `util/uri.rs` | URI conversion utilities |
| `transport/` | Transport layer integration |
| `textdoc.rs` | Text document handling |
| `diagnostics_catalog.rs` | Diagnostic codes |
| `call_hierarchy_provider.rs` | Call hierarchy |
| `execute_command.rs` | Command execution |
| `cancellation.rs` | Request cancellation |
| `fallback/` | Fallback implementations |

## Features

| Feature | Purpose |
|---------|---------|
| `workspace` | Workspace indexing support (default) |
| `incremental` | Incremental parsing support |
| `test-performance` | Performance testing support |
| `dap-phase1` | Debug Adapter Protocol phase 1 |
| `lsp-ga-lock` | Emergency capability lock |

## LSP Capabilities

The server supports:

| Capability | Provider Crate |
|------------|----------------|
| `textDocument/completion` | `perl-lsp-completion` |
| `textDocument/definition` | `perl-lsp-navigation` |
| `textDocument/references` | `perl-lsp-navigation` |
| `textDocument/hover` | `perl-lsp-navigation` |
| `textDocument/documentSymbol` | `perl-lsp-navigation` |
| `textDocument/formatting` | `perl-lsp-formatting` |
| `textDocument/codeAction` | `perl-lsp-code-actions` |
| `textDocument/rename` | `perl-lsp-rename` |
| `textDocument/semanticTokens` | `perl-lsp-semantic-tokens` |
| `textDocument/inlayHint` | `perl-lsp-inlay-hints` |
| `textDocument/publishDiagnostics` | `perl-lsp-diagnostics` |

## Threading Model

The server uses adaptive threading. Key environment variables:

```bash
RUST_TEST_THREADS=2     # Limit test parallelism
CARGO_BUILD_JOBS=1      # Limit build parallelism (CI)
```

## Important Notes

- This is the user-facing binary — stability is critical
- URI handling has one `#[allow(clippy::expect_used)]` (see `util/uri.rs`)
- Test with threading constraints to avoid resource exhaustion
- See `features.toml` in root for canonical capability definitions
