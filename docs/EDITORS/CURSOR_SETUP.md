# Cursor IDE Setup Guide for perl-lsp

Cursor is a VS Code-derived editor, so `perl-lsp` works with the same LSP wiring model: launch `perllsp --stdio` and keep the project root as the workspace folder.

## Prerequisites

- Cursor installed
- `perllsp` installed and available on `PATH`
- a Perl project opened as a folder/workspace in Cursor

Verify the server before configuring the editor:

```bash
perllsp --version
perllsp --health
```

## Installation Paths

Use one of these:

- install `perllsp` from crates.io: `cargo install perllsp`
- download a release binary from GitHub Releases
- build from source in this repository

If Cursor cannot find `perllsp`, set an absolute path in your LSP client settings or launch Cursor from a terminal where `perllsp --version` already works.

## Recommended Setup (Official perl-lsp Extension)

Cursor can install compatible VS Code extensions. The easiest setup is the official extension:

- Extension ID: `EffortlessMetrics.perl-lsp-rs`
- Then open a `.pl`, `.pm`, or `.t` file and confirm LSP features start automatically.

Optional workspace settings (`.vscode/settings.json` in your project):

```json
{
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.enableRefactoring": true
}
```

## Manual Fallback (Generic LSP Client)

If you prefer a generic LSP client in Cursor, configure the command exactly as:

```json
{
  "command": ["perllsp", "--stdio"],
  "languages": ["perl"]
}
```

Use your extension's schema for the exact setting keys; the important part is the process command and Perl file association.

## Quick Validation Checklist

1. Open a Perl file in Cursor.
2. Trigger hover on `print` (or another built-in) and confirm docs/signature appear.
3. Place cursor on a symbol and run **Go to Definition**.
4. Introduce a syntax error and confirm diagnostics appear.

If any step fails, follow [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md) and compare against the VS Code reference guide at [VS_CODE_SETUP.md](VS_CODE_SETUP.md).
