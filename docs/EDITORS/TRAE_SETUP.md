# Trae Setup Guide for perl-lsp

Trae (ByteDance) uses a VS Code-compatible extension and settings model, so the
`perl-lsp` integration is the same core setup: launch `perllsp --stdio` for Perl
files.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- Trae installed
- a project folder opened as the workspace root

Verify the server from a shell first:

```bash
perllsp --version
perllsp --health
```

## Option 1: Install the Perl extension (best UX)

If Trae can install VS Code-compatible extensions in your environment, install
`EffortlessMetrics.perl-lsp-rs` and keep the default settings. This enables
automatic server management and extension commands.

## Option 2: Configure a generic language server client

If extension marketplace access is unavailable, configure an LSP client entry
that launches:

```json
{
  "command": ["perllsp", "--stdio"],
  "languages": ["perl"]
}
```

Use Trae's LSP/client settings UI and map the client to Perl files (`.pl`,
`.pm`, `.t`, `.psgi`).

## Recommended workspace settings

In Trae workspace settings, keep Perl configured similarly to VS Code:

```json
{
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false
}
```

## Troubleshooting

- If the server is not found, open Trae's integrated terminal and run
  `perllsp --version` to confirm `PATH` is inherited.
- If diagnostics/completion do not appear, restart the language server from
  Trae's command palette and check its LSP output logs.
- If Perl files are detected as plain text, associate `*.pl`, `*.pm`, and
  `*.t` with Perl in Trae file associations.

For deeper triage, continue with [Troubleshooting](../how-to/TROUBLESHOOTING.md).
