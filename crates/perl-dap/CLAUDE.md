# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Tier**: 6 (application/executable crate)
- **Purpose**: Debug Adapter Protocol server for Perl. Provides a native adapter that drives `perl -d` directly, and a `BridgeAdapter` library that proxies DAP messages to Perl::LanguageServer.
- **Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap               # Build
cargo build -p perl-dap --release     # Build optimized
cargo test -p perl-dap                # Run tests
cargo clippy -p perl-dap              # Lint
cargo doc -p perl-dap --open          # View docs
./target/release/perl-dap --stdio     # Run native adapter (stdio)
./target/release/perl-dap --socket --port 13603  # Run native adapter (TCP)
./target/release/perl-dap --bridge    # Run bridge adapter
RUST_LOG=debug ./target/release/perl-dap  # Run with debug logging
```

## Architecture

### Dependencies

**Internal crates**:
- `perl-parser` -- AST for breakpoint validation
- `perl-dap-breakpoint` -- `AstBreakpointValidator`, `BreakpointValidator` trait
- `perl-dap-eval` -- `SafeEvaluator` for expression evaluation
- `perl-dap-stack` -- `PerlStackParser` for stack trace extraction
- `perl-dap-variables` -- `PerlVariableRenderer`, `VariableParser`, `VariableRenderer`

**External crates**: `tokio` (async runtime), `lsp-types` (shared types with LSP), `serde`/`serde_json` (protocol serialization), `anyhow`/`thiserror` (errors), `clap` (CLI), `tracing` (logging), `regex` (debugger output parsing), `ropey` (position mapping), `nix` (Unix signals), `winapi` (Windows process control)

### Key Types and Modules

| Module | Key types | Purpose |
|--------|-----------|---------|
| `lib.rs` | `DapServer`, `DapConfig`, `DapMode` | Server entry point; dispatches to Native or Bridge mode |
| `main.rs` | `Args` (clap) | CLI binary; parses `--stdio`, `--socket`, `--bridge`, `--port`, `--log-level` |
| `debug_adapter.rs` | `DebugAdapter`, `DapMessage` | Native adapter: manages `perl -d` process, handles all DAP requests |
| `bridge_adapter.rs` | `BridgeAdapter` | Spawns Perl::LanguageServer in DAP mode, proxies messages via stdio |
| `protocol.rs` | `Request`, `Response`, `Event`, `Capabilities`, `SourceBreakpoint`, `Breakpoint`, ... | Full DAP protocol type definitions (serde-annotated) |
| `breakpoints.rs` | `BreakpointStore`, `BreakpointRecord`, `BreakpointHitOutcome` | Breakpoint storage with REPLACE semantics, AST validation |
| `dispatcher.rs` | `DapDispatcher` (deprecated), `DispatchResult` | Legacy message router; use `DebugAdapter` instead |
| `configuration.rs` | `LaunchConfiguration`, `AttachConfiguration`, `create_launch_json_snippet()`, `create_attach_json_snippet()` | Launch/attach config structs with validation |
| `platform.rs` | `resolve_perl_path()`, `normalize_path()`, `setup_environment()` | Cross-platform path resolution and env setup |
| `security.rs` | `SecurityError`, `validate_path()`, `validate_expression()` | Path traversal prevention, expression sanitization, timeout caps |
| `tcp_attach.rs` | `TcpAttachConfig`, `TcpAttachSession`, `DapEvent` | TCP socket attachment to running Perl debuggers |
| `inline_values.rs` | `collect_inline_values()` | Regex-based inline value extraction for scalar variables |
| `feature_catalog.rs` | `has_feature()`, `advertised_features()` | Auto-generated from `features.toml` at build time |

### Feature Flags

| Feature | Purpose |
|---------|---------|
| `dap-phase1` | Phase 1: bridge to Perl::LanguageServer (AC1-AC4) |
| `dap-phase2` | Phase 2: native adapter features (AC5-AC16) |
| `dap-phase3` | Phase 3: production hardening (AC17-AC19) |

## Usage Examples

```rust
// Native mode (default)
use perl_dap::{DapConfig, DapMode, DapServer};
let config = DapConfig { log_level: "info".into(), mode: DapMode::Native, workspace_root: None };
let mut server = DapServer::new(config)?;
server.run()?; // stdio transport

// Bridge mode
use perl_dap::BridgeAdapter;
let mut adapter = BridgeAdapter::new();
adapter.spawn_pls_dap().await?;
adapter.proxy_messages().await?;
adapter.shutdown().await?;

// Configuration generation
use perl_dap::{create_launch_json_snippet, create_attach_json_snippet};
println!("{}", create_launch_json_snippet());
```

## Important Notes

- `DapDispatcher` is deprecated; use `DebugAdapter` directly for new code
- Platform-specific code gated with `cfg(unix)` / `cfg(windows)` for signal handling
- Security module enforces workspace-boundary path checks and expression sanitization
- All regex patterns use `OnceLock<Result<Regex, regex::Error>>` or `Lazy<Option<Regex>>` for graceful degradation
- Build script generates `dap_feature_catalog.rs` from `features.toml`
- Test suites cover acceptance criteria AC1-AC19 across 10 test targets
