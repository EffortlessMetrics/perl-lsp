# JetBrains Setup Guide for perl-lsp

This guide shows how to run `perllsp` from JetBrains IDEs using the
[LSP4IJ plugin](https://plugins.jetbrains.com/plugin/23257-lsp4ij).

## Prerequisites

- A JetBrains IDE with the LSP4IJ plugin installed
- `perllsp` installed and available on `PATH`

Verify the server before configuring the IDE:

```bash
perllsp --version
perllsp --health
```

## Configure LSP4IJ

1. Open **Settings** → **Languages & Frameworks** → **Language Servers**.
2. Add a new server.
3. Set:
   - **Name**: `perl-lsp`
   - **Command**: `perllsp`
   - **Arguments**: `--stdio`
   - **File types / patterns**: `*.pl`, `*.pm`, `*.t`, `*.psgi`
   - **Working directory**: your project root
4. Save and restart the language server from the Language Servers panel.

## Recommended project settings

- Mark your Perl source roots (for example `lib/` and `t/`) as project content
  roots so workspace indexing has stable roots.
- Keep one server instance per project root.

## Troubleshooting

- If JetBrains shows "server not found", use an absolute path for `perllsp`.
- If diagnostics do not appear, open the LSP4IJ logs and confirm the initialize
  request includes your project root.
- If external file changes are not reflected immediately, restart the language
  server from the Language Servers panel.

For common server-side issues, see
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
