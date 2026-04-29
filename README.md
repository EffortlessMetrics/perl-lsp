<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://crates.io/crates/perl-lsp-rs"><img src="https://img.shields.io/crates/v/perl-lsp-rs.svg" alt="crates.io" /></a>
  <a href="https://crates.io/crates/perl-lsp-rs"><img src="https://img.shields.io/crates/d/perl-lsp-rs.svg" alt="Downloads" /></a>
  <a href="https://docs.rs/perl-lsp-rs"><img src="https://docs.rs/perl-lsp-rs/badge.svg" alt="docs.rs" /></a>
  <a href="https://github.com/EffortlessMetrics/perl-lsp/releases"><img src="https://img.shields.io/github/v/release/EffortlessMetrics/perl-lsp?display_name=tag" alt="GitHub release" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/MSRV-1.92-blue" alt="MSRV" /></a>
  <a href="https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"><img src="https://img.shields.io/badge/VS%20Marketplace-180%20installs-0078D4" alt="VSCode Marketplace" /></a>
  <a href="https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs"><img src="https://img.shields.io/open-vsx/dt/EffortlessMetrics/perl-lsp-rs" alt="Open VSX" /></a>
</p>

---

A native Rust LSP server and debug adapter for Perl 5. Fast completions, reliable navigation, and full debugger integration — no Perl runtime required for IDE features.

## Install

**VS Code** — install from the marketplace and you're done:

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

The extension auto-downloads the matching `perllsp` binary for your platform.

**Other editors** — download a prebuilt binary from [Releases](https://github.com/EffortlessMetrics/perl-lsp/releases), add it to `PATH`, then point your LSP client at it:

```lua
-- Neovim (nvim-lspconfig)
local capabilities = vim.lsp.protocol.make_client_capabilities()
local ok_cmp, cmp_lsp = pcall(require, "cmp_nvim_lsp")
if ok_cmp then
  capabilities = cmp_lsp.default_capabilities(capabilities)
end

require("lspconfig").perl_lsp.setup({
  cmd = { "perllsp", "--stdio" },
  capabilities = capabilities,
})
```

```elisp
;; Emacs (eglot) — perl-ts-mode is a third-party package, omit if not installed
(add-to-list 'eglot-server-programs
             '((perl-mode cperl-mode perl-ts-mode) . ("perllsp" "--stdio")))
```

See [Editor Setup](docs/how-to/EDITOR_SETUP.md) for Zed, Helix, and other editors.

Verify the install:

```bash
perllsp --health
```

> **Note:** Use `cargo install --path crates/perllsp` to build from source. Do not use `cargo install perl-lsp` — that name is an unrelated project on crates.io.

For a full walkthrough, see [Getting Started](docs/tutorials/GETTING_STARTED.md).

## Features

- **Complete LSP surface** — completions, diagnostics, hover, go-to-definition, find references, rename, formatting, semantic tokens, inlay hints, code actions, code lens, workspace symbols (88 LSP + 24 DAP + 7 extension capabilities)
- **Native debug adapter** — DAP breakpoints, stepping, stack frames, variable inspection, evaluate; no wrapper script
- **Semantic analysis** — symbol resolution, scope tracking, Moose/Moo method modifiers and role composition
- **Refactoring** — extract variable, extract subroutine, workspace-scoped rename, subroutine inlining
- **Diagnostics** — dead code, strict/warnings, perlcritic with walk-up discovery
- **Fast native parser** — recursive-descent v3 parser validated against a curated CPAN corpus
- **Windows, macOS, Linux** — first-class on all platforms

## Documentation

| | |
|---|---|
| First-time setup | [Getting Started](docs/tutorials/GETTING_STARTED.md) |
| Editor-specific config | [Editor Setup](docs/how-to/EDITOR_SETUP.md) |
| All configuration options | [Config Reference](docs/reference/CONFIG.md) |
| Troubleshooting | [Troubleshooting](docs/how-to/TROUBLESHOOTING.md) |
| Upgrading | [Upgrade Guide](docs/how-to/UPGRADING.md) |
| Commands reference | [Commands Reference](docs/reference/COMMANDS_REFERENCE.md) |
| Status and metrics | [docs/project/status/index.md](docs/project/status/index.md) |
| Roadmap | [ROADMAP.md](docs/project/ROADMAP.md) |
| Release history | [RELEASE_HISTORY.md](RELEASE_HISTORY.md) |
| Full docs index | [docs/INDEX.md](docs/INDEX.md) |

## Using as a Library

| Use case | Crate |
|---|---|
| Parse Perl from Rust | [`perl-parser`](crates/perl-parser) + [`perl-lexer`](crates/perl-lexer) |
| Tokenize only | [`perl-lexer`](crates/perl-lexer) |
| Symbol resolution and scope tracking | [`perl-semantic-analyzer`](crates/perl-semantic-analyzer) |
| Cross-file symbol index | [`perl-workspace-index`](crates/perl-workspace-index) |
| Tree-sitter consumers (Neovim, Helix, GitHub) | [`tree-sitter-perl-c`](crates/tree-sitter-perl-c) |
| Regex safety and complexity analysis | [`perl-regex`](crates/perl-regex) |
| Debug adapter | [`perl-dap`](crates/perl-dap) |

## Status

**v0.12.4** — public alpha. See [status](docs/project/status/index.md) for live metrics and [roadmap](docs/project/ROADMAP.md) for the v0.13.0 milestone.

## Contributing

```bash
cargo test --workspace --lib
cargo xtask fmt
cargo clippy --workspace
cargo xtask semantic-scorecard   # semantic fixture baseline (compiler-lite harness)
nix develop -c just ci-gate   # required before merge
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow. AI implementation agents: read [AGENTS.md](AGENTS.md) first.

## Security

Release artifacts include SBOM generation and provenance attestations. See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md).

## License

Dual licensed under MIT or Apache-2.0: [LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE)
