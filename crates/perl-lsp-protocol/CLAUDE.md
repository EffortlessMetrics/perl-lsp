# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-protocol` is a **Tier 1 leaf crate** (no internal workspace dependencies) providing JSON-RPC/LSP protocol types and capability configuration.

**Purpose**: Isolates protocol types from the LSP runtime so they can be shared across binaries and provider layers.

**Version**: 0.9.1

## Commands

```bash
cargo build -p perl-lsp-protocol         # Build this crate
cargo test -p perl-lsp-protocol          # Run tests
cargo clippy -p perl-lsp-protocol        # Lint
cargo doc -p perl-lsp-protocol --open    # View documentation
```

## Architecture

### Dependencies

- `serde`, `serde_json` -- Protocol serialization
- `lsp-types` (0.97) -- Standard LSP type definitions

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-ga-lock` | Conservative capability set for production stability |

### Modules

| Module | Visibility | Purpose |
|--------|-----------|---------|
| `capabilities` | `pub` | `BuildFlags`, `AdvertisedFeatures`, `capabilities_for()`, `default_capabilities()`, `capabilities_json()`, `get_supported_commands()` |
| `methods` | `pub` | `&str` constants for all LSP 3.17 method names (lifecycle, text document sync, language features, workspace, window, special) |
| `jsonrpc` | re-exported | `JsonRpcRequest` (Deserialize), `JsonRpcResponse` (Serialize), `JsonRpcError` (Serialize + Error) |
| `errors` | re-exported | Error code constants (`PARSE_ERROR`, `METHOD_NOT_FOUND`, `REQUEST_CANCELLED`, `SERVER_CANCELLED`, etc.), builder functions (`cancelled_response`, `method_not_found`, `internal_error`, `invalid_params`, `server_not_initialized`, `connection_closed_error`, `transport_error`), param extractors (`req_uri`, `req_position`, `req_range`) |

### Capability Configuration

```rust
use perl_lsp_protocol::capabilities::{BuildFlags, capabilities_for, default_capabilities};

// Production defaults (or GA-lock when feature enabled)
let caps = default_capabilities();

// Custom build flags
let flags = BuildFlags::production();
let caps = capabilities_for(flags);
```

### Error Handling

```rust
use perl_lsp_protocol::{JsonRpcResponse, JsonRpcError, method_not_found, req_uri};

// Build error responses
let err = method_not_found("textDocument/foo");
let response = JsonRpcResponse::error(Some(id), err);

// Extract params with validation
let uri = req_uri(&params)?; // Returns Result<&str, JsonRpcError>
```

### Method Dispatch

```rust
use perl_lsp_protocol::methods;

match method {
    methods::INITIALIZE => { /* ... */ }
    methods::TEXT_DOCUMENT_HOVER => { /* ... */ }
    methods::SHUTDOWN => { /* ... */ }
    _ => { /* unknown */ }
}
```

## Important Notes

- `BuildFlags::production()` enables all capabilities except formatting (depends on perltidy availability)
- `BuildFlags::ga_lock()` is the conservative set selected by the `lsp-ga-lock` feature flag
- `BuildFlags::all()` enables everything, used in tests
- `jsonrpc` and `errors` modules are re-exported at crate root; `capabilities` and `methods` are public submodules
- See `features.toml` in the workspace root for canonical capability definitions
- Protocol version is tied to the `lsp-types` crate version (currently 0.97 / LSP 3.17)
- Changes to capabilities affect client compatibility
