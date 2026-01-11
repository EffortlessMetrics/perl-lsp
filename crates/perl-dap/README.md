# perl-dap

Debug Adapter Protocol (DAP) server for Perl debugging in VS Code and other DAP-compatible editors.

## Overview

The **perl-dap** crate provides production-grade debugging capabilities for the Perl LSP ecosystem. It implements a bridge between VS Code's debugger client and Perl's runtime debugging facilities.

**Current Status**: Phase 1 Complete (Bridge Implementation)

## Architecture

```
VS Code
  ↓ (DAP over stdio)
perl-dap (Rust)
  ├─ Phase 1: BridgeAdapter → Perl::LanguageServer  ✅ Complete
  ├─ Phase 2: Native DAP server (planned)
  └─ Phase 3: Security hardening (planned)
  ↓
Perl Debugger Runtime (perl -d)
```

## Features

### Phase 1 (Current)

- **Bridge Mode**: Proxies DAP messages between VS Code and Perl::LanguageServer
- **Launch/Attach Configurations**: Full DAP configuration support with validation
- **Cross-Platform Support**: Windows, macOS, Linux, and WSL path translation
- **VS Code Integration**: Generates `launch.json` snippets

### Planned (Phase 2/3)

- Native Rust DAP protocol server
- AST-based breakpoint validation
- Safe expression evaluation
- Performance optimization (<50ms targets)

## Quick Start

### Installation

```bash
cargo install --path crates/perl-dap
```

### Prerequisites

- Perl 5.20+ with `Perl::LanguageServer` installed:
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

### Bridge Mode (Phase 1)

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
| `perlBinary` | String | Path to perl executable |
| `includePaths` | String[] | Additional @INC paths |
| `stopOnEntry` | Boolean | Break at first line |

### Attach Configuration

| Option | Type | Description |
|--------|------|-------------|
| `port` | Number | Debug port (default: 13603) |
| `host` | String | Host address (default: localhost) |

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
