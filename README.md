<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/EffortlessMetrics/perl-lsp/releases"><img src="https://img.shields.io/github/v/release/EffortlessMetrics/perl-lsp?display_name=tag" alt="GitHub release" /></a>
  <a href="https://docs.rs/perl-lsp-rs"><img src="https://docs.rs/perl-lsp-rs/badge.svg" alt="docs.rs" /></a>
</p>

<p align="center">
  <a href="https://crates.io/crates/perl-lsp-rs"><img src="https://img.shields.io/crates/d/perl-lsp-rs.svg?label=crates.io%20downloads" alt="crates.io downloads" /></a>
  <a href="https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"><img src="https://img.shields.io/badge/VS%20Marketplace-277%20installs-0078D4" alt="VS Marketplace installs" /></a>
  <a href="https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs"><img src="https://img.shields.io/open-vsx/dt/EffortlessMetrics/perl-lsp-rs?label=Open%20VSX%20downloads" alt="Open VSX downloads" /></a>
</p>

<p align="center">
  <a href="https://codecov.io/gh/EffortlessMetrics/perl-lsp"><img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="code coverage" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/MSRV-1.92-blue" alt="MSRV" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
</p>

---

`perl-lsp` is a native Rust language server, parser stack, and debug adapter for Perl 5.

## The problem

Perl has decades of real production code, but editor tooling still struggles with the parts that matter in daily work: incomplete code while typing, cross-file navigation, package and module resolution, diagnostics, refactoring, and debugger integration.

`perl-lsp` is built around a native parser, semantic analysis layer, workspace index, LSP server, and DAP implementation designed for real editor use.

## Status at a glance

These are behavioral and corpus-backed signals, not feature-inventory counts. Protocol coverage and full capability catalogs live in the generated status docs.

| Area | Current signal |
|---|---:|
| Release track | `v0.13.3` public-alpha patch |
| Published crate surface | 31 crates in `[workspace.metadata.publish.allow]` |
| Ubuntu system Perl corpus | 94.5% clean (`2825/2990`) |
| CPAN top 1000 corpus | 95.3% clean (`8931/9372`) |
| Project parser corpus | 100.0% clean (`95/95`) |
| Parser NodeKind coverage | 65/69 |
| Parser reliability | 0 project-corpus timeouts / 0 panics |
| Editor UX scenarios | 23 scenario files tracked |
| First-five-minutes UX workflows | 21 workflows tracked |
| Issue-regression UX workflows | 13 workflows tracked |
| Workspace stale-index defects | 0 / 7 tested scenarios |
| Multi-root workspace tests | 8 / 8 |

See [project status](docs/project/status/index.md), [parser status](docs/project/status/parser.md), [workspace status](docs/project/status/workspace.md), and [quality metrics](docs/project/status/quality.md) for generated details.

## What works

- **Editor workflows**: completion, diagnostics, hover, go-to-definition, references, rename, formatting, semantic tokens, inlay hints, code actions, code lens, and workspace symbols.
- **Parser stack**: native lexer, parser-core, parser facade, corpus ratchets, and tree-sitter integration.
- **UX testing**: 23 tracked editor UX scenarios, including first-five-minutes flows, issue-regression guards, cross-file navigation, diagnostics-after-edit, workspace churn, and rename.
- **Workspace intelligence**: module resolution, symbol indexing, stale-index guards, multi-root workspaces, and workspace-aware rename.
- **Debug adapter**: breakpoints, stepping, stack frames, variables, evaluate, and launch/attach flows.
- **Editor support**: VS Code, Open VSX, Neovim, Vim, Emacs, Helix, Zed, Sublime, and any editor with LSP support.

## Install

- Install: [docs/how-to/INSTALLATION.md](docs/how-to/INSTALLATION.md)
- Editor setup: [docs/how-to/EDITOR_SETUP.md](docs/how-to/EDITOR_SETUP.md)

Current public install artifacts are public alpha. Verify the binary before
wiring it into shared editor or CI configuration.

The VS Code extension downloads the matching `perllsp` binary automatically. Other editors use the `perllsp --stdio` server command after installing a release binary.

Do not install `perl-lsp` from crates.io; that is a different project.

## Crate surface

The v0.13 architecture collapsed the old microcrate graph into a smaller published surface. Most implementation detail now lives in modules behind focused public crates.

| Need | Crate |
|---|---|
| Binary language server | `perllsp` |
| LSP library facade | `perl-lsp-rs` |
| LSP implementation core | `perl-lsp-rs-core` |
| Parser facade | `perl-parser` |
| Parser engine | `perl-parser-core` |
| Lexer | `perl-lexer` |
| Semantic analysis | `perl-semantic-analyzer` |
| Workspace index | `perl-workspace-index` |
| Diagnostics catalog | `perl-diagnostics` |
| Debug adapter | `perl-dap` |
| Tree-sitter integration | `tree-sitter-perl-rs`, `tree-sitter-perl-c` |

## Documentation

| Task | Link |
|---|---|
| Install | [docs/how-to/INSTALLATION.md](docs/how-to/INSTALLATION.md) |
| Editor setup | [docs/how-to/EDITOR_SETUP.md](docs/how-to/EDITOR_SETUP.md) |
| Getting started | [docs/tutorials/GETTING_STARTED.md](docs/tutorials/GETTING_STARTED.md) |
| Configuration | [docs/reference/CONFIG.md](docs/reference/CONFIG.md) |
| Troubleshooting | [docs/how-to/TROUBLESHOOTING.md](docs/how-to/TROUBLESHOOTING.md) |
| Project status and metrics | [docs/project/status/index.md](docs/project/status/index.md) |
| Roadmap | [docs/project/ROADMAP.md](docs/project/ROADMAP.md) |
| Release history | [RELEASE_HISTORY.md](RELEASE_HISTORY.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Agent workflow | [AGENTS.md](AGENTS.md) |

## How this project is built

This repository uses a documented AI-assisted development conveyor: candidate generation, ensemble curation, layered verification, CI evidence, fix-forward, and release ratchets.

Start here:

| Topic | Link |
|---|---|
| Pipeline state machine | [docs/articles/PIPELINE_STATE_MACHINE.md](docs/articles/PIPELINE_STATE_MACHINE.md) |
| Forensics / learning archive | [docs/forensics/](docs/forensics/) |
| Dispatch index | [docs/forensics/dispatch-index.toml](docs/forensics/dispatch-index.toml) |
| Agent workflow | [AGENTS.md](AGENTS.md) |
| Contributor workflow | [CONTRIBUTING.md](CONTRIBUTING.md) |

The short version: bad PRs should fail cheaply, duplicate PRs should close quickly, good PRs should merge safely, master should stay healthy, and every cycle should improve both the codebase and the conveyor.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). AI implementation agents should read [AGENTS.md](AGENTS.md) first.

## Security

Release artifacts include SBOM generation and provenance attestations. See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md).

## License

Dual licensed under MIT or Apache-2.0: [LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE).
