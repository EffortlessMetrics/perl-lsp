# Perl Language Server

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/effortlesssteven.perl-lsp?label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)
[![Visual Studio Marketplace Downloads](https://img.shields.io/visual-studio-marketplace/d/effortlesssteven.perl-lsp)](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp)

Fast, modern Perl language support for Visual Studio Code, powered by `perl-lsp`.

## Features

- Go to Definition, Find References, Hover, Rename
- Completions, Signature Help, Symbols, CodeLens
- Semantic highlighting, inlay hints, call/type hierarchy
- Diagnostics for syntax and semantic issues
- Refactorings and code actions (extract variable/subroutine, loop transforms, import cleanup)
- Built-in debugger integration (`type: "perl"`)
- Formatting support with `perltidy`

## Installation

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=effortlesssteven.perl-lsp).

The extension can automatically download a matching `perl-lsp` server binary for your platform.

Manual alternatives:

```bash
# Homebrew (macOS/Linux)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# From source
cargo install --git https://github.com/EffortlessMetrics/perl-lsp --bin perl-lsp
```

## Configuration

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.serverPath": "",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"],
  "perl-lsp.trace.server": "off"
}
```

## Debugging

This extension contributes a Perl debugger (`type: "perl"`) for `launch` and `attach` workflows.

Minimal `launch.json` example:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Perl: Launch Script",
      "program": "${workspaceFolder}/script.pl"
    }
  ]
}
```

## Requirements

- Perl 5
- Optional: `perltidy` for formatting

## Resources

- [Project README](https://github.com/EffortlessMetrics/perl-lsp#readme)
- [Issue Tracker](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose)
- [Changelog](https://github.com/EffortlessMetrics/perl-lsp/blob/master/CHANGELOG.md)
- [Migration Guide](https://github.com/EffortlessMetrics/perl-lsp/blob/master/MIGRATION.md)

## License

MIT © Steven Zimmerman
