# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap` is the **Debug Adapter Protocol server** providing debugging support for Perl programs.

**Purpose**: Debug Adapter Protocol server for Perl — bridges to perl's built-in debugger with IDE-compatible debugging features.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap               # Build this crate
cargo build -p perl-dap --release     # Build optimized
cargo test -p perl-dap                # Run tests
cargo clippy -p perl-dap              # Lint
cargo doc -p perl-dap --open          # View documentation
```

## Running the Server

```bash
# Run DAP server
./target/release/perl-dap

# With debug logging
RUST_LOG=debug ./target/release/perl-dap
```

## Architecture

### Role in Workspace

This is a **Tier 6 executable crate** — the debug adapter binary for Perl.

### Key Dependencies

**Internal**:
- `perl-parser` - For source analysis

**External**:
- `tokio` (full features) - Async runtime
- `lsp-types` - Shared types with LSP
- `serde`, `serde_json` - Protocol serialization
- `anyhow` - Error handling
- `tracing` - Logging
- `clap` - CLI argument parsing
- `ropey` - Rope text handling
- `regex` - Pattern matching
- Platform-specific: `nix` (Unix), `winapi` (Windows)

### DAP Feature Modules

| Crate | Purpose |
|-------|---------|
| `perl-dap-breakpoint` | Breakpoint validation |
| `perl-dap-eval` | Safe expression evaluation |
| `perl-dap-stack` | Stack trace handling |
| `perl-dap-variables` | Variable rendering |

### Main Modules

| File | Purpose |
|------|---------|
| `main.rs` | Server entry point |
| `lib.rs` | DAP library interface |

## Features

| Feature | Purpose |
|---------|---------|
| `dap-phase1` | Phase 1 debugging features |
| `dap-phase2` | Phase 2 extended features |
| `dap-phase3` | Phase 3 advanced features |

## Test Suites

The crate has extensive acceptance testing:

| Test Suite | Purpose |
|------------|---------|
| `bridge_integration_tests` | Debugger bridge integration |
| `dap_adapter_tests` | DAP adapter protocol |
| `dap_golden_transcript_tests` | Golden file testing |
| `dap_breakpoint_matrix_tests` | Breakpoint behavior matrix |
| `dap_performance_tests` | Performance validation |
| `dap_security_tests` | Security validation |
| `dap_dependency_tests` | Dependency testing |
| `dap_packaging_tests` | Package validation |

Tests cover acceptance criteria AC1-AC19.

## Bridge Mode

The DAP server operates in "bridge mode" — it communicates with Perl's built-in debugger (`perl -d`) via a PTY/pipe interface:

```
IDE <--DAP--> perl-dap <--PTY--> perl -d script.pl
```

## Important Notes

- Currently in phase 1 development
- Platform-specific code for Unix/Windows process handling
- Security validation for expression evaluation (via `perl-dap-eval`)
- See `perl-dap-*` crates for individual feature documentation
