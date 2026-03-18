<p align="center">
  <img src="vscode-extension/icon.png" alt="perl-lsp logo" width="120" />
</p>

<h1 align="center">perl-lsp</h1>

<p align="center">
  A fast, native <strong>Perl language server</strong> written in Rust — bringing modern IDE features to Perl 5.
</p>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://crates.io/crates/perl-lsp"><img src="https://img.shields.io/crates/v/perl-lsp.svg" alt="crates.io" /></a>
  <a href="https://docs.rs/perl-lsp"><img src="https://docs.rs/perl-lsp/badge.svg" alt="docs.rs" /></a>
  <a href="https://codecov.io/gh/EffortlessMetrics/perl-lsp"><img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="codecov" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.92%2B-orange.svg" alt="Rust" /></a>
  <a href="https://crates.io/crates/perl-lsp"><img src="https://img.shields.io/crates/d/perl-lsp.svg" alt="Downloads" /></a>
</p>

---

> **Note**: perl-lsp is in public alpha. Core features work well, but expect some rough edges. [Report issues](https://github.com/EffortlessMetrics/perl-lsp/issues).

## Quick Start

```bash
# 1. Install
cargo install perl-lsp

# 2. Verify the install
perl-lsp --health  # should print: ok X.Y.Z

# 3. Configure your editor (VS Code)
code --install-extension effortlessmetrics.perl-lsp-rs

# 4. Open a Perl file — completions, diagnostics, hover, and navigation work immediately.
```

New to language servers? See the **[Getting Started guide](docs/tutorials/GETTING_STARTED.md)** for a full walkthrough with editor-specific setup, a visual feature tour, and troubleshooting tips.

<details>
<summary><strong>Neovim / Emacs setup</strong></summary>

**Neovim** (nvim-lspconfig):

```lua
require('lspconfig').perl_ls.setup {
  cmd = { "perl-lsp", "--stdio" },
}
```

**Emacs** (eglot):

```elisp
(add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
```

</details>

## Features

| Feature | Description |
|---------|-------------|
| **Completion** | Symbols, keywords, modules, variables, snippets, built-in function signatures |
| **Navigation** | Go-to-definition, find references, workspace symbols, document symbols |
| **Hover** | Documentation and type information |
| **Diagnostics** | Real-time error detection |
| **Refactoring** | Rename, code actions, formatting |
| **Debug Adapter** | Breakpoints, stepping, variable inspection via DAP bridge |
| **Fast** | Sub-millisecond incremental parsing, native Rust binary |
| **Zero Perl dependency** | No Perl runtime needed for parsing or LSP |
| **Cross-file navigation** | Dual-indexed workspace with broad reference coverage |
| **Unicode-safe** | Full UTF-8/UTF-16 position conversion |

Full LSP coverage: all 53 advertised capabilities implemented (see [`features.toml`](features.toml)).
For live metrics, see [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Why perl-lsp?

| | perl-lsp | Perl::LanguageServer | PLS |
|---|----------|---------------------|-----|
| **Language** | Rust (native binary) | Perl | Perl |
| **Requires Perl runtime** | No | Yes | Yes |
| **LSP feature coverage** | 53/53 advertised | Partial | Partial |
| **Incremental parsing** | Yes (sub-ms updates) | N/A | N/A |
| **Debug adapter** | Built-in (DAP bridge) | Built-in | No |
| **Supply chain security** | SBOM + SLSA Level 2 | N/A | N/A |

## Parser Coverage

The v3 parser is a native recursive-descent implementation covering broad Perl 5 syntax
(5.8 through 5.40), including heredocs, regex, quoting constructs, formats, and more.
It is tested continuously against real-world Perl code to drive coverage improvements:

- **Corpus test suite** -- 600+ test sections in `tree-sitter-perl/test/corpus/` plus 70+ standalone `.pl` fixtures.
- **System Perl corpus sweep** -- benchmarked against all `.pm` and `.pl` files found in the system Perl installation. Current parse rates are tracked in [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) and the baseline file (`.ci/parser-corpus-baseline.json`).
- **Common-files gate** -- a curated set of core modules that must parse with zero errors on every PR (`.ci/common-corpus-manifest.txt`).
- **Ratcheting CI gate** -- the overall parse rate can only go up, never down. Regressions fail the build.
- **CPAN top 1000 goal** -- the long-term target is for 90%+ of the most-downloaded CPAN distributions to parse cleanly, driving parser improvements toward real-world Perl idioms.

For detailed parse rates and the edge-case roadmap, see [PARSER_EDGE_CASE_ROADMAP.md](docs/project/PARSER_EDGE_CASE_ROADMAP.md).

### Corpus Commands

```bash
just corpus-sweep          # Sweep system Perl corpus and print results
just corpus-sweep-check    # Check against baseline (fails on regression)
just corpus-sweep-update   # Update baseline with current results
just common-corpus-check   # Check pinned modules parse cleanly (PR gate)
just cpan-corpus-fetch     # Fetch CPAN top-1000 distribution list
just cpan-corpus-install   # Install CPAN corpus locally; auto-fetches the list, bootstraps cpanm, and reuses a local cache
just cpan-corpus-baseline-update  # Seed/update CPAN ratchet baseline
just cpan-corpus-check     # Check CPAN baseline + known-clean manifest
```

See [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for the latest computed metrics.

## Install

### From crates.io

```bash
cargo install perl-lsp
perl-lsp --health  # should print: ok X.Y.Z
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health  # should print: ok X.Y.Z
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

## Architecture

```text
Editor / IDE
    |  JSON-RPC (stdio)
    v
perl-lsp  (LSP server)
    |
    +-- LSP Providers (completion, hover, diagnostics, ...)
    +-- Workspace Index (dual-indexed symbol lookup)
    +-- perl-dap (Debug Adapter Protocol bridge)
    |
perl-parser  (v3 recursive-descent)
    |
perl-lexer   (mode-aware tokenizer)
```

The workspace is organized as a family of focused Rust crates (115+), each with a
single responsibility. This keeps compile times fast and boundaries clear.

For the full tier system, architecture decision records, and design rationale, see:

- [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)
- [Architecture Decision Records](docs/adr/README.md) (microcrate architecture, dual indexing, incremental parsing, supply chain security)
- [CLAUDE.md](CLAUDE.md) for the complete developer command reference

## Published Crates

| Crate | Purpose |
|-------|---------|
| [`perl-lsp`](https://crates.io/crates/perl-lsp) | Language Server Protocol binary |
| [`perl-dap`](https://crates.io/crates/perl-dap) | Debug Adapter Protocol binary |
| [`perl-parser`](https://crates.io/crates/perl-parser) | Recursive-descent Perl parser library |
| [`perl-lexer`](https://crates.io/crates/perl-lexer) | Context-aware Perl tokenizer |
| [`perl-corpus`](https://crates.io/crates/perl-corpus) | Parser and LSP test corpus |

## Development

```bash
cargo build --workspace            # Build everything
cargo test --workspace             # Run all tests
cargo clippy --workspace --lib     # Lint
cargo fmt --all                    # Format
nix develop -c just ci-gate        # Full local gate (required before push)
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines,
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards,
and [SUPPORT.md](SUPPORT.md) for how to get help.

## Security

Release artifacts include SBOM generation (SPDX and CycloneDX) and SLSA Level 2
provenance attestations. Production code enforces zero `unsafe`, zero `unwrap`/`expect`,
and zero `panic!`-family macros via CI ratchets. Path traversal and command injection
vectors are hardened.

See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md) for details.

## Documentation

Start with the docs index if you are not sure where to go next: [docs/README.md](docs/README.md).

| Need | Start here |
|------|------------|
| Install and configure the server | [Getting Started](docs/tutorials/GETTING_STARTED.md) |
| Browse the full documentation map | [docs/README.md](docs/README.md) |
| Understand current project health | [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) |
| Review release planning | [ROADMAP.md](ROADMAP.md) |
| Check API and compatibility promises | [Stability Policy](docs/reference/STABILITY.md) |
| Set up debugger workflows | [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) |
| Inspect the canonical LSP capability catalog | [features.toml](features.toml) |
| Read the longer-form guide | [book/](book/) |

### Documentation paths by audience

- **New users**: start with [Getting Started](docs/tutorials/GETTING_STARTED.md), then [Editor Setup](docs/how-to/EDITOR_SETUP.md), and [Troubleshooting](docs/how-to/TROUBLESHOOTING.md).
- **Contributors**: start with [CONTRIBUTING.md](CONTRIBUTING.md), then [Commands Reference](docs/reference/COMMANDS_REFERENCE.md), and [docs/README.md](docs/README.md).
- **Maintainers**: use [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), [CI Local Validation](docs/project/CI_LOCAL_VALIDATION.md), and [ROADMAP.md](ROADMAP.md).

## History

This project began as a fork of [tree-sitter-perl](https://github.com/tree-sitter-perl/tree-sitter-perl) in July 2025. It has since been rewritten as a native Rust recursive-descent parser and grown into a full-featured LSP/DAP toolkit with over 115 crates.

## License

Dual licensed under MIT or Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
