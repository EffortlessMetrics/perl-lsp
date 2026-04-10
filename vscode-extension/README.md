# Perl Language Server

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/EffortlessMetrics.perl-lsp-rs?label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
[![Visual Studio Marketplace Downloads](https://img.shields.io/visual-studio-marketplace/d/EffortlessMetrics.perl-lsp-rs)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
[![Open VSX Version](https://img.shields.io/open-vsx/v/EffortlessMetrics/perl-lsp-rs?label=Open%20VSX)](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs)
[![Open VSX Downloads](https://img.shields.io/open-vsx/dt/EffortlessMetrics/perl-lsp-rs?label=Open%20VSX%20downloads)](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs)

A fast, native Perl 5 language server extension. Written in Rust for speed and reliability. No runtime dependencies -- just install and code.

> **0.12.3 Public Alpha** -- This extension is under active development. Please [report issues](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose) if you encounter problems.

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

### Test Explorer
- **Test Discovery** -- Automatic discovery of `.t` test files
- **Run Tests** -- Run individual tests or entire files from the Testing panel (`Shift+Alt+T`)
- **TAP Support** -- Native Test Anything Protocol result parsing

### Walkthrough Previews

These storyboard SVGs are preview assets for the walkthrough flow. They are not
the final recorded GIFs.

- [Install, auto-download, and health check storyboard](media/walkthrough/install-health.svg)
- [Go to definition and find references storyboard](media/walkthrough/find-references.svg)
- [Extract variable code action storyboard](media/walkthrough/extract-variable.svg)

See [media/walkthrough/README.md](media/walkthrough/README.md) for the capture plan, recommended render inputs, and GIF size checks.

## Installation

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs) or [Open VSX Registry](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs).

```bash
# VS Code
code --install-extension EffortlessMetrics.perl-lsp-rs

# VSCodium / Open VSX
codium --install-extension EffortlessMetrics.perl-lsp-rs
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
- XS interface files (`.xs` and `.i`) are associated with Perl for bundled syntax highlighting

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

## Resources

- [Source Code](https://github.com/EffortlessMetrics/perl-lsp)
- [Issue Tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/CHANGELOG.md)

## License

MIT
