# perl-lsp

`perl-lsp` is a native Rust Perl 5 toolchain centered on editor support. This repository ships the Language Server Protocol server, a separate Debug Adapter Protocol server, embeddable parser and lexer crates, and the corpus, workspace, and status machinery used to harden them together.

> Status: public alpha. APIs, behavior, and packaging are still evolving. See the [Stability Policy](docs/reference/STABILITY.md).

## Why This Repository Exists

Perl developers often end up stitching together separate parser, editor, and debugger stories. This project keeps those pieces in one native workspace:

- `perl-lsp` for editor integration over standard LSP
- `perl-dap` for debugging workflows
- `perl-parser` and `perl-lexer` for native Perl-aware tooling in Rust
- Shared corpus, workspace, semantic, and status tooling so fixes land across the stack instead of in isolated forks

If you want editor features, install `perl-lsp`. If you want to build Perl-aware Rust tooling, start with `perl-parser`.

## What You Get

- Diagnostics, completion, hover, rename, code actions, formatting, and navigation for Perl files
- Cross-file indexing and workspace-aware symbol lookup
- A native parser and lexer stack that does not depend on a Perl runtime for parsing, indexing, or analysis
- A DAP server for debugger integration through `perl-dap`
- One repository containing the shipped binaries, parser libraries, VS Code extension, docs, CI gates, and test corpus

Live capability and health tracking lives in:

- [Current Status](docs/project/CURRENT_STATUS.md)
- [Roadmap](docs/project/ROADMAP.md)
- [`features.toml`](features.toml)

## Quick Start

Choose the path that matches what you are trying to do:

- VS Code: install the extension and let it find a `PATH` binary, use `perl-lsp.serverPath`, or auto-download at runtime with `perl-lsp.autoDownload`
- Other editors: install `perl-lsp` and point your client at `perl-lsp --stdio`
- Debugging: install or build `perl-dap` separately; it is not installed by the `perl-lsp` installer
- Rust tooling: start with `perl-parser` if you want to embed parser or lexer behavior

### Install the LSP server

```bash
cargo install perl-lsp
perl-lsp --health
```

Build from source:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health
```

Best-effort installer script for Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

### Install the VS Code extension

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

The extension can use `perl-lsp` from `PATH`, a configured `perl-lsp.serverPath`, or automatic runtime download when `perl-lsp.autoDownload` is enabled.

### Useful CLI checks

```bash
perl-lsp --version
perl-lsp --health
perl-lsp --info
perl-lsp --stdio
perl-lsp --check script.pl
```

For a walkthrough from install to first editor connection, see [Getting Started](docs/tutorials/GETTING_STARTED.md).

## Editor Setup

`perl-lsp` speaks standard JSON-RPC over stdio, so it works with editors that support LSP. This repository includes setup guidance for VS Code, Neovim, Emacs, Helix, and other editor clients.

Start here:

- [Getting Started](docs/tutorials/GETTING_STARTED.md)
- [Editor Setup](docs/how-to/EDITOR_SETUP.md)
- [Extension Guide](docs/EXTENSION.md)
- [VS Code Extension README](vscode-extension/README.md)

## Repository Overview

This workspace is intentionally split into focused crates. The main entry points are:

| Path | Purpose |
| --- | --- |
| [`crates/perl-lsp`](crates/perl-lsp/) | LSP binary and server host |
| [`crates/perl-dap`](crates/perl-dap/) | DAP server and debugger integration |
| [`crates/perl-parser`](crates/perl-parser/) | Native recursive-descent Perl parser |
| [`crates/perl-lexer`](crates/perl-lexer/) | Context-aware tokenizer |
| [`crates/perl-parser-core`](crates/perl-parser-core/) | Shared parser infrastructure |
| [`crates/perl-semantic-analyzer`](crates/perl-semantic-analyzer/) | Semantic analysis and resolution |
| [`crates/perl-corpus`](crates/perl-corpus/) | Corpus and regression fixtures |

## Contributor Devex

Recommended contributor flow:

1. Check the environment with `just devex` or `just doctor`.
2. Iterate with `just pr-fast`.
3. Run `nix develop -c just ci-gate` before push.
4. Install the pre-push hook with `bash scripts/install-githooks.sh` if you want the gate wired automatically.

Common local commands:

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

Useful fast-feedback commands:

```bash
just devex
just doctor
just pr-fast
just status-check
```

For `perl-lsp` integration-heavy tests in constrained environments:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

If your change affects generated status or capability docs, run:

```bash
just status-update
just status-check
```

See:

- [Contributing Guide](CONTRIBUTING.md)
- [Commands Reference](docs/reference/COMMANDS_REFERENCE.md)
- [Extension Guide](docs/EXTENSION.md)
- [Documentation Index](docs/README.md)

## Documentation

Good starting points:

| Resource | Purpose |
| --- | --- |
| [Getting Started](docs/tutorials/GETTING_STARTED.md) | Install, configure an editor, and verify the server |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Set up and use the debug adapter |
| [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | Understand the server architecture |
| [Crate Architecture Guide](docs/reference/CRATE_ARCHITECTURE_GUIDE.md) | Navigate the workspace structure |
| [Current Status](docs/project/CURRENT_STATUS.md) | Evidence-backed project metrics and receipts |
| [Roadmap](docs/project/ROADMAP.md) | Active milestone and forward plan |
| [Stability Policy](docs/reference/STABILITY.md) | Alpha status, versioning, and support expectations |

## License

Dual licensed under:

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
