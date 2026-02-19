# CLAUDE.md

## Crate Overview

- **Name**: `perl-lsp`
- **Version**: 0.9.0
- **Tier**: 6 (executable / application crate)
- **Purpose**: Standalone LSP server binary and library for Perl. Provides the `perl-lsp` binary (stdio and TCP modes) and a public library API (`LspServer`, `run_stdio()`, JSON-RPC types). Delegates parsing to `perl-parser` and dispatches LSP features through dedicated provider crates.

## Commands

```bash
cargo build -p perl-lsp                   # Build (debug)
cargo build -p perl-lsp --release         # Build (release)
cargo install --path crates/perl-lsp      # Install binary
cargo test -p perl-lsp                    # Run tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2  # Tests with threading limits
cargo clippy -p perl-lsp                  # Lint
cargo doc -p perl-lsp --open              # View docs

# Run the server
./target/release/perl-lsp --stdio         # stdio mode (editor integration)
./target/release/perl-lsp --socket --port 9257  # TCP mode
./target/release/perl-lsp --health        # Health check
./target/release/perl-lsp --version       # Version info
./target/release/perl-lsp --features-json # Feature catalog as JSON
```

## Architecture

### Dependencies

**Workspace crates** (core):
- `perl-parser` (with `test-performance` feature) -- parsing engine
- `perl-lsp-providers` (with `lsp-compat`) -- provider aggregation
- `perl-lsp-protocol` -- JSON-RPC / LSP message types
- `perl-lsp-transport` -- message framing (stdio, TCP)
- `perl-lsp-formatting` -- perltidy integration
- `perl-lsp-tooling` -- external tool support

**Workspace crates** (feature providers):
- `perl-lsp-completion`, `perl-lsp-navigation`, `perl-lsp-diagnostics`
- `perl-lsp-code-actions`, `perl-lsp-rename`, `perl-lsp-semantic-tokens`, `perl-lsp-inlay-hints`

**External**: `lsp-types`, `serde`, `serde_json`, `tokio`, `ropey`, `parking_lot`, `anyhow`, `walkdir`, `regex`, `url`, `md5`, `once_cell`

### Key Types and Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | Binary entry point: arg parsing, stdio/TCP dispatch |
| `lib.rs` | Library root: public re-exports (`LspServer`, `run_stdio`, JSON-RPC types), crate-internal compatibility re-exports |
| `server.rs` | Re-exports `LspServer` from `runtime` |
| `runtime/` | Server lifecycle, message routing (`routing.rs`), text sync (`text_sync.rs`), diagnostics, file discovery, window messages |
| `state/` | Server state: `config.rs` (settings), `document.rs` (open documents), `limits.rs` (resource budgets) |
| `dispatch.rs` | Request/notification routing to handlers |
| `handlers/` | Request and notification handler implementations |
| `features/` | All LSP capability implementations: completion, diagnostics, hover, references, rename, formatting, semantic tokens, code actions, code lens, inlay hints, folding, selection range, signature help, type hierarchy, document links, linked editing |
| `protocol/` | JSON-RPC message types (`JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`) |
| `transport/` | Transport layer integration |
| `convert/` | Conversions between `perl_parser` types and `lsp_types` |
| `util/uri.rs` | URI conversion (contains the sole `#[allow(clippy::expect_used)]`) |
| `fallback/` | Text-based fallback when parsing fails |
| `cancellation.rs` | Request cancellation infrastructure |
| `security/` | Sandbox and input validation |
| `diagnostics_catalog.rs` | Stable diagnostic codes |
| `execute_command.rs` | LSP command execution |
| `call_hierarchy_provider.rs` | Call hierarchy support |

### Features (Cargo)

| Feature | Default | Description |
|---------|---------|-------------|
| `workspace` | yes | Workspace-wide indexing for cross-file features |
| `incremental` | no | Incremental parsing (`perl-parser/incremental`) |
| `dap-phase1` | no | Debug Adapter Protocol bridge (`perl-dap`) |
| `lsp-ga-lock` | no | Lock to conservative GA capability set |
| `experimental-features` | no | Experimental LSP features |

## Usage Examples

```rust
// Library: run LSP server on stdio
perl_lsp::run_stdio().unwrap();

// Library: create server with custom output
use perl_lsp::LspServer;
let mut server = LspServer::new();
server.run().unwrap();
```

```bash
# Binary: editor integration
perl-lsp --stdio

# Binary: TCP for remote debugging
perl-lsp --socket --port 9257
```

## Important Notes

- This is the user-facing binary -- stability is critical.
- `util/uri.rs` has the single allowed `#[allow(clippy::expect_used)]` for `lsp_types::Uri` fallback.
- Tests should use threading constraints (`RUST_TEST_THREADS=2`) to avoid resource exhaustion.
- The `features.toml` in the workspace root is the canonical source for LSP capability definitions.
- The crate re-exports many `perl_parser` modules internally (`pub(crate)`) for migration compatibility; these are not part of the public API.
