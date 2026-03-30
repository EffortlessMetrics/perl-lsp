<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  Native Perl 5 language tooling in Rust: editor support, parser infrastructure, and debugging without a Perl runtime dependency for IDE features.
</p>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/EffortlessMetrics/perl-lsp/releases"><img src="https://img.shields.io/github/v/release/EffortlessMetrics/perl-lsp?display_name=tag" alt="GitHub release" /></a>
  <a href="docs/README.md"><img src="https://img.shields.io/badge/docs-project%20docs-blue.svg" alt="Project docs" /></a>
  <a href="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master"><img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="codecov" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.92%2B-orange.svg" alt="Rust" /></a>
</p>

---

Perl editor support too often starts with "make Perl itself work first, then add the editor layer later." `perl-lsp` flips that around. Install one native binary, `perllsp`, and get completions, diagnostics, navigation, formatting, and debugging for Perl 5 on Windows, macOS, and Linux.

## Start Here

| If you want to... | Start here |
| --- | --- |
| Get IDE support quickly | [Quick Start](#quick-start) |
| Set up a specific editor | [docs/how-to/EDITOR_SETUP.md](docs/how-to/EDITOR_SETUP.md) |
| Upgrade an existing install | [docs/how-to/UPGRADING.md](docs/how-to/UPGRADING.md) |
| Troubleshoot a broken setup | [docs/how-to/TROUBLESHOOTING.md](docs/how-to/TROUBLESHOOTING.md) |
| See what is true right now | [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) |
| See the current release plan | [docs/project/ROADMAP.md](docs/project/ROADMAP.md) |
| Use the Rust crates directly | [`crates/perllsp`](crates/perllsp/), [`crates/perl-parser`](crates/perl-parser/), [`crates/perl-dap`](crates/perl-dap/) |

## Why Teams Pick It

- Native editor tooling: no Perl runtime dependency just to get LSP or DAP features into your editor.
- One workspace, multiple entry points: install the binaries or depend on the parser, semantic, workspace, and protocol crates directly.
- Real Perl coverage: parser and runtime changes are validated against curated corpus and release receipts, not only toy examples.
- Windows included: install, path handling, packaging, and shell interactions are part of the release surface rather than an afterthought.

## What You Get

| Surface | What it covers |
| --- | --- |
| Language server | Diagnostics, completion, hover, navigation, rename, formatting, semantic tokens, code actions, code lens, workspace symbols |
| Debug adapter | Breakpoints, stepping, stack frames, variables, evaluate support, and editor-driven DAP flows |
| Parser stack | Native recursive-descent parser, lexer, semantic analysis, workspace indexing, and refactoring helpers |
| Rust crates | Focused crates for parser, URI/path handling, LSP/DAP protocol layers, workspace indexing, and feature providers |

## Quick Start

### VS Code

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

The VS Code extension auto-downloads the matching `perllsp` binary for your platform.

### GitHub release binary

Download a prebuilt binary from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases), add it to your `PATH`, and then verify it:

```bash
perllsp --health
```

If you are testing the workspace locally before a tagged release, install from source instead:

```bash
cargo install --path crates/perllsp
```

Do not use `cargo install perl-lsp` on crates.io. That package name is owned by another project.

### Other editors

Neovim:

```lua
require('lspconfig').perl_ls.setup {
  cmd = { "perllsp", "--stdio" },
}
```

Emacs (`eglot`):

```elisp
(add-to-list 'eglot-server-programs '(perl-mode "perllsp" "--stdio"))
```

Generic LSP client:

```text
perllsp --stdio
```

For a full setup path, use [docs/tutorials/GETTING_STARTED.md](docs/tutorials/GETTING_STARTED.md).

## Configuration

The defaults are meant to be usable without project-specific setup. When you do need configuration, you can use editor LSP settings or a repo-level `.perl-lsp.toml`.

Editor settings example:

```jsonc
{
  "perl-lsp.inlayHints": {
    "enabled": true,
    "parameterHints": true,
    "typeHints": true
  },
  "perl-lsp.workspace": {
    "includePaths": ["lib", ".", "local/lib/perl5"],
    "useSystemInc": false
  }
}
```

Project config example:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
perlcritic = false

[features]
inlay_hints = true
```

Use [docs/reference/CONFIG.md](docs/reference/CONFIG.md) for the full reference and precedence rules.

## Workspace Entry Points

| Crate | Use it when you need... |
| --- | --- |
| [`crates/perllsp`](crates/perllsp/) | the public Cargo package for installing the `perllsp` binary |
| [`crates/perl-lsp`](crates/perl-lsp/) | the internal implementation crate behind the `perllsp` binary |
| [`crates/perl-dap`](crates/perl-dap/) | the native debug adapter runtime |
| [`crates/perl-parser`](crates/perl-parser/) | one facade for parsing, semantic analysis, and workspace tooling |
| [`crates/perl-lexer`](crates/perl-lexer/) | tokenization and lexical state handling |
| [`crates/perl-semantic-analyzer`](crates/perl-semantic-analyzer/) | symbol, scope, and type analysis over parsed trees |
| [`crates/perl-workspace-index`](crates/perl-workspace-index/) | document storage, indexing, and cross-file lookups |

Published library crates are documented on docs.rs. Internal and supporting docs start at [docs/README.md](docs/README.md).

## Docs By Job

- New user: [docs/tutorials/GETTING_STARTED.md](docs/tutorials/GETTING_STARTED.md)
- Installation and editor setup: [docs/how-to/INSTALLATION.md](docs/how-to/INSTALLATION.md), [docs/how-to/EDITOR_SETUP.md](docs/how-to/EDITOR_SETUP.md)
- Upgrade and troubleshooting: [docs/how-to/UPGRADING.md](docs/how-to/UPGRADING.md), [docs/how-to/TROUBLESHOOTING.md](docs/how-to/TROUBLESHOOTING.md)
- Commands and configuration: [docs/reference/COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md), [docs/reference/CONFIG.md](docs/reference/CONFIG.md)
- Project truth and planning: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Full docs map: [docs/INDEX.md](docs/INDEX.md)
- Published release tracking: [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Contributing

```bash
cargo build --workspace
cargo test --workspace --lib
cargo fmt --all
nix develop -c just ci-gate
```

Use [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow and [docs/reference/COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) for the broader command surface.

## Security

Release artifacts include SBOM generation and provenance attestations. Production code is kept under the workspace ratchets for `unsafe`, `unwrap` / `expect`, and panic-family macros. See [docs/reference/SUPPLY_CHAIN_SECURITY.md](docs/reference/SUPPLY_CHAIN_SECURITY.md).

## History

This project started from the `tree-sitter-perl` line and grew into a native Rust Perl tooling workspace with its own parser, LSP, DAP, and release stack.

## License

Dual licensed under MIT or Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
