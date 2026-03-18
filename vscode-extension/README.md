# Perl Language Server

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/EffortlessMetrics.perl-lsp-rs?label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
[![Visual Studio Marketplace Downloads](https://img.shields.io/visual-studio-marketplace/d/EffortlessMetrics.perl-lsp-rs)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)

A polished Perl development experience for Visual Studio Code, VSCodium, Cursor, Gitpod, and Codespaces.

Built on the Rust-based `perl-lsp` engine, this extension focuses on three things that matter in day-to-day Perl work:

- **Fast feedback** with syntax diagnostics, semantic highlighting, and completions.
- **Safe navigation and refactoring** across modules, packages, test files, and workspace boundaries.
- **Low-friction setup** with automatic server download plus optional custom binary paths for internal deployments.

## Why teams install this extension

Perl codebases are often long-lived, multi-package, and full of context-sensitive syntax. This extension is designed to make those codebases feel modern inside VS Code:

- Open a `.pl`, `.pm`, `.t`, `.pod`, or `.psgi` file and the language server activates automatically.
- Jump to definitions and references across workspace files, including package-qualified symbols.
- Organize `use` statements, inspect hover docs, and surface code actions without leaving the editor.
- Run tests from the editor and debug Perl processes with the bundled DAP integration.
- Keep installs predictable with release channels, version pinning, and internal artifact mirrors.

## Feature tour

### Core editing intelligence
- **Go to Definition** for variables, subs, and packages
- **Find References** across workspace modules
- **Hover Documentation** for quick symbol insight
- **Auto-completion** for modules, functions, and variables
- **Signature Help** while typing function arguments
- **Document Symbols** for breadcrumbs and outline navigation

### Quality, diagnostics, and formatting
- **Real-time syntax diagnostics** while editing
- **Semantic highlighting** with modern Perl awareness
- **Format document** using `perltidy`
- **Format on save** when you want automatic cleanup
- **Trace logging** for troubleshooting editor/server communication

### Refactoring and code actions
- **Rename** symbols safely across files
- **Organize Use Statements** with a command or keybinding
- **Quick fixes** when the connected server advertises them
- **Feature profiles** to choose conservative vs. all-on behavior

### Tests and debugging
- **Run tests in the current file** from commands, menus, or keybindings
- **Debug Perl scripts** with launch and attach configurations
- **Editor actions** for common Perl workflows instead of manual shell hopping

## Quick start

1. Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs).
2. Open any Perl workspace.
3. Run **`Perl: Show Server Version`** from the command palette to verify the server is ready.
4. If your team manages binaries centrally, set `perl-lsp.serverPath` or `perl-lsp.downloadBaseUrl`.
5. Optionally enable formatting with `perltidy` installed locally.

The extension automatically downloads the correct `perl-lsp` binary for your platform when `perl-lsp.serverPath` is not set.

Supported host platforms:
- Windows (x64, ARM64)
- macOS (Intel, Apple Silicon)
- Linux (x64, ARM64)

## Recommended settings

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.channel": "latest",
  "perl-lsp.serverPath": "",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"],
  "perl-lsp.trace.server": "off"
}
```

### Useful deployment controls
- **`perl-lsp.channel`**: choose `latest`, `stable`, or `tag`
- **`perl-lsp.versionTag`**: pin a specific release such as `v0.11.0`
- **`perl-lsp.serverPath`**: use a preinstalled binary on managed machines
- **`perl-lsp.downloadBaseUrl`**: point downloads at an internal artifact mirror
- **`perl-lsp.featureProfile`**: select runtime feature exposure

## Commands and shortcuts

| Command | Purpose | Default shortcut |
|---|---|---|
| `Perl: Restart Perl Language Server` | Restart the LSP process | `Shift+Alt+R` |
| `Perl: Show Server Version` | Confirm the running binary version | — |
| `Perl: Show Output Channel` | Inspect logs and startup messages | — |
| `Perl: Show Status Menu` | Open quick status actions | — |
| `Perl: Organize Use Statements` | Sort and clean imports | `Shift+Alt+O` |
| `Perl: Run Tests in Current File` | Run the current Perl test/script | `Shift+Alt+T` |

## Debugging and testing

The extension contributes a **Perl** debugger with ready-to-edit launch and attach templates.

### Launch example

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Perl: Launch Script",
  "program": "${workspaceFolder}/script.pl",
  "stopOnEntry": true
}
```

### Attach example

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Perl: Attach to Debugger",
  "host": "localhost",
  "port": 13603,
  "timeout": 5000
}
```

## Manual installation options

```bash
# Homebrew (macOS/Linux)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# From source
cargo install --git https://github.com/EffortlessMetrics/perl-lsp --bin perl-lsp
```

## Supported Perl coverage

### Modern Perl
- `class`, `method`, and `field`
- `try`, `catch`, and `finally`
- `defer`
- Subroutine signatures
- Type constraints

### Syntax edge cases handled well
- Regexes with alternate delimiters like `m!pattern!` and `s{}{}`
- Heredocs, including indented variants
- Unicode identifiers such as `my $café = 'coffee'`
- Postfix dereferencing like `$ref->@*`
- Smart match operator `~~`
- Indirect object syntax

### Built-in function support
- Rich signatures for 150+ Perl built-ins

## Best fit environments

This extension works especially well for:
- application teams maintaining large internal Perl services
- CPAN/module authors who need navigation across packages and tests
- platform teams standardizing editor tooling via pinned server versions
- developers using VS Code-compatible environments such as Cursor, Gitpod, or GitHub Codespaces

## Resources

- [Marketplace listing](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
- [Project documentation](https://github.com/EffortlessMetrics/perl-lsp#readme)
- [Issue tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Extension changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/CHANGELOG.md)
- [Internal deployment guide](./INTERNAL_DEPLOYMENT.md)
- [Publishing guide](./PUBLISHING.md)

## License

MIT © Steven Zimmerman
