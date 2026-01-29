# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-protocol` is a **Tier 2 protocol definition crate** providing JSON-RPC/LSP protocol types and capability configuration.

**Purpose**: JSON-RPC/LSP protocol types and capability configuration for perl-lsp — defines server capabilities and protocol structures.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-protocol         # Build this crate
cargo test -p perl-lsp-protocol          # Run tests
cargo clippy -p perl-lsp-protocol        # Lint
cargo doc -p perl-lsp-protocol --open    # View documentation
```

## Architecture

### Dependencies

- `serde`, `serde_json` - Protocol serialization
- `lsp-types` - Standard LSP type definitions

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-ga-lock` | Emergency capability lock |

### Key Types

| Type | Purpose |
|------|---------|
| `ServerCapabilities` | What the server supports |
| `InitializeParams` | Client initialization request |
| `InitializeResult` | Server initialization response |

### Capability Configuration

```rust
use perl_lsp_protocol::capabilities;

// Build server capabilities
let caps = capabilities::server_capabilities();

// Capabilities include:
// - textDocumentSync
// - completionProvider
// - definitionProvider
// - referencesProvider
// - hoverProvider
// - documentFormattingProvider
// - codeActionProvider
// - renameProvider
// - semanticTokensProvider
// - inlayHintProvider
```

## Usage

```rust
use perl_lsp_protocol::{ServerCapabilities, InitializeResult};

fn handle_initialize(params: InitializeParams) -> InitializeResult {
    InitializeResult {
        capabilities: ServerCapabilities::default(),
        server_info: Some(ServerInfo {
            name: "perl-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    }
}
```

### Emergency Lock

The `lsp-ga-lock` feature can disable experimental capabilities:

```rust
#[cfg(feature = "lsp-ga-lock")]
fn server_capabilities() -> ServerCapabilities {
    // Return stable-only capabilities
}
```

## Important Notes

- See `features.toml` in root for canonical capability definitions
- Changes affect client compatibility
- Protocol version is tied to `lsp-types` version
- Emergency lock for production stability
