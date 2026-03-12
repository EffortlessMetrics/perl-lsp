# Perl Language Server

[![Visual Studio Marketplace Version](https://img.shields.io/visual-studio-marketplace/v/effortlesssteven.perl-lsp?label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)
[![Visual Studio Marketplace Downloads](https://img.shields.io/visual-studio-marketplace/d/effortlesssteven.perl-lsp)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)

Perl language support for VS Code, powered by the native `perl-lsp` server.

## Features

- Go to Definition, Find References, Rename Symbol
- Hover, Signature Help, Auto Completion, Document Symbols
- Diagnostics for syntax and semantic issues
- Semantic highlighting and inlay hints
- Refactor commands (Extract Variable/Subroutine, Inline Variable)
- Test runner command for `.t` and `.pl` files
- Perl debug adapter integration (launch + attach)
- Optional formatting integration (`perltidy`)

## Installation

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp).

On first activation the extension can auto-download a matching `perl-lsp` binary for your platform.

## Quick configuration

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.serverPath": "",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": false,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"]
}
```

## Common settings

- `perl-lsp.autoDownload`: Download server binary automatically if needed.
- `perl-lsp.serverPath`: Absolute path to a local `perl-lsp` binary.
- `perl-lsp.enableDiagnostics`: Enable parser + semantic diagnostics.
- `perl-lsp.enableSemanticTokens`: Enable semantic token highlighting.
- `perl-lsp.enableFormatting`: Enable formatting support (requires formatter tooling).
- `perl-lsp.perltidyConfig`: Path to `.perltidyrc`.
- `perl-lsp.includePaths`: Additional include paths for module resolution.

## Commands

- `Perl: Restart Perl Language Server`
- `Perl: Show Server Version`
- `Perl: Show Output Channel`
- `Perl: Show Status Menu`
- `Perl: Organize Use Statements`
- `Perl: Run Tests in Current File`
- `Perl Refactor: Extract Subroutine`
- `Perl Refactor: Extract Variable`
- `Perl Refactor: Inline Variable`

## Requirements

- VS Code `1.88.0+`
- Perl runtime available on PATH for running/debugging scripts
- `perltidy` installed if formatting is enabled

## Resources

- [Project repository](https://github.com/EffortlessMetrics/perl-lsp)
- [Issue tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Extension changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/CHANGELOG.md)

## License

MIT © Steven Zimmerman
