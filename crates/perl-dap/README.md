# perl-dap

Debug Adapter Protocol (DAP) server for Perl debugging in VS Code and other DAP-compatible editors.

## Overview

The **perl-dap** crate provides a DAP server for Perl debugging. It includes a native adapter used by the CLI (drives `perl -d` directly) and a BridgeAdapter library that can proxy to Perl::LanguageServer.

**Current Status**: Native adapter CLI (launch + attach + stepping + evaluate) with BridgeAdapter library available for compatibility workflows.

## Architecture

```
VS Code
  ↓ (DAP over stdio)
perl-dap (Rust)
  ├─ Native adapter (default CLI) → perl -d
  └─ BridgeAdapter (library) → Perl::LanguageServer DAP → perl -d
```

## Features

### Native Adapter (Default CLI)

- **Launch Debugging**: Start `perl -d` via stdio
- **Attach Debugging**: Attach via TCP (`host`/`port`) or PID signal-control mode (`processId`)
- **Breakpoints + Stepping**: Best-effort breakpoint/step control
- **Stack/Threads**: Best-effort stack frames and single-thread view
- **Variables/Evaluate**: Parsed best-effort values from debugger output with safe evaluation checks
- **Cross-Platform Support**: Windows, macOS, Linux, and WSL path translation

### BridgeAdapter (Library)

- **Bridge Mode**: Proxy DAP messages to Perl::LanguageServer
- **Launch/Attach Configurations**: Shared config types + snippets

### Planned

- Deeper debugger protocol parity (exception/thread controls)
- More deterministic stack/variable extraction from complex frames
- Additional hardening and transcript coverage

## Quick Start

### Installation

```bash
cargo install --path crates/perl-dap
```

### Prerequisites

- Perl 5.20+ available on PATH
- `Perl::LanguageServer` only if you use BridgeAdapter:
  ```bash
  cpanm Perl::LanguageServer
  ```

### VS Code Configuration

Add to your `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Debug Perl Script",
      "program": "${file}",
      "stopOnEntry": true,
      "reloadModules": true
    }
  ]
}
```

## Usage

### Native CLI (Default)

The VS Code extension will launch `perl-dap` automatically. You can also run it directly:

```bash
perl-dap --stdio
```

### BridgeAdapter (Library Only)

```rust
use perl_dap::BridgeAdapter;

let mut adapter = BridgeAdapter::new();
adapter.spawn_pls_dap()?;   // Starts Perl::LanguageServer with DAP flag
adapter.proxy_messages()?;  // Proxies stdin/stdout
```

### Configuration Generation

```rust
use perl_dap::{create_launch_json_snippet, create_attach_json_snippet};

println!("{}", create_launch_json_snippet());
println!("{}", create_attach_json_snippet());
```

## Configuration Options

### Launch Configuration

| Option | Type | Description |
|--------|------|-------------|
| `program` | String | Path to Perl script to debug |
| `args` | String[] | Command-line arguments |
| `cwd` | String | Working directory |
| `env` | Object | Environment variables |
| `perlPath` | String | Path to perl executable |
| `includePaths` | String[] | Additional @INC paths |
| `stopOnEntry` | Boolean | Break at first line |

### Attach Configuration (Native Adapter + BridgeAdapter)

| Option | Type | Description |
|--------|------|-------------|
| `processId` | Number | Attach to local running process by PID (signal-control mode) |
| `port` | Number | Debug port (default: 13603) |
| `host` | String | Host address (default: localhost) |
| `timeout` | Number | TCP attach timeout in milliseconds |

## Testing

```bash
# Run all DAP tests
cargo test -p perl-dap

# Run bridge integration tests
cargo test -p perl-dap --test bridge_integration_tests

# Run with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-dap
```

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | Supported | Native signal handling |
| macOS | Supported | Symlink resolution |
| Windows | Supported | Drive letter normalization, UNC paths |
| WSL | Supported | `/mnt/c` ↔ `C:\` path translation |

## Documentation

- [DAP Implementation Specification](../../docs/DAP_IMPLEMENTATION_SPECIFICATION.md)
- [DAP User Guide](../../docs/DAP_USER_GUIDE.md)
- [DAP Security Specification](../../docs/DAP_SECURITY_SPECIFICATION.md)
- [Crate Architecture](../../docs/CRATE_ARCHITECTURE_DAP.md)

## Related Crates

| Crate | Relationship |
|-------|--------------|
| [perl-parser](../perl-parser/) | AST for breakpoint validation (Phase 2) |
| [perl-lsp](../perl-lsp/) | Optional LSP integration |
| [perl-lexer](../perl-lexer/) | Position mapping support |

## License

MIT OR Apache-2.0

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
