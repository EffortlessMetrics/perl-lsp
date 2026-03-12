# Perl Language Server

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/effortlesssteven.perl-lsp?label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)
[![Visual Studio Marketplace Downloads](https://img.shields.io/visual-studio-marketplace/d/effortlesssteven.perl-lsp)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)

Fast Perl language support for VS Code, powered by [`perl-lsp`](https://github.com/EffortlessMetrics/perl-lsp).

## Why this extension

- Full Perl 5 language support with modern syntax coverage.
- Rich editor tooling: navigation, completion, diagnostics, semantic tokens, refactoring, and formatting.
- Built-in debugger integration (`perl-dap`) plus test-friendly commands.
- Automatic server download with manual override for internal/corporate environments.

## Features

### Core IDE support
- Go to Definition / Find References / Rename
- Hover documentation and signature help
- Document symbols, highlights, call hierarchy, type hierarchy
- Semantic tokens, inlay hints, and CodeLens

### Quality and refactoring
- Real-time syntax + semantic diagnostics
- Organize `use` statements
- Extract variable / extract subroutine / inline variable
- Optional formatting integration via `perltidy`

### Debugging and test workflows
- Launch + attach debugging for Perl scripts
- One-click command to run tests in the current file
- Snippets for `launch.json` and common Perl boilerplate

## Installation

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp).

On first run, the extension can auto-download the correct `perl-lsp` binary for your platform.

Supported platforms:
- Windows (x64, ARM64)
- macOS (Intel, Apple Silicon)
- Linux (x64, ARM64)

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

## Debug configuration example

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Perl: Launch Script",
      "program": "${workspaceFolder}/script.pl",
      "stopOnEntry": true
    }
  ]
}
```

## Troubleshooting

- Run `Perl: Show Output Channel` from the Command Palette to inspect logs.
- If auto-download is disabled, set `perl-lsp.serverPath` to an absolute `perl-lsp` binary path.
- If formatting does not run, install `perltidy` and enable `perl-lsp.enableFormatting`.

## Resources

- [Project repository](https://github.com/EffortlessMetrics/perl-lsp)
- [Issue tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Extension changelog](./CHANGELOG.md)
- [Workspace documentation](https://github.com/EffortlessMetrics/perl-lsp/tree/master/docs)

## License

MIT © Steven Zimmerman
