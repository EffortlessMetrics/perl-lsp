# Windsurf Setup Guide for perl-lsp

This guide covers the quickest ways to run `perllsp` in Windsurf.

## Prerequisites

- `perllsp` installed and available on `PATH`
- Windsurf installed
- a workspace folder opened at your project root

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Option 1: Use the VS Code Extension Path (Recommended)

Windsurf is VS Code-compatible, so the simplest route is to use the same
workflow as VS Code:

1. Open the Extensions panel.
2. Install the `perl-lsp` extension if it is available in your extension
   catalog.
3. Open a Perl file (`.pl`, `.pm`, `.t`) and confirm language features start.

If your Windsurf build cannot install the extension from your catalog, use
Option 2.

## Option 2: Configure a Generic LSP Client

Configure the Perl language server command to:

```json
["perllsp", "--stdio"]
```

And scope it to Perl filetypes (`perl`, `.pl`, `.pm`, `.t`).

## Recommended Workspace Settings

Add these to your workspace settings JSON (or the equivalent Windsurf
settings UI):

```json
{
  "perl.perlPath": "perl",
  "perl.workspace.includePaths": ["lib", "local/lib/perl5"]
}
```

## Troubleshooting

- If the server does not start, run `perllsp --version` in a terminal started
  from Windsurf to confirm `PATH` visibility.
- If diagnostics/completion do not appear, verify the opened folder is the
  project root (not a parent directory with no Perl sources).
- If a specific extension setting does not appear in Windsurf, configure the
  server via the generic command approach in Option 2.
