# perl-lsp

[![codecov](https://codecov.io/gh/EffortlessMetrics/tree-sitter-perl-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/tree-sitter-perl-rs)
[![Crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Rust workspace for Perl parsing, language server (LSP), and debug adapter (DAP) tooling.

## Release Status

- Current workspace release line: `0.9.x`
- Main parser crate: `perl-parser`
- Main editor binary: `perl-lsp`
- Main debugger binary: `perl-dap`

## Published Crates

| Crate | Purpose |
| --- | --- |
| `perl-parser` | Native recursive-descent Perl parser and analysis APIs |
| `perl-lsp` | Language Server Protocol binary for Perl editors |
| `perl-dap` | Debug Adapter Protocol binary for Perl debugging |
| `perl-lexer` | Perl lexer/tokenization library |
| `perl-corpus` | Parser/LSP test corpus and corpus tooling |
| `perl-parser-pest` | Legacy Pest-based parser crate |

## Workspace Layout

- `crates/perl-parser`: parser entry points and high-level APIs
- `crates/perl-lsp`: LSP server binary and integration tests
- `crates/perl-dap`: DAP server binary
- `crates/perl-lsp-*`: focused LSP feature crates (completion, diagnostics, navigation, etc.)
- `crates/perl-*`: parser/semantic/indexing support crates
- `xtask`: development automation and local workflows

Each crate now has a crate-local `README.md` describing its scope and public surface.

## Quick Start

```bash
# Build all workspace crates
cargo build

# Run test suite
cargo test

# Run LSP server
cargo run -p perl-lsp -- --stdio

# Run DAP server
cargo run -p perl-dap
```

## Development

```bash
# Lint
cargo clippy --workspace --all-targets

# Format
cargo fmt --all

# Optional full local gate (when Nix is available)
nix develop -c just ci-gate
```

## Documentation

- Workspace docs index: [`docs/README.md`](docs/README.md)
- LSP architecture: [`docs/LSP_IMPLEMENTATION_GUIDE.md`](docs/LSP_IMPLEMENTATION_GUIDE.md)
- DAP usage: [`docs/DAP_USER_GUIDE.md`](docs/DAP_USER_GUIDE.md)
- Commands reference: [`docs/COMMANDS_REFERENCE.md`](docs/COMMANDS_REFERENCE.md)

## License

Dual licensed under MIT OR Apache-2.0:

- [`LICENSE-MIT`](LICENSE-MIT)
- [`LICENSE-APACHE`](LICENSE-APACHE)
