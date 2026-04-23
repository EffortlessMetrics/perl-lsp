# Perl Language Server

[![Visual Studio Marketplace](https://img.shields.io/badge/VS%20Marketplace-live%20listing-0078D4)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
[![VS Marketplace Installs (manual)](https://img.shields.io/badge/VS%20Marketplace-180%20installs-0078D4)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
[![Open VSX Version](https://img.shields.io/open-vsx/v/EffortlessMetrics/perl-lsp-rs?label=Open%20VSX)](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs)
[![Open VSX Downloads](https://img.shields.io/open-vsx/dt/EffortlessMetrics/perl-lsp-rs?label=Open%20VSX%20downloads)](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs)

A fast, native Perl 5 language server extension. Written in Rust for speed and reliability. No runtime dependencies -- just install and code.

> **0.12.3 Public Alpha** -- This extension is under active development. Every feature listed below is wired up and exercised by tests, but as an alpha you will find edge cases where behavior is incomplete or wrong. Please [report issues](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose) if you encounter problems. For what the project's headline numbers mean (and do not mean), see the [status overview](https://github.com/EffortlessMetrics/perl-lsp/blob/master/docs/project/status/index.md).

## Features

### Navigation and Intelligence
- **Go to Definition** -- Jump to any symbol declaration across files
- **Find References** -- Find all usages of a symbol across your project
- **Hover Documentation** -- Instant docs for functions, variables, and modules
- **Auto-completion** -- Smart suggestions for variables, functions, and module names
- **Signature Help** -- Real-time parameter hints as you type function calls
- **Symbol Navigation** -- Outline view, breadcrumbs, and workspace symbol search

### Refactoring and Code Actions
- **Rename** -- Safe renaming of symbols across files
- **Extract Variable** -- Pull out expressions into named variables
- **Extract Subroutine** -- Create functions from selected code blocks
- **Organize Imports** -- Sort and clean `use` statements (`Shift+Alt+O`)

### Diagnostics and Quality
- **Real-time Errors** -- Syntax and semantic error detection as you type
- **Undefined Variables** -- Catch typos under `use strict`
- **Unused Variables** -- Find dead code
- **Missing Pragmas** -- Suggest `strict` and `warnings`
- **Document Formatting** -- Format with `perltidy` (`Shift+Alt+F`)

### Advanced Features
- **Semantic Highlighting** -- Context-aware syntax coloring beyond TextMate grammars
- **Type Hierarchy** -- Navigate inheritance with `@ISA` and `use parent`
- **Call Hierarchy** -- Trace function calls inbound and outbound
- **CodeLens** -- Inline reference counts above functions
- **Inlay Hints** -- Type annotations shown inline in the editor
- **Code Folding** -- Collapse subs, blocks, POD, and heredocs

### Debugging (via perl-dap)
- **Breakpoints** -- Set breakpoints with conditional support
- **Step Debugging** -- Step into, over, and out of function calls
- **Variable Inspection** -- View variables, watch expressions, and call stack
- **Attach to Process** -- Debug running Perl processes by PID or TCP

Debugging is optional and uses `perl-dap` as a separate adapter. See the
[debugging guide](../docs/tutorials/DAP_USER_GUIDE.md) for setup steps and
the required launch configuration.

### Test Explorer
- **Test Discovery** -- Automatic discovery of `.t` test files
- **Run Tests** -- Run individual tests or entire files from the Testing panel (`Shift+Alt+T`)
- **TAP Support** -- Native Test Anything Protocol result parsing

### Extension Coexistence

If VS Code warns that other Perl extensions are installed, keep one provider
for navigation, diagnostics, and formatting where possible. Perl Navigator,
Perl::Critic, and PerlTidy can overlap with perl-lsp features. If you see
duplicate hover, completion, or formatting results, disable the competing
feature in one extension and keep the other as the source of truth.

### Walkthrough Media

The extension includes a "Get Started" walkthrough in VS Code. Walkthrough
media assets and recording notes live in:

- [media/walkthrough/README.md](media/walkthrough/README.md)
- [media/walkthrough/install-health.svg](media/walkthrough/install-health.svg)
- [media/walkthrough/find-references.svg](media/walkthrough/find-references.svg)
- [media/walkthrough/extract-variable.svg](media/walkthrough/extract-variable.svg)

## Installation

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs) or [Open VSX Registry](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs).

```bash
# VS Code
code --install-extension EffortlessMetrics.perl-lsp-rs

# VSCodium / Open VSX
codium --install-extension EffortlessMetrics.perl-lsp-rs

# PearAI (VS Code-compatible)
# Install from Open VSX inside PearAI's Extensions view:
# EffortlessMetrics.perl-lsp-rs
```

The extension automatically downloads the correct `perllsp` binary for your platform on first activation:

| Platform | Architectures |
|----------|--------------|
| **Windows** | x64, ARM64 |
| **macOS** | Intel (x64), Apple Silicon (ARM64) |
| **Linux** | x64, ARM64 (glibc and musl) |

### Enterprise / offline / air-gapped deployments

The extension downloads the Perl LSP server binary on first activation. If your environment blocks internet access during extension install or uses a strict proxy, see [`INTERNAL_DEPLOYMENT.md`](./INTERNAL_DEPLOYMENT.md) for:

- Pre-downloading the binary and bundling it with your VSIX
- Using `perl-lsp.serverPath` to point at a shared binary
- Corporate proxy and certificate configuration

### Manual Installation

If you prefer to manage the binary yourself:

```bash
# Homebrew (macOS/Linux)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# From source
cargo install --git https://github.com/EffortlessMetrics/perl-lsp --package perllsp
```

Then point the extension to your `perllsp` binary via `perl-lsp.serverPath`.

## Configuration

All settings are under the `perl-lsp.*` namespace. Open settings with `Ctrl+,` and search for "perl-lsp".

| Setting | Default | Description |
|---------|---------|-------------|
| `perl-lsp.autoDownload` | `true` | Automatically download `perllsp` if not found locally |
| `perl-lsp.serverPath` | `""` | Absolute path to a `perllsp` binary (overrides auto-download) |
| `perl-lsp.channel` | `"latest"` | Release channel: `latest`, `stable`, or `tag` |
| `perl-lsp.versionTag` | `""` | Specific release tag (e.g. `v0.12.1`) when channel is `tag` |
| `perl-lsp.enableDiagnostics` | `true` | Enable real-time syntax diagnostics |
| `perl-lsp.enableSemanticTokens` | `true` | Enable semantic syntax highlighting |
| `perl-lsp.enableFormatting` | `true` | Enable document formatting (requires `perltidy`) |
| `perl-lsp.formatOnSave` | `false` | Format document on save |
| `perl-lsp.enableRefactoring` | `true` | Enable refactoring code actions |
| `perl-lsp.enableTestIntegration` | `true` | Enable Test Explorer integration |
| `perl-lsp.includePaths` | `["lib", "local/lib/perl5"]` | Additional library paths for module resolution |
| `perl-lsp.perltidyConfig` | `""` | Path to `.perltidyrc` (auto-detected if empty) |
| `perl-lsp.trace.server` | `"off"` | LSP trace level for debugging: `off`, `messages`, `verbose` |
| `perl-lsp.featureProfile` | `"auto"` | Runtime feature profile: `auto`, `ga`, `ga-lock`, `prod`, `all` |
| `perl-lsp.downloadBaseUrl` | `""` | Internal mirror URL for air-gapped deployments |

### Internal / Air-Gapped Deployment

For environments without internet access, set `perl-lsp.downloadBaseUrl` to an internal server hosting the release archives and `SHA256SUMS` file. See [INTERNAL_DEPLOYMENT.md](https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/INTERNAL_DEPLOYMENT.md) for details.

## Keyboard Shortcuts

Use `Ctrl+Shift+P` (Command Palette) and search "Perl" to see all available commands.

| Action | Shortcut |
|--------|----------|
| Organize Imports | `Shift+Alt+O` |
| Run Tests | `Shift+Alt+T` |
| Restart Server | `Shift+Alt+R` |
| Format Document | `Shift+Alt+F` |
| Show Status Menu | Click status bar item |

## Supported Perl Features

### Modern Perl (5.38+)
- `class` / `method` / `field` keywords
- `try` / `catch` / `finally` blocks
- `defer` blocks
- Subroutine signatures
- Type constraints

### Complete Syntax Support
- Regular expressions with any delimiter (`m!pattern!`, `s{}{}``)
- Heredocs (all variants including indented `<<~`)
- Unicode identifiers (`my $cafe = 'coffee'`)
- Postfix dereferencing (`$ref->@*`)
- Smart match operator (`~~`)
- Indirect object syntax
- Built-in function signatures with parameter documentation
- XS interface files (`.xs`) and SWIG interface files (`.i`) are associated with Perl for bundled syntax highlighting, including common SWIG directives and embedded C/C++ blocks

## Commands

Open the command palette (`Ctrl+Shift+P`) and search for "Perl":

| Command | Description |
|---------|-------------|
| **Perl: Restart Language Server** | Restart the language server |
| **Perl: Show Server Version** | Display installed perllsp version |
| **Perl: Reinstall Server Binary** | Re-download the managed binary |
| **Perl: Organize Use Statements** | Sort and clean `use` statements |
| **Perl: Run Tests in Current File** | Run tests in the active `.t` or `.pl` file |
| **Perl: Show Output Channel** | Open the extension output log |
| **Perl: Show Status Menu** | Quick-access menu for all actions |

## Compatibility

The `perllsp` binary works with any editor that supports the Language Server Protocol:

| Editor | How to connect |
|--------|---------------|
| **VS Code / VSCodium** | This extension (auto-configured) |
| **Cursor** | This extension |
| **PearAI** | This extension (install from Open VSX) |
| **Neovim** | `nvim-lspconfig` with `perl_lsp` server |
| **Emacs** | `lsp-mode` or `eglot` |
| **Helix** | `languages.toml` with `perllsp --stdio` |
| **Sublime Text** | LSP package with `perllsp --stdio` |
| **GitHub Codespaces** | This extension |
| **Gitpod** | This extension |

## Troubleshooting

**Server not starting?**
1. Open the output channel: Command Palette > "Perl: Show Output Channel"
2. Check that `perllsp` is available: Command Palette > "Perl: Show Server Version"
3. If auto-download failed, check your network/proxy settings or install manually

**Formatting not working?**
- Ensure `perltidy` is installed and available in your PATH
- Check `perl-lsp.enableFormatting` is `true`

**Diagnostics too noisy?**
- Set `perl-lsp.enableDiagnostics` to `false` to disable
- File an issue if you see false positives

## Known Issues

- Variable/watch rendering in debugger sessions is still evolving; complex Perl
  structures may appear with placeholder values in some scenarios.
- The `Format Document` shortcut (`Shift+Alt+F`) is provided by VS Code's
  built-in formatter binding. perl-lsp participates through the registered
  formatting provider when `perl-lsp.enableFormatting` is enabled.
- On first activation, environments with strict proxies or blocked outbound
  traffic may fail auto-download. Use `perl-lsp.serverPath` or
  `perl-lsp.downloadBaseUrl` for managed/internal deployment.

## Resources

- [Source Code](https://github.com/EffortlessMetrics/perl-lsp)
- [Issue Tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/CHANGELOG.md)
- [Open VSX Registry](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs) — alternative marketplace for VSCodium and other open-source VS Code derivatives
- [Sponsor this project](https://github.com/EffortlessMetrics/perl-lsp) — support continued development

## License

MIT
