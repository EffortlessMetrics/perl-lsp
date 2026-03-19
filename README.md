# perl-lsp

`perl-lsp` is a native Rust language server for Perl 5. This repository also contains the parser, lexer, debug adapter, and supporting crates that power it.

> Status: public alpha. APIs, behavior, and packaging are still evolving. See the [Stability Policy](docs/reference/STABILITY.md).

## What This Repository Contains

This workspace is split into many focused crates. The most important entry points are:

| Crate | Purpose |
| --- | --- |
| [`crates/perl-lsp`](crates/perl-lsp/) | Language Server Protocol binary for editor integration |
| [`crates/perl-dap`](crates/perl-dap/) | Debug Adapter Protocol server for Perl debugging workflows |
| [`crates/perl-parser`](crates/perl-parser/) | Native recursive-descent Perl parser library |
| [`crates/perl-lexer`](crates/perl-lexer/) | Context-aware tokenizer used by the parser and editor tooling |
| [`crates/perl-corpus`](crates/perl-corpus/) | Corpus and fixture support for parser and LSP hardening |

If you want IDE features in an editor, install `perl-lsp`. If you want parsing and syntax tooling in your own Rust code, start with `perl-parser`.

## Quick Start

### Install the LSP server

```bash
cargo install perl-lsp
perl-lsp --health
```

You can also build from source:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health
```

There is also a best-effort installer script for Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

### Install the VS Code extension

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

### Run the server directly

```bash
perl-lsp --stdio
```

Useful CLI checks:

```bash
perl-lsp --version
perl-lsp --health
perl-lsp --info
perl-lsp --check script.pl
```

For a full walkthrough, see [Getting Started](docs/tutorials/GETTING_STARTED.md).

## Editor Setup

`perl-lsp` speaks standard JSON-RPC over stdio, so it works with editors that support LSP.

The repository includes editor setup guidance for VS Code, Neovim, Emacs, Helix, and other LSP-capable editors. For complete configuration examples, see:

- [Getting Started](docs/tutorials/GETTING_STARTED.md)
- [Editor Setup](docs/how-to/EDITOR_SETUP.md)

## What You Get

The repository is broader than a single server binary, but the main user-facing capabilities are:

- Real-time diagnostics for Perl files
- Completion for symbols, modules, variables, keywords, and builtins
- Hover information and signature help for common Perl constructs
- Navigation features such as go-to-definition, references, and symbols
- Rename, code actions, and formatting support
- Cross-file workspace indexing
- Debug Adapter Protocol support through `perl-dap`
- Native parser and lexer libraries for Perl-aware tooling

For a deeper feature breakdown, see:

- [LSP Features](docs/reference/LSP_FEATURES.md)
- [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)
- [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md)

## Parser and Tooling

The parser stack in this repository is implemented natively in Rust. The current parser is a recursive-descent implementation backed by a context-aware lexer and exercised against curated corpus, fixture, and real-world test suites.

If you are working on parser behavior directly, the most relevant crates are:

- [`crates/perl-parser`](crates/perl-parser/)
- [`crates/perl-parser-core`](crates/perl-parser-core/)
- [`crates/perl-lexer`](crates/perl-lexer/)
- [`crates/perl-semantic-analyzer`](crates/perl-semantic-analyzer/)
- [`crates/perl-corpus`](crates/perl-corpus/)

Project health and generated metrics live in [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Development

Common commands:

```bash
cargo build -p perl-lsp --release
cargo build -p perl-parser --release

cargo test --workspace --lib
cargo test -p perl-parser
cargo test -p perl-lsp

cargo fmt --all
cargo clippy --workspace --lib
```

Canonical local validation before push:

```bash
nix develop -c just ci-gate
```

Helpful additional commands:

```bash
just doctor
just pr-fast
just ci-lsp-def
```

See:

- [Contributing Guide](CONTRIBUTING.md)
- [Commands Reference](docs/reference/COMMANDS_REFERENCE.md)
- [Documentation Index](docs/README.md)

## Documentation

Good starting points:

| Resource | Purpose |
| --- | --- |
| [Getting Started](docs/tutorials/GETTING_STARTED.md) | Install, configure an editor, and verify the server |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Set up and use the debug adapter |
| [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | Understand the server architecture |
| [Crate Architecture Guide](docs/reference/CRATE_ARCHITECTURE_GUIDE.md) | Navigate the workspace structure |
| [Current Status](docs/project/CURRENT_STATUS.md) | Generated project metrics and health snapshots |
| [Stability Policy](docs/reference/STABILITY.md) | Alpha status, versioning, and support expectations |

## License

Dual licensed under:

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
