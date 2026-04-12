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
  <a href="https://open-vsx.org/extension/effortlessmetrics/perl-lsp-rs"><img src="https://img.shields.io/open-vsx/v/effortlessmetrics/perl-lsp-rs" alt="Open VSX" /></a>
</p>

---

> **Initial Public Alpha (`main` preparing v0.12.0)** -- perl-lsp is production-ready for daily use and actively improving.
> The workspace version is already `v0.12.0`, but the latest published release remains `v0.11.0` until the `v0.12.0` tag is cut.
> Install in minutes, get completions and navigation immediately.
> [Report issues](https://github.com/EffortlessMetrics/perl-lsp/issues) or [join the conversation](https://github.com/EffortlessMetrics/perl-lsp/discussions).

**The only Perl language server that doesn't require Perl to work.** A zero-dependency Rust binary with 98 LSP features, validated against thousands of real-world CPAN modules. Works on Windows, Mac, and Linux out of the box.

## Why perl-lsp?

- **No Perl runtime required** -- a single native binary; no dependency on a working Perl installation for IDE features.
- **Fast** -- sub-millisecond incremental parsing, under 50ms LSP response times.
- **Comprehensive** -- 98 LSP/DAP features including completion, diagnostics, hover, go-to-definition, references, rename, formatting, semantic highlighting, code actions, and debugging.
- **Perl-aware hover** -- hover over any special variable (`$_`, `@ARGV`, `%ENV`, `$/`, and 60+ more) to get full built-in documentation inline, without leaving your editor.
- **Broad syntax coverage** -- parses Perl 5.8 through 5.40 including heredocs, regex, quoting constructs, formats, and OO frameworks.
- **CPAN-validated** -- continuously tested against top CPAN distributions with a ratchet-only-forward CI gate that never allows regressions.

## Quick Start

### VS Code (recommended)

Install the extension and open a Perl file -- completions, diagnostics, hover, and navigation work immediately:

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

The extension auto-downloads the server binary for your platform.

### Binary install

```bash
cargo install perl-lsp
perl-lsp --health
```

That installs the latest published release from crates.io. For the current `main`
branch during `v0.12.0` initial-public-alpha prep, use the source install below or
download the matching artifact once the release is tagged.

Or download a pre-built binary from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

### Neovim

```lua
require('lspconfig').perl_ls.setup {
  cmd = { "perl-lsp", "--stdio" },
}
```

### Emacs (eglot)

```elisp
(add-to-list 'eglot-server-programs '(perl-mode "perl-lsp" "--stdio"))
```

### Other editors

Any editor with LSP support works. Point it at `perl-lsp --stdio` as the language server command.

For a full walkthrough with troubleshooting tips, see the **[Getting Started guide](docs/tutorials/GETTING_STARTED.md)**.

## Features

| What you see | What it does |
|-------------|-------------|
| **Diagnostics** | Real-time parse error detection as you type |
| **Completions** | 150+ builtins, workspace symbols, modules, and keywords |
| **Go to definition** | Cross-file navigation for subs, methods, and modules |
| **Hover** | Function signatures, documentation, module info, and **built-in special variable docs** (hover over `$_`, `@ARGV`, `%ENV`, etc. for instant reference) |
| **Find references** | Locate all usages of a symbol across your workspace |
| **Rename** | Scoped refactoring across files |
| **Formatting** | Perl::Tidy integration |
| **Code actions** | Organize imports, modernize syntax, quick fixes |
| **Semantic highlighting** | Context-aware syntax coloring |
| **Debugging** | Built-in DAP: breakpoints, stepping, variables, watch — [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) |
| **And 85+ more...** | Inlay hints, code lens, call hierarchy, folding, color decorators |

The full feature catalog lives in [`features.toml`](features.toml). For live project metrics, see [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

### Demo Walkthroughs

The launch demo assets are staged as storyboard SVG previews in [`vscode-extension/media/walkthrough/`](vscode-extension/media/walkthrough/). They are planning aids, not final demos.

- [Install → auto-download → health check](vscode-extension/media/walkthrough/install-health.svg)
- [Go to definition + find references](vscode-extension/media/walkthrough/find-references.svg)
- [Extract variable code action](vscode-extension/media/walkthrough/extract-variable.svg)

When the screen recordings are ready, render them with [`scripts/marketing/render-walkthrough-gif.py`](scripts/marketing/render-walkthrough-gif.py). Use `--max-bytes` so the final GIF stays readable and README-friendly.

## Comparison

| | perl-lsp | PerlNavigator | Perl::LanguageServer |
|---|----------|--------------|---------------------|
| **Language** | Rust (native binary) | Perl | Perl |
| **Requires Perl runtime** | No | Yes | Yes |
| **Windows support** | Native | Via Perl | Limited |
| **Incremental parsing** | Yes (sub-ms) | N/A | N/A |
| **Debug adapter** | Built-in (DAP) | No | Built-in |
| **CPAN corpus validation** | CI-gated, ratchet-forward | N/A | N/A |
| **Install** | Single binary | CPAN + Perl | CPAN + Perl |

## Configuration

perl-lsp is configured through your editor's LSP settings (via `didChangeConfiguration`). All settings are optional -- defaults work out of the box.

```jsonc
// VS Code settings.json example
{
  "perl-lsp.inlayHints": {
    "enabled": true,
    "parameterHints": true,
    "typeHints": true,
    "maxLength": 30
  },
  "perl-lsp.workspace": {
    "includePaths": ["lib", ".", "local/lib/perl5"],
    "useSystemInc": false
  },
  "perl-lsp.testRunner": {
    "enabled": true,
    "command": "perl",
    "timeout": 60000
  }
}
```

| Setting | Default | Description |
|---------|---------|-------------|
| `inlayHints.enabled` | `true` | Toggle inlay hints globally |
| `inlayHints.parameterHints` | `true` | Show parameter name hints at call sites |
| `inlayHints.typeHints` | `true` | Show inferred type hints for variables |
| `workspace.includePaths` | `["lib", ".", "local/lib/perl5"]` | Module resolution search paths |
| `workspace.useSystemInc` | `false` | Include system `@INC` paths |
| `testRunner.command` | `"perl"` | Command for integrated test runner (`perl`, `prove`) |
| `testRunner.timeout` | `60000` | Test execution timeout (ms) |

For Neovim and other editors, pass these as the LSP `settings` table under the `perl-lsp` key.

### Project Configuration File

For team-wide defaults, add a `.perl-lsp.toml` to your repository root. It is editor-agnostic and committed to version control:

```toml
# .perl-lsp.toml — shared project defaults for perl-lsp

[perl]
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
perlcritic = false

[features]
inlay_hints = true
```

Settings from `.perl-lsp.toml` are the lowest-priority layer. Editor settings (`initializationOptions` / `didChangeConfiguration`) always override them. See [CONFIG.md](docs/reference/CONFIG.md) for the full reference including precedence rules.

## Install

### From crates.io

```bash
cargo install perl-lsp
perl-lsp --health
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
perl-lsp --health
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

### Windows package managers

The release automation keeps the Windows package-manager manifests in sync with
each release. If you are on Windows, these are the user-facing install paths:

```powershell
scoop install perl-lsp
choco install perl-lsp
```

After installation, verify the binary with `perl-lsp --health`.
The repo documents the automated vs manual verification boundary in
[docs/how-to/INSTALLATION.md](docs/how-to/INSTALLATION.md) and
[docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md).

### Linux package-manager scaffold

Repo-owned packaging templates for `apt`, `dnf`, and `pacman` live under [`distribution/linux/`](distribution/linux/).
They are intentionally kept as templates only in this slice, so downstream release automation can render them without depending on external distro approvals.
To render them for a specific release, use [`scripts/render-linux-packages.py`](scripts/render-linux-packages.py) with the values from [`distribution/linux/package-metadata.toml`](distribution/linux/package-metadata.toml).

### VS Code Extension

Install from the VS Code Marketplace or:

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

You can set `perl-lsp.serverPath` to use a specific binary, or disable `perl-lsp.autoDownload` for airgapped environments.

## Parser

The v3 parser is a native recursive-descent implementation covering broad Perl 5 syntax
(5.8 through 5.40), including heredocs, regex, quoting constructs, and formats. It is
tested continuously against real-world Perl code:

- **Corpus test suite** -- 600+ test sections plus 70+ standalone `.pl` fixtures.
- **CPAN corpus** -- benchmarked against the top 1000 CPAN distributions with a ratchet-only-forward CI gate.
- **Common-files gate** -- a curated set of core modules that must parse with zero errors on every PR.

Current parse rates and the edge-case roadmap are tracked in [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) and [PARSER_EDGE_CASE_ROADMAP.md](docs/project/PARSER_EDGE_CASE_ROADMAP.md).

## Architecture

The workspace is organized as 130+ focused Rust crates, each with a single responsibility. The main entry points:

| Crate | Purpose |
|-------|---------|
| [`perl-lsp`](crates/perl-lsp/) | LSP server binary |
| [`perl-dap`](crates/perl-dap/) | Debug Adapter Protocol server |
| [`perl-parser`](crates/perl-parser/) | Native recursive-descent Perl parser |
| [`perl-lexer`](crates/perl-lexer/) | Context-aware tokenizer |
| [`perl-semantic-analyzer`](crates/perl-semantic-analyzer/) | Semantic analysis and resolution |

Published crates are available on [crates.io](https://crates.io/crates/perl-lsp): `perl-lsp`, `perl-dap`, `perl-parser`, `perl-lexer`, and `perl-corpus`.

For design details, see the [LSP Implementation Guide](docs/reference/LSP_IMPLEMENTATION_GUIDE.md), [Crate Architecture Guide](docs/reference/CRATE_ARCHITECTURE_GUIDE.md), and [Architecture Decision Records](docs/adr/README.md).

## Contributing

```bash
cargo build --workspace            # Build everything
cargo test --workspace --lib       # Run all tests
cargo clippy --workspace --lib     # Lint
cargo fmt --all                    # Format
nix develop -c just ci-gate        # Full local gate (required before push)
```

Quick iteration: `just pr-fast`. Environment check: `just devex` or `just doctor`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines,
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards,
and [SUPPORT.md](SUPPORT.md) for how to get help.

## Security

Release artifacts include SBOM generation (SPDX and CycloneDX) and SLSA Level 2
provenance attestations. Production code enforces zero `unsafe`, zero `unwrap`/`expect`,
and zero `panic!`-family macros via CI ratchets.

See [Supply Chain Security](docs/reference/SUPPLY_CHAIN_SECURITY.md) for details.

## Documentation

| Resource | Description |
|----------|-------------|
| **[Getting Started](docs/tutorials/GETTING_STARTED.md)** | Installation, editor setup, and first-run walkthrough |
| [Full Documentation Index](docs/INDEX.md) | Complete guide to all project documentation |
| [Current Status](docs/project/CURRENT_STATUS.md) | Live project metrics |
| [Roadmap](docs/project/ROADMAP.md) | Version milestones and planning |
| [Troubleshooting](docs/how-to/TROUBLESHOOTING.md) | Common issues and solutions |
| [features.toml](features.toml) | Canonical LSP feature catalog |
| [Stability Policy](docs/reference/STABILITY.md) | API versioning and compatibility |
| [DAP User Guide](docs/tutorials/DAP_USER_GUIDE.md) | Debugger setup and usage |
| [Contributing](CONTRIBUTING.md) | Development guidelines and workflow |
| [Changelog](CHANGELOG.md) | Release history and notable changes |
| **[Report an Issue](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)** | Bug reports, feature requests, parser issues |

## History

This project began as a fork of [tree-sitter-perl](https://github.com/tree-sitter-perl/tree-sitter-perl) in July 2025. It has since been rewritten as a native Rust recursive-descent parser and grown into a full-featured LSP/DAP toolkit.

## License

Dual licensed under MIT or Apache-2.0:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
